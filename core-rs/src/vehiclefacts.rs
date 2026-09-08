#![allow(deprecated)]
use mavlink::dialects::ardupilotmega::MavMessage;
use std::f64::consts::{FRAC_PI_2, PI};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VehicleFacts {
    pub roll: f64,
    pub pitch: f64,
    pub heading: f64,
    pub roll_rate: f64,
    pub pitch_rate: f64,
    pub yaw_rate: f64,
    pub altitude_relative: f64,
    pub altitude_amsl: f64,
    pub air_speed: f64,
    pub ground_speed: f64,
    pub climb_rate: f64,
    pub throttle_pct: i16,
    pub altitude_tuning: f64,
    pub altitude_tuning_setpoint: f64,
    pub air_speed_setpoint: f64,
    pub x_track_error: f64,
    pub distance_to_next_wp: f64,
    pub range_finder_dist: f64,
    altitude_tuning_offset: Option<f64>,
    receiving_quaternion: bool,
    vehicle: (u8, u8),
}

pub fn limit_angle_to_pm_pi(angle: f64) -> f64 {
    let (pi, eps) = (std::f32::consts::PI as f64, f32::EPSILON as f64);
    match angle {
        a if a > -20.0 * PI && a < 20.0 * PI => {
            let down = (0..).map(|n| a - n as f64 * 2.0 * pi).find(|v| *v <= pi + eps).unwrap();
            (0..).map(|n| down + n as f64 * 2.0 * pi).find(|v| *v > -(pi + eps)).unwrap()
        }
        a => (a as f32 % std::f32::consts::PI) as f64,
    }
}

pub fn quaternion_to_euler([a, b, c, d]: [f64; 4]) -> (f64, f64, f64) {
    let (a2, b2, c2, d2) = (a * a, b * b, c * c, d * d);
    let dcm = [
        [(a2 + b2 - c2 - d2) as f32, (2.0 * (b * c - a * d)) as f32, (2.0 * (a * c + b * d)) as f32],
        [(2.0 * (b * c + a * d)) as f32, (a2 - b2 + c2 - d2) as f32, (2.0 * (c * d - a * b)) as f32],
        [(2.0 * (b * d - a * c)) as f32, (2.0 * (a * b + c * d)) as f32, (a2 - b2 - c2 + d2) as f32],
    ];
    let theta = (-dcm[2][0]).asin();
    let gimbal_lock = (theta.abs() - FRAC_PI_2 as f32).abs() < 1.0e-3;
    let (phi, psi) = if gimbal_lock {
        (0.0f32, (dcm[1][2] - dcm[0][1]).atan2(dcm[0][2] + dcm[1][1]))
    } else {
        (dcm[2][1].atan2(dcm[2][2]), dcm[1][0].atan2(dcm[0][0]))
    };
    (phi as f64, theta as f64, psi as f64)
}

fn attitude_degrees(roll: f64, pitch: f64, yaw: f64) -> (f64, f64, f64) {
    let yaw_degrees = limit_angle_to_pm_pi(yaw).to_degrees();
    let heading = if yaw_degrees < 0.0 { yaw_degrees + 360.0 } else { yaw_degrees };
    (limit_angle_to_pm_pi(roll).to_degrees(), limit_angle_to_pm_pi(pitch).to_degrees(), heading.trunc())
}

fn zero_if_nan(value: f32) -> f64 {
    if value.is_nan() { 0.0 } else { value as f64 }
}

impl VehicleFacts {
    pub fn for_vehicle(system_id: u8, component_id: u8) -> Self {
        VehicleFacts { vehicle: (system_id, component_id), ..Default::default() }
    }

    fn set_attitude(&mut self, roll: f64, pitch: f64, yaw: f64) {
        (self.roll, self.pitch, self.heading) = attitude_degrees(roll, pitch, yaw);
    }

    pub fn apply(&mut self, from: (u8, u8), message: &MavMessage) -> bool {
        match message {
            MavMessage::ATTITUDE(d) => {
                if from == self.vehicle && !self.receiving_quaternion {
                    self.set_attitude(d.roll as f64, d.pitch as f64, d.yaw as f64);
                }
                true
            }
            MavMessage::ATTITUDE_QUATERNION(d) if from == self.vehicle => {
                self.receiving_quaternion = true;
                let (roll, pitch, yaw) = quaternion_to_euler([d.q1 as f64, d.q2 as f64, d.q3 as f64, d.q4 as f64]);
                self.set_attitude(roll, pitch, yaw);
                self.roll_rate = (d.rollspeed as f64).to_degrees();
                self.pitch_rate = (d.pitchspeed as f64).to_degrees();
                self.yaw_rate = (d.yawspeed as f64).to_degrees();
                true
            }
            MavMessage::ALTITUDE(d) => {
                self.altitude_relative = d.altitude_relative as f64;
                self.altitude_amsl = d.altitude_amsl as f64;
                true
            }
            MavMessage::VFR_HUD(d) => {
                self.air_speed = zero_if_nan(d.airspeed);
                self.ground_speed = zero_if_nan(d.groundspeed);
                self.climb_rate = zero_if_nan(d.climb);
                self.throttle_pct = d.throttle as i16;
                let offset = self.altitude_tuning_offset.filter(|o| !o.is_nan()).unwrap_or(d.alt as f64);
                self.altitude_tuning_offset = Some(offset);
                self.altitude_tuning = d.alt as f64 - offset;
                true
            }
            MavMessage::NAV_CONTROLLER_OUTPUT(d) => {
                self.altitude_tuning_setpoint = self.altitude_tuning - d.alt_error as f64;
                self.x_track_error = d.xtrack_error as f64;
                self.air_speed_setpoint = self.air_speed - d.aspd_error as f64;
                self.distance_to_next_wp = d.wp_dist as f64;
                true
            }
            MavMessage::RANGEFINDER(d) => {
                self.range_finder_dist = zero_if_nan(d.distance);
                true
            }
            MavMessage::HIGH_LATENCY(d) => {
                self.altitude_amsl = d.altitude_amsl as f64;
                self.air_speed = d.airspeed as f64;
                self.ground_speed = d.groundspeed as f64;
                self.climb_rate = d.climb_rate as f64;
                self.throttle_pct = d.throttle as i16;
                self.heading = (d.heading as f64 / 100.0).trunc();
                self.roll = d.roll as f64 / 100.0;
                self.pitch = d.pitch as f64 / 100.0;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::{ATTITUDE_DATA, ATTITUDE_QUATERNION_DATA, NAV_CONTROLLER_OUTPUT_DATA, VFR_HUD_DATA};

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn attitude_becomes_degrees_with_a_truncated_compass_heading() {
        let mut facts = VehicleFacts::for_vehicle(1, 1);
        let mut data = ATTITUDE_DATA::default();
        data.roll = 0.5;
        data.pitch = -0.25;
        data.yaw = -1.5;
        facts.apply((1, 1), &MavMessage::ATTITUDE(data.clone()));
        assert!(near(facts.roll, 28.6479) && near(facts.pitch, -14.3239));
        assert_eq!(facts.heading, 274.0);
        data.yaw = 7.0;
        facts.apply((1, 1), &MavMessage::ATTITUDE(data));
        assert_eq!(facts.heading, 41.0);
    }

    #[test]
    fn a_quaternion_takes_over_from_plain_attitude() {
        let mut facts = VehicleFacts::for_vehicle(1, 1);
        let half = 0.5f32;
        let mut quat = ATTITUDE_QUATERNION_DATA::default();
        (quat.q1, quat.q4, quat.yawspeed) = (half.cos(), half.sin(), 0.1);
        facts.apply((1, 1), &MavMessage::ATTITUDE_QUATERNION(quat.clone()));
        assert_eq!((facts.roll, facts.pitch, facts.heading), (0.0, 0.0, 57.0));
        let mut other = VehicleFacts::for_vehicle(1, 1);
        other.apply((1, 154), &MavMessage::ATTITUDE_QUATERNION(quat));
        assert_eq!(other.heading, 0.0);
        assert!(near(facts.yaw_rate, 5.7296));
        let mut plain = ATTITUDE_DATA::default();
        plain.yaw = 1.0;
        facts.apply((1, 1), &MavMessage::ATTITUDE(plain));
        assert_eq!(facts.heading, 57.0);
    }

    #[test]
    fn tuning_offsets_are_taken_from_the_first_hud() {
        let mut facts = VehicleFacts::for_vehicle(1, 1);
        let mut hud = VFR_HUD_DATA::default();
        (hud.alt, hud.airspeed, hud.groundspeed, hud.throttle) = (f32::NAN, f32::NAN, 4.5, 55);
        facts.apply((1, 1), &MavMessage::VFR_HUD(hud.clone()));
        hud.alt = 100.0;
        facts.apply((1, 1), &MavMessage::VFR_HUD(hud.clone()));
        hud.alt = 112.5;
        facts.apply((1, 1), &MavMessage::VFR_HUD(hud));
        assert_eq!((facts.altitude_tuning, facts.air_speed, facts.ground_speed, facts.throttle_pct), (12.5, 0.0, 4.5, 55));
        let mut nav = NAV_CONTROLLER_OUTPUT_DATA::default();
        (nav.alt_error, nav.wp_dist) = (2.5, 40);
        facts.apply((1, 1), &MavMessage::NAV_CONTROLLER_OUTPUT(nav));
        assert_eq!((facts.altitude_tuning_setpoint, facts.distance_to_next_wp), (10.0, 40.0));
        assert!(!facts.apply((1, 1), &MavMessage::HEARTBEAT(Default::default())));
    }

    #[test]
    fn the_sample_log_keeps_the_heading_on_the_compass() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
        let mut facts = VehicleFacts::for_vehicle(1, 1);
        let mut headings = Vec::new();
        crate::tlog::for_each(&bytes, |_, header, message| {
            if facts.apply((header.system_id, header.component_id), message) {
                headings.push(facts.heading);
            }
        });
        assert!(!headings.is_empty(), "no vehicle fact messages in the sample log");
        assert!(headings.iter().all(|h| (0.0..360.0).contains(h) && h.fract() == 0.0));
    }
}
