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
    pub imu_temp: f64,
    pub coordinate: Option<(f64, f64, f64)>,
    altitude_message_seen: bool,
    global_position_seen: bool,
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

fn quaternion_product([a1, b1, c1, d1]: [f64; 4], [a2, b2, c2, d2]: [f64; 4]) -> [f64; 4] {
    [
        a1 * a2 - b1 * b2 - c1 * c2 - d1 * d2,
        a1 * b2 + b1 * a2 + c1 * d2 - d1 * c2,
        a1 * c2 - b1 * d2 + c1 * a2 + d1 * b2,
        a1 * d2 + b1 * c2 - c1 * b2 + d1 * a2,
    ]
}

fn rotated(q: [f64; 4], [x, y, z]: [f64; 3]) -> [f64; 3] {
    let [_, x, y, z] = quaternion_product(quaternion_product(q, [0.0, x, y, z]), [q[0], -q[1], -q[2], -q[3]]);
    [x, y, z]
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

pub fn imu_temperature(centidegrees: i16) -> f64 {
    match centidegrees {
        0 => 0.0,
        raw => f64::from(raw) * 0.01,
    }
}

pub fn time_to_home(distance_to_home: Option<f64>, ground_speed: f64) -> Option<f64> {
    distance_to_home.filter(|d| !d.is_nan()).map(|d| d / ground_speed)
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
                let q = [d.q1 as f64, d.q2 as f64, d.q3 as f64, d.q4 as f64];
                let rates = [d.rollspeed as f64, d.pitchspeed as f64, d.yawspeed as f64];
                let offset = d.repr_offset_q.map(f64::from);
                let (q, [roll_rate, pitch_rate, yaw_rate]) = match offset.iter().map(|v| v * v).sum::<f64>().sqrt() >= 0.5 {
                    true => (quaternion_product(q, offset), rotated(offset, rates)),
                    false => (q, rates),
                };
                let (roll, pitch, yaw) = quaternion_to_euler(q);
                self.set_attitude(roll, pitch, yaw);
                self.roll_rate = roll_rate.to_degrees();
                self.pitch_rate = pitch_rate.to_degrees();
                self.yaw_rate = yaw_rate.to_degrees();
                true
            }
            MavMessage::ALTITUDE(d) => {
                self.altitude_message_seen = true;
                self.altitude_relative = d.altitude_relative as f64;
                self.altitude_amsl = d.altitude_amsl as f64;
                true
            }
            MavMessage::GLOBAL_POSITION_INT(d) if from == self.vehicle => {
                if !self.altitude_message_seen {
                    self.altitude_relative = d.relative_alt as f64 / 1000.0;
                    self.altitude_amsl = d.alt as f64 / 1000.0;
                }
                if d.lat != 0 || d.lon != 0 {
                    self.global_position_seen = true;
                    self.coordinate = Some((d.lat as f64 / 1e7, d.lon as f64 / 1e7, d.alt as f64 / 1000.0));
                }
                true
            }
            MavMessage::GPS_RAW_INT(d) if from == self.vehicle => {
                if (d.fix_type as u8) >= 3 && !self.global_position_seen {
                    self.coordinate = Some((d.lat as f64 / 1e7, d.lon as f64 / 1e7, d.alt as f64 / 1000.0));
                    if !self.altitude_message_seen {
                        self.altitude_amsl = d.alt as f64 / 1000.0;
                    }
                }
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
            MavMessage::RAW_IMU(d) => {
                self.imu_temp = imu_temperature(d.temperature);
                true
            }
            MavMessage::RANGEFINDER(d) => {
                self.range_finder_dist = zero_if_nan(d.distance);
                true
            }
            MavMessage::HIGH_LATENCY2(d) => {
                self.coordinate = Some((d.latitude as f64 / 1e7, d.longitude as f64 / 1e7, f64::from(d.altitude)));
                self.air_speed = f64::from(d.airspeed) / 5.0;
                self.ground_speed = f64::from(d.groundspeed) / 5.0;
                self.climb_rate = f64::from(d.climb_rate) / 10.0;
                self.heading = f64::from(d.heading) * 2.0;
                self.altitude_relative = f64::NAN;
                self.altitude_amsl = f64::from(d.altitude);
                true
            }
            MavMessage::HIGH_LATENCY(d) => {
                self.coordinate = Some((d.latitude as f64 / 1e7, d.longitude as f64 / 1e7, f64::from(d.altitude_amsl)));
                self.altitude_relative = f64::NAN;
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

    #[test]
    fn imu_temperature_and_time_to_home_follow_vehicle_fact_group() {
        assert_eq!(imu_temperature(4512), 45.12);
        assert_eq!(imu_temperature(0), 0.0, "zero means not reported and stays zero");
        assert_eq!(time_to_home(Some(100.0), 5.0), Some(20.0));
        assert_eq!(time_to_home(None, 5.0), None);
        assert!(time_to_home(Some(100.0), 0.0).is_some_and(f64::is_infinite), "QGC divides by a zero ground speed too");
    }
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
    fn altitude_and_position_fall_back_through_the_messages_in_the_vehicle_order() {
        use mavlink::dialects::ardupilotmega::{ALTITUDE_DATA, GLOBAL_POSITION_INT_DATA, GPS_RAW_INT_DATA, GpsFixType};
        let mut facts = VehicleFacts::for_vehicle(1, 1);
        let mut raw = GPS_RAW_INT_DATA::default();
        (raw.lat, raw.lon, raw.alt, raw.fix_type) = (473_000_000, 85_000_000, 500_000, GpsFixType::GPS_FIX_TYPE_3D_FIX);
        facts.apply((1, 1), &MavMessage::GPS_RAW_INT(raw.clone()));
        assert_eq!((facts.coordinate, facts.altitude_amsl), (Some((47.3, 8.5, 500.0)), 500.0));
        let mut elsewhere = raw.clone();
        elsewhere.lat = 0;
        facts.apply((1, 191), &MavMessage::GPS_RAW_INT(elsewhere.clone()));
        assert_eq!(facts.coordinate, Some((47.3, 8.5, 500.0)), "Vehicle::_handleGpsRawInt ignores a companion's position");
        let mut global = GLOBAL_POSITION_INT_DATA::default();
        (global.lat, global.lon, global.alt, global.relative_alt) = (474_000_000, 86_000_000, 520_000, 20_000);
        facts.apply((1, 191), &MavMessage::GLOBAL_POSITION_INT(global.clone()));
        assert_eq!(facts.coordinate, Some((47.3, 8.5, 500.0)), "and _handleGlobalPositionInt too");
        facts.apply((1, 1), &MavMessage::GLOBAL_POSITION_INT(global.clone()));
        assert_eq!((facts.coordinate, facts.altitude_relative, facts.altitude_amsl), (Some((47.4, 8.6, 520.0)), 20.0, 520.0));
        facts.apply((1, 1), &MavMessage::GPS_RAW_INT(raw));
        assert_eq!(facts.coordinate, Some((47.4, 8.6, 520.0)), "global position wins over raw gps once seen");
        let mut bogus = GLOBAL_POSITION_INT_DATA::default();
        bogus.relative_alt = 30_000;
        facts.apply((1, 1), &MavMessage::GLOBAL_POSITION_INT(bogus));
        assert_eq!((facts.coordinate, facts.altitude_relative), (Some((47.4, 8.6, 520.0)), 30.0), "a 0,0 position still carries altitude");
        let mut altitude = ALTITUDE_DATA::default();
        (altitude.altitude_relative, altitude.altitude_amsl) = (12.5, 512.5);
        facts.apply((1, 1), &MavMessage::ALTITUDE(altitude));
        facts.apply((1, 1), &MavMessage::GLOBAL_POSITION_INT(global));
        assert_eq!((facts.altitude_relative, facts.altitude_amsl), (12.5, 512.5), "the ALTITUDE message takes precedence");
    }

    #[test]
    fn a_representation_offset_rotates_attitude_and_rates_as_vehicle_fact_group_does() {
        use mavlink::dialects::ardupilotmega::ATTITUDE_QUATERNION_DATA;
        let mut facts = VehicleFacts::for_vehicle(1, 1);
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let mut tailsitter = ATTITUDE_QUATERNION_DATA::default();
        (tailsitter.q1, tailsitter.q2, tailsitter.q3, tailsitter.q4) = (1.0, 0.0, 0.0, 0.0);
        tailsitter.rollspeed = 1.0;
        tailsitter.repr_offset_q = [half, 0.0, half, 0.0];
        facts.apply((1, 1), &MavMessage::ATTITUDE_QUATERNION(tailsitter.clone()));
        assert!((facts.pitch - 90.0).abs() < 0.1, "pitched by the offset: {}", facts.pitch);
        assert!(facts.roll_rate.abs() < 1e-3 && (facts.yaw_rate.abs() - 1f64.to_degrees()).abs() < 1e-3, "rates turned with it: {} {}", facts.roll_rate, facts.yaw_rate);
        tailsitter.repr_offset_q = [0.0; 4];
        facts.apply((1, 1), &MavMessage::ATTITUDE_QUATERNION(tailsitter));
        assert!(facts.pitch.abs() < 1e-6 && (facts.roll_rate - 1f64.to_degrees()).abs() < 1e-3, "a zero offset is ignored");
    }

    #[test]
    fn the_sample_log_keeps_the_heading_on_the_compass() {
        let bytes = crate::samplelog::bytes();
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

    #[test]
    fn an_angle_that_is_not_a_number_leaves_by_the_arm_that_cannot_loop() {
        [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -1e300].iter().for_each(|angle| {
            assert!(
                super::limit_angle_to_pm_pi(*angle).is_nan(),
                "the -20pi..20pi guard is the only thing keeping NaN and the infinities out of the two (0..) searches below it, which have no upper bound and never match: {angle} must leave by the modulo arm. Measured: widening that guard makes this spin for 27s and then panic in core::iter::range with 'attempt to add with overflow', because (0..) is i32 and the predicate never matches"
            );
        });

        let (pi, eps) = (std::f32::consts::PI as f64, f32::EPSILON as f64);
        [0.0, pi, -pi, 3.0 * pi, -3.0 * pi, 19.0 * pi, -19.0 * pi].iter().for_each(|angle| {
            let wrapped = super::limit_angle_to_pm_pi(*angle);
            assert!(wrapped <= pi + eps && wrapped > -(pi + eps), "{angle} wrapped to {wrapped}, outside the range the searches are written to reach");
        });
    }
}
