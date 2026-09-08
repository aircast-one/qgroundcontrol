#![allow(deprecated)]
use mavlink::dialects::ardupilotmega::{EstimatorStatusFlags, MavMessage, MavSensorOrientation};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct WindFacts {
    pub direction: f64,
    pub speed: f64,
    pub vertical_speed: f64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TemperatureFacts {
    pub temperature1: f64,
    pub temperature2: f64,
    pub temperature3: f64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DistanceSensorFacts {
    pub by_orientation: BTreeMap<u32, f64>,
    pub min_distance: f64,
    pub max_distance: f64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LocalPositionFacts {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct EstimatorStatusFacts {
    pub good_attitude: bool,
    pub good_horiz_vel: bool,
    pub good_vert_vel: bool,
    pub good_horiz_pos_rel: bool,
    pub good_horiz_pos_abs: bool,
    pub good_vert_pos_abs: bool,
    pub good_vert_pos_agl: bool,
    pub good_const_pos_mode: bool,
    pub good_pred_horiz_pos_rel: bool,
    pub good_pred_horiz_pos_abs: bool,
    pub gps_glitch: bool,
    pub accel_error: bool,
    pub vel_ratio: f64,
    pub horiz_pos_ratio: f64,
    pub vert_pos_ratio: f64,
    pub mag_ratio: f64,
    pub hagl_ratio: f64,
    pub tas_ratio: f64,
    pub horiz_pos_accuracy: f64,
    pub vert_pos_accuracy: f64,
}

pub const DISTANCE_ORIENTATIONS: [MavSensorOrientation; 10] = [
    MavSensorOrientation::MAV_SENSOR_ROTATION_NONE,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_45,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_90,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_135,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_180,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_225,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_270,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_315,
    MavSensorOrientation::MAV_SENSOR_ROTATION_PITCH_90,
    MavSensorOrientation::MAV_SENSOR_ROTATION_PITCH_270,
];

fn compass(degrees: f32) -> f64 {
    (if degrees < 0.0 { degrees + 360.0 } else { degrees }) as f64
}

impl WindFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        match message {
            MavMessage::WIND_COV(d) => {
                self.direction = compass(d.wind_y.atan2(d.wind_x).to_degrees());
                self.speed = d.wind_x.hypot(d.wind_y) as f64;
                self.vertical_speed = d.wind_z as f64;
                true
            }
            MavMessage::HIGH_LATENCY(d) => {
                self.speed = d.airspeed as f64 / 5.0;
                true
            }
            MavMessage::HIGH_LATENCY2(d) => {
                self.direction = d.wind_heading as f64 * 2.0;
                self.speed = d.windspeed as f64 / 5.0;
                true
            }
            MavMessage::WIND(d) => {
                self.direction = compass(d.direction);
                self.speed = d.speed as f64;
                self.vertical_speed = d.speed_z as f64;
                true
            }
            _ => false,
        }
    }
}

impl TemperatureFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        match message {
            MavMessage::SCALED_PRESSURE(d) => {
                self.temperature1 = d.temperature as f64 / 100.0;
                true
            }
            MavMessage::SCALED_PRESSURE2(d) => {
                self.temperature2 = d.temperature as f64 / 100.0;
                true
            }
            MavMessage::SCALED_PRESSURE3(d) => {
                self.temperature3 = d.temperature as f64 / 100.0;
                true
            }
            MavMessage::HIGH_LATENCY(d) => {
                self.temperature1 = d.temperature_air as f64;
                true
            }
            MavMessage::HIGH_LATENCY2(d) => {
                self.temperature1 = d.temperature_air as f64;
                true
            }
            _ => false,
        }
    }
}

impl DistanceSensorFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let MavMessage::DISTANCE_SENSOR(d) = message else { return false };
        if DISTANCE_ORIENTATIONS.contains(&d.orientation) {
            self.by_orientation.insert(d.orientation as u32, d.current_distance as f64 / 100.0);
        }
        self.min_distance = d.min_distance as f64 / 100.0;
        self.max_distance = d.max_distance as f64 / 100.0;
        true
    }
}

impl LocalPositionFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let MavMessage::LOCAL_POSITION_NED(d) = message else { return false };
        *self = LocalPositionFacts { x: d.x as f64, y: d.y as f64, z: d.z as f64, vx: d.vx as f64, vy: d.vy as f64, vz: d.vz as f64 };
        true
    }
}

impl EstimatorStatusFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let MavMessage::ESTIMATOR_STATUS(d) = message else { return false };
        let has = |flag: EstimatorStatusFlags| d.flags.contains(flag);
        *self = EstimatorStatusFacts {
            good_attitude: has(EstimatorStatusFlags::ESTIMATOR_ATTITUDE),
            good_horiz_vel: has(EstimatorStatusFlags::ESTIMATOR_VELOCITY_HORIZ),
            good_vert_vel: has(EstimatorStatusFlags::ESTIMATOR_VELOCITY_VERT),
            good_horiz_pos_rel: has(EstimatorStatusFlags::ESTIMATOR_POS_HORIZ_REL),
            good_horiz_pos_abs: has(EstimatorStatusFlags::ESTIMATOR_POS_HORIZ_ABS),
            good_vert_pos_abs: has(EstimatorStatusFlags::ESTIMATOR_POS_VERT_ABS),
            good_vert_pos_agl: has(EstimatorStatusFlags::ESTIMATOR_POS_VERT_AGL),
            good_const_pos_mode: has(EstimatorStatusFlags::ESTIMATOR_CONST_POS_MODE),
            good_pred_horiz_pos_rel: has(EstimatorStatusFlags::ESTIMATOR_PRED_POS_HORIZ_REL),
            good_pred_horiz_pos_abs: has(EstimatorStatusFlags::ESTIMATOR_PRED_POS_HORIZ_ABS),
            gps_glitch: has(EstimatorStatusFlags::ESTIMATOR_GPS_GLITCH),
            accel_error: has(EstimatorStatusFlags::ESTIMATOR_ACCEL_ERROR),
            vel_ratio: d.vel_ratio as f64,
            horiz_pos_ratio: d.pos_horiz_ratio as f64,
            vert_pos_ratio: d.pos_vert_ratio as f64,
            mag_ratio: d.mag_ratio as f64,
            hagl_ratio: d.hagl_ratio as f64,
            tas_ratio: d.tas_ratio as f64,
            horiz_pos_accuracy: d.pos_horiz_accuracy as f64,
            vert_pos_accuracy: d.pos_vert_accuracy as f64,
        };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::{DISTANCE_SENSOR_DATA, ESTIMATOR_STATUS_DATA, HIGH_LATENCY2_DATA, SCALED_PRESSURE2_DATA, WIND_COV_DATA, WIND_DATA};

    #[test]
    fn wind_direction_is_a_compass_bearing_from_the_vector() {
        let mut wind = WindFacts::default();
        let mut cov = WIND_COV_DATA::default();
        (cov.wind_x, cov.wind_y, cov.wind_z) = (0.0, -3.0, 0.5);
        wind.apply(&MavMessage::WIND_COV(cov));
        assert_eq!((wind.direction, wind.speed, wind.vertical_speed), (270.0, 3.0, 0.5));
        let mut plain = WIND_DATA::default();
        (plain.direction, plain.speed) = (-10.0, 2.0);
        wind.apply(&MavMessage::WIND(plain));
        assert_eq!((wind.direction, wind.speed), (350.0, 2.0));
        let mut hl = HIGH_LATENCY2_DATA::default();
        (hl.wind_heading, hl.windspeed, hl.temperature_air) = (90, 25, -7);
        wind.apply(&MavMessage::HIGH_LATENCY2(hl.clone()));
        assert_eq!((wind.direction, wind.speed), (180.0, 5.0));
        let mut temperature = TemperatureFacts::default();
        temperature.apply(&MavMessage::HIGH_LATENCY2(hl));
        let mut pressure = SCALED_PRESSURE2_DATA::default();
        pressure.temperature = 2350;
        temperature.apply(&MavMessage::SCALED_PRESSURE2(pressure));
        assert_eq!((temperature.temperature1, temperature.temperature2), (-7.0, 23.5));
    }

    #[test]
    fn distance_sensors_are_kept_per_known_orientation_in_metres() {
        let mut sensors = DistanceSensorFacts::default();
        let mut d = DISTANCE_SENSOR_DATA::default();
        (d.current_distance, d.min_distance, d.max_distance) = (250, 20, 4000);
        d.orientation = MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_90;
        sensors.apply(&MavMessage::DISTANCE_SENSOR(d.clone()));
        d.orientation = MavSensorOrientation::MAV_SENSOR_ROTATION_ROLL_90;
        d.current_distance = 999;
        sensors.apply(&MavMessage::DISTANCE_SENSOR(d));
        assert_eq!(sensors.by_orientation, BTreeMap::from([(2, 2.5)]));
        assert_eq!((sensors.min_distance, sensors.max_distance), (0.2, 40.0));
    }

    #[test]
    fn estimator_flags_split_into_booleans() {
        let mut estimator = EstimatorStatusFacts::default();
        let mut d = ESTIMATOR_STATUS_DATA::default();
        d.flags = EstimatorStatusFlags::ESTIMATOR_ATTITUDE | EstimatorStatusFlags::ESTIMATOR_GPS_GLITCH;
        d.mag_ratio = 0.4;
        estimator.apply(&MavMessage::ESTIMATOR_STATUS(d));
        assert!(estimator.good_attitude && estimator.gps_glitch && !estimator.good_horiz_vel && !estimator.accel_error);
        assert!((estimator.mag_ratio - 0.4).abs() < 1e-6);
        assert!(!estimator.apply(&MavMessage::HEARTBEAT(Default::default())));
    }

    #[test]
    fn the_sample_log_feeds_at_least_one_sensor_group() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
        let (mut wind, mut temperature, mut distance, mut local, mut estimator) =
            (WindFacts::default(), TemperatureFacts::default(), DistanceSensorFacts::default(), LocalPositionFacts::default(), EstimatorStatusFacts::default());
        let mut applied = [0usize; 5];
        crate::tlog::for_each(&bytes, |_, _, message| {
            let hits = [wind.apply(message), temperature.apply(message), distance.apply(message), local.apply(message), estimator.apply(message)];
            applied = std::array::from_fn(|i| applied[i] + hits[i] as usize);
        });
        assert!(applied.iter().any(|n| *n > 0), "no sensor fact messages in the sample log");
        assert!(applied[0] == 0 || (0.0..360.0).contains(&wind.direction));
        assert!(applied[1] == 0 || temperature.temperature1.abs() < 100.0);
    }
}
