#![allow(deprecated)]
use mavlink::dialects::ardupilotmega::{EstimatorStatusFlags, MavMessage, MavSensorOrientation};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct WindFacts {
    pub direction: f64,
    pub speed: f64,
    pub vertical_speed: f64,
    pub seen: [bool; 3],
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct EscFacts {
    pub rpm: i32,
    pub current: f32,
    pub voltage: f32,
    pub count: u8,
    pub connection_type: u8,
    pub info: u8,
    pub failure_flags: u16,
    pub error_count: u32,
    pub temperature: i16,
    pub telemetry: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Escs {
    pub by_id: std::collections::BTreeMap<u32, EscFacts>,
}

impl Escs {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let first = match message {
            MavMessage::ESC_INFO(_) => 0,
            MavMessage::ESC_STATUS(d) => u32::from(d.index),
            _ => return false,
        };
        (first..=first + 3).for_each(|id| {
            self.by_id.entry(id).or_default();
        });
        self.by_id.iter_mut().for_each(|(id, esc)| match message {
            MavMessage::ESC_INFO(d) if (u32::from(d.index)..u32::from(d.index) + 4).contains(id) => {
                let slot = (*id % 4) as usize;
                *esc = EscFacts { count: d.count, connection_type: d.connection_type as u8, info: d.info, failure_flags: d.failure_flags[slot], error_count: d.error_count[slot], temperature: d.temperature[slot], telemetry: true, ..esc.clone() };
            }
            MavMessage::ESC_STATUS(d) if (u32::from(d.index)..u32::from(d.index) + 4).contains(id) => {
                let slot = (*id % 4) as usize;
                *esc = EscFacts { rpm: d.rpm[slot], current: d.current[slot], voltage: d.voltage[slot], telemetry: true, ..esc.clone() };
            }
            _ => {}
        });
        true
    }
}

pub const SUB_INFO_READINGS: [&str; 9] = ["cameraTilt", "tetherTurns", "lights1", "lights2", "pilotGain", "inputHold", "rollPitchToggle", "rangefinderDistance", "rangefinderTarget"];

const SUB_NAMED_VALUES: [(&str, &str, f32); 8] = [
    ("CamTilt", "cameraTilt", 100.0),
    ("TetherTrn", "tetherTurns", 1.0),
    ("Lights1", "lights1", 100.0),
    ("Lights2", "lights2", 100.0),
    ("PilotGain", "pilotGain", 100.0),
    ("InputHold", "inputHold", 1.0),
    ("RollPitch", "rollPitchToggle", 1.0),
    ("RFTarget", "rangefinderTarget", 1.0),
];

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SubInfoFacts {
    pub readings: [Option<f32>; 9],
    pub seen: bool,
}

impl SubInfoFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let update = match message {
            MavMessage::NAMED_VALUE_FLOAT(d) => {
                let name = d.name.to_str().unwrap_or("");
                SUB_NAMED_VALUES.iter().find(|(sent, _, _)| *sent == name).map(|(_, fact, scale)| (*fact, d.value * scale))
            }
            MavMessage::RANGEFINDER(d) => Some(("rangefinderDistance", d.distance)),
            _ => None,
        };
        let Some((fact, value)) = update else { return false };
        let at = SUB_INFO_READINGS.iter().position(|n| *n == fact).unwrap_or(0);
        self.readings[at] = Some(value);
        self.seen = true;
        true
    }

    pub fn reading(&self, name: &str) -> Option<f32> {
        SUB_INFO_READINGS.iter().position(|n| *n == name).and_then(|i| self.readings[i])
    }
}

pub const RPM_READINGS: [&str; 6] = ["rpm1", "rpm2", "rpm3", "rpm4", "rpmSensor1", "rpmSensor2"];

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RpmFacts {
    pub readings: [Option<f32>; 6],
    pub seen: bool,
}

impl RpmFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let updates: Vec<(usize, f32)> = match message {
            MavMessage::RAW_RPM(d) if d.index < 4 => vec![(usize::from(d.index), d.frequency)],
            MavMessage::RAW_RPM(_) => Vec::new(),
            MavMessage::RPM(d) => vec![(4, d.rpm1), (5, d.rpm2)],
            _ => return false,
        };
        updates.into_iter().for_each(|(at, value)| self.readings[at] = Some(value));
        self.seen = true;
        true
    }

    pub fn reading(&self, name: &str) -> Option<f32> {
        RPM_READINGS.iter().position(|n| *n == name).and_then(|i| self.readings[i])
    }
}

pub const EFI_READINGS: [&str; 18] = [
    "ecuIndex", "rpm", "fuelConsumed", "fuelFlow", "engineLoad", "throttlePos", "sparkTime", "baroPress", "intakePress", "intakeTemp", "cylinderTemp", "ignTime", "injTime", "exGasTemp", "throttleOut", "ptComp", "ignVoltage", "fuelPressure",
];

#[derive(Debug, Default, Clone, PartialEq)]
pub struct EfiFacts {
    pub health: u8,
    pub readings: [f32; 18],
    pub seen: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GeneratorFacts {
    pub status: u64,
    pub speed: u16,
    pub battery_current: f32,
    pub load_current: f32,
    pub power_generated: f32,
    pub bus_voltage: f32,
    pub battery_current_setpoint: f32,
    pub rectifier_temperature: i16,
    pub generator_temperature: i16,
    pub runtime: u32,
    pub time_until_maintenance: i32,
    pub status_changed: bool,
    pub seen: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct HygrometerFacts {
    pub temperature: f64,
    pub humidity: f64,
    pub id: u8,
    pub seen: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SetpointFacts {
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
    pub roll_rate: f64,
    pub pitch_rate: f64,
    pub yaw_rate: f64,
    pub seen: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TemperatureFacts {
    pub temperature1: f64,
    pub temperature2: f64,
    pub temperature3: f64,
    pub seen: [bool; 3],
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DistanceSensorFacts {
    pub by_orientation: BTreeMap<u32, f64>,
    pub min_distance: f64,
    pub max_distance: f64,
    pub seen: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LocalPositionFacts {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub seen: bool,
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
    pub seen: bool,
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
                self.speed = ((d.wind_x as f64).powi(2) + (d.wind_y as f64).powi(2)).sqrt() as f32 as f64;
                self.vertical_speed = d.wind_z as f64;
                self.seen = [true; 3];
                true
            }
            MavMessage::HIGH_LATENCY(d) => {
                self.speed = d.airspeed as f64 / 5.0;
                self.seen[1] = true;
                true
            }
            MavMessage::HIGH_LATENCY2(d) => {
                self.direction = d.wind_heading as f64 * 2.0;
                self.speed = d.windspeed as f64 / 5.0;
                self.seen[0] = true;
                self.seen[1] = true;
                true
            }
            MavMessage::WIND(d) => {
                self.direction = compass(d.direction);
                self.speed = d.speed as f64;
                self.vertical_speed = d.speed_z as f64;
                self.seen = [true; 3];
                true
            }
            _ => false,
        }
    }
}

impl EfiFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let MavMessage::EFI_STATUS(d) = message else { return false };
        *self = EfiFacts {
            health: if d.health == 127 { 0 } else { d.health },
            readings: [
                d.ecu_index, d.rpm, d.fuel_consumed, d.fuel_flow, d.engine_load, d.throttle_position, d.spark_dwell_time, d.barometric_pressure, d.intake_manifold_pressure, d.intake_manifold_temperature,
                d.cylinder_head_temperature, d.ignition_timing, d.injection_time, d.exhaust_gas_temperature, d.throttle_out, d.pt_compensation, d.ignition_voltage, d.fuel_pressure,
            ],
            seen: true,
        };
        true
    }

    pub fn reading(&self, name: &str) -> Option<f32> {
        EFI_READINGS.iter().position(|n| *n == name).map(|i| self.readings[i])
    }
}

impl GeneratorFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let MavMessage::GENERATOR_STATUS(d) = message else { return false };
        let status = if d.status.bits() == u64::from(u16::MAX) { 0 } else { d.status.bits() };
        *self = GeneratorFacts {
            status,
            speed: if d.generator_speed == u16::MAX { 0 } else { d.generator_speed },
            battery_current: d.battery_current,
            load_current: d.load_current,
            power_generated: d.power_generated,
            bus_voltage: d.bus_voltage,
            battery_current_setpoint: d.bat_current_setpoint,
            rectifier_temperature: if d.rectifier_temperature == i16::MAX { 0 } else { d.rectifier_temperature },
            generator_temperature: if d.generator_temperature == i16::MAX { 0 } else { d.generator_temperature },
            runtime: if d.runtime == u32::MAX { 0 } else { d.runtime },
            time_until_maintenance: if d.time_until_maintenance == i32::MAX { 0 } else { d.time_until_maintenance },
            status_changed: self.status_changed || status != self.status,
            seen: true,
        };
        true
    }
}

impl HygrometerFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let MavMessage::HYGROMETER_SENSOR(d) = message else { return false };
        *self = HygrometerFacts { temperature: f64::from(f32::from(d.temperature) / 100.0), humidity: f64::from(d.humidity), id: d.id, seen: true };
        true
    }
}

impl SetpointFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let MavMessage::ATTITUDE_TARGET(d) = message else { return false };
        let (roll, pitch, yaw) = crate::vehiclefacts::quaternion_to_euler(d.q.map(|q| q as f64));
        let yaw = if (yaw as f32) < 0.0 { yaw as f32 + 2.0 * std::f32::consts::PI } else { yaw as f32 };
        let degrees = |radians: f32| radians.to_degrees() as f64;
        *self = SetpointFacts {
            roll: degrees(roll as f32),
            pitch: degrees(pitch as f32),
            yaw: degrees(yaw),
            roll_rate: degrees(d.body_roll_rate),
            pitch_rate: degrees(d.body_pitch_rate),
            yaw_rate: degrees(d.body_yaw_rate),
            seen: true,
        };
        true
    }
}

impl TemperatureFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        match message {
            MavMessage::SCALED_PRESSURE(d) => {
                self.temperature1 = d.temperature as f64 / 100.0;
                self.seen[0] = true;
                true
            }
            MavMessage::SCALED_PRESSURE2(d) => {
                self.temperature2 = d.temperature as f64 / 100.0;
                self.seen[1] = true;
                true
            }
            MavMessage::SCALED_PRESSURE3(d) => {
                self.temperature3 = d.temperature as f64 / 100.0;
                self.seen[2] = true;
                true
            }
            MavMessage::HIGH_LATENCY(d) => {
                self.temperature1 = d.temperature_air as f64;
                self.seen[0] = true;
                true
            }
            MavMessage::HIGH_LATENCY2(d) => {
                self.temperature1 = d.temperature_air as f64;
                self.seen[0] = true;
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
        self.seen = true;
        true
    }
}

impl LocalPositionFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        let MavMessage::LOCAL_POSITION_NED(d) = message else { return false };
        *self = LocalPositionFacts { x: d.x as f64, y: d.y as f64, z: d.z as f64, vx: d.vx as f64, vy: d.vy as f64, vz: d.vz as f64, seen: true };
        true
    }

    pub fn apply_target(&mut self, message: &MavMessage) -> bool {
        let MavMessage::POSITION_TARGET_LOCAL_NED(d) = message else { return false };
        *self = LocalPositionFacts { x: d.x as f64, y: d.y as f64, z: d.z as f64, vx: d.vx as f64, vy: d.vy as f64, vz: d.vz as f64, seen: true };
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
            seen: true,
        };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::{DISTANCE_SENSOR_DATA, ESTIMATOR_STATUS_DATA, HIGH_LATENCY2_DATA, SCALED_PRESSURE2_DATA, WIND_COV_DATA, WIND_DATA};

    #[test]
    fn a_position_target_fills_the_setpoint_not_the_position() {
        let mut position = LocalPositionFacts::default();
        let target = mavlink::dialects::ardupilotmega::POSITION_TARGET_LOCAL_NED_DATA { x: 1.5, vz: -0.25, ..Default::default() };
        assert!(!position.apply(&MavMessage::POSITION_TARGET_LOCAL_NED(target.clone())));
        assert!(position.apply_target(&MavMessage::POSITION_TARGET_LOCAL_NED(target)));
        assert_eq!((position.x, position.vz, position.seen), (1.5, -0.25, true));
    }

    #[test]
    fn esc_info_always_opens_the_first_four_because_qt_reads_its_index_from_the_wrong_message() {
        use mavlink::dialects::ardupilotmega::{ESC_INFO_DATA, ESC_STATUS_DATA};
        let mut escs = Escs::default();
        escs.apply(&MavMessage::ESC_STATUS(ESC_STATUS_DATA { index: 4, rpm: [100, 200, 300, 400], ..Default::default() }));
        assert_eq!(escs.by_id.keys().copied().collect::<Vec<_>>(), vec![4, 5, 6, 7]);
        assert_eq!((escs.by_id[&5].rpm, escs.by_id[&5].telemetry), (200, true));
        escs.apply(&MavMessage::ESC_INFO(ESC_INFO_DATA { index: 4, count: 8, temperature: [0, 3150, 0, 0], ..Default::default() }));
        assert_eq!(escs.by_id.keys().copied().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!((escs.by_id[&5].temperature, escs.by_id[&5].count, escs.by_id[&5].rpm, escs.by_id[&0].telemetry), (3150, 8, 200, false));
    }

    #[test]
    fn sub_info_follows_ardusub_named_values() {
        let named = |name: &str, value: f32| MavMessage::NAMED_VALUE_FLOAT(mavlink::dialects::ardupilotmega::NAMED_VALUE_FLOAT_DATA { time_boot_ms: 0, value, name: name.into() });
        let mut sub = SubInfoFacts::default();
        sub.apply(&named("CamTilt", 0.25));
        sub.apply(&named("TetherTrn", 3.0));
        sub.apply(&named("Other", 9.0));
        sub.apply(&MavMessage::RANGEFINDER(mavlink::dialects::ardupilotmega::RANGEFINDER_DATA { distance: 1.5, voltage: 0.0 }));
        assert_eq!((sub.reading("cameraTilt"), sub.reading("tetherTurns"), sub.reading("rangefinderDistance"), sub.reading("lights1")), (Some(25.0), Some(3.0), Some(1.5), None), "ArduSubFirmwarePlugin::_handleNamedValueFloat scales tilt, lights and gain to percent");
    }

    #[test]
    fn rpm_readings_follow_vehicle_rpm_fact_group() {
        let mut rpm = RpmFacts::default();
        rpm.apply(&MavMessage::RAW_RPM(mavlink::dialects::ardupilotmega::RAW_RPM_DATA { frequency: 1200.0, index: 2 }));
        rpm.apply(&MavMessage::RAW_RPM(mavlink::dialects::ardupilotmega::RAW_RPM_DATA { frequency: 9.0, index: 7 }));
        rpm.apply(&MavMessage::RPM(mavlink::dialects::ardupilotmega::RPM_DATA { rpm1: 3000.0, rpm2: 3100.0 }));
        assert_eq!((rpm.reading("rpm3"), rpm.reading("rpm1"), rpm.reading("rpmSensor1"), rpm.reading("rpmSensor2")), (Some(1200.0), None, Some(3000.0), Some(3100.0)), "RAW_RPM fills rpm1-4 by index, ignoring others; ArduPilot RPM fills both sensors");
        assert!(rpm.seen);
    }

    #[test]
    fn efi_readings_are_named_in_message_order() {
        let mut efi = EfiFacts::default();
        let status = mavlink::dialects::ardupilotmega::EFI_STATUS_DATA { health: 127, rpm: 5200.0, fuel_pressure: 310.5, ..Default::default() };
        efi.apply(&MavMessage::EFI_STATUS(status));
        assert_eq!((efi.health, efi.reading("rpm"), efi.reading("fuelPressure"), efi.reading("health")), (0, Some(5200.0), Some(310.5), None));
    }

    #[test]
    fn generator_sentinels_read_as_zero_and_a_status_change_is_remembered() {
        use mavlink::dialects::ardupilotmega::{GENERATOR_STATUS_DATA, MavGeneratorStatusFlag};
        let mut generator = GeneratorFacts::default();
        let unknown = GENERATOR_STATUS_DATA { status: MavGeneratorStatusFlag::from_bits_retain(u64::from(u16::MAX)), generator_speed: u16::MAX, runtime: u32::MAX, rectifier_temperature: i16::MAX, bus_voltage: 48.5, ..Default::default() };
        generator.apply(&MavMessage::GENERATOR_STATUS(unknown));
        assert_eq!((generator.status, generator.speed, generator.runtime, generator.rectifier_temperature, generator.bus_voltage, generator.status_changed), (0, 0, 0, 0, 48.5, false));
        generator.apply(&MavMessage::GENERATOR_STATUS(GENERATOR_STATUS_DATA { status: MavGeneratorStatusFlag::from_bits_retain(4), ..Default::default() }));
        generator.apply(&MavMessage::GENERATOR_STATUS(GENERATOR_STATUS_DATA { status: MavGeneratorStatusFlag::empty(), ..Default::default() }));
        assert_eq!((generator.status, generator.status_changed), (0, true));
    }

    #[test]
    fn hygrometer_temperature_is_centidegrees_in_single_precision() {
        let mut hygrometer = HygrometerFacts::default();
        let sensor = mavlink::dialects::ardupilotmega::HYGROMETER_SENSOR_DATA { temperature: 2137, humidity: 4550, id: 2 };
        hygrometer.apply(&MavMessage::HYGROMETER_SENSOR(sensor));
        assert_eq!((hygrometer.temperature, hygrometer.humidity, hygrometer.id, hygrometer.seen), (f64::from(21.37f32), 4550.0, 2, true));
    }

    #[test]
    fn setpoint_yaw_is_brought_into_a_heading_range() {
        let mut setpoint = SetpointFacts::default();
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let target = mavlink::dialects::ardupilotmega::ATTITUDE_TARGET_DATA { q: [half, 0.0, 0.0, -half], body_yaw_rate: std::f32::consts::PI, ..Default::default() };
        setpoint.apply(&MavMessage::ATTITUDE_TARGET(target));
        assert!(setpoint.seen);
        assert!((setpoint.yaw - 270.0).abs() < 1e-3, "{}", setpoint.yaw);
        assert_eq!(setpoint.yaw_rate, 180.0);
    }

    #[test]
    fn high_latency_wind_marks_only_the_facts_it_carries() {
        let mut wind = WindFacts::default();
        wind.apply(&MavMessage::HIGH_LATENCY(Default::default()));
        assert_eq!(wind.seen, [false, true, false]);
        wind.apply(&MavMessage::HIGH_LATENCY2(Default::default()));
        assert_eq!(wind.seen, [true, true, false]);
    }

    #[test]
    fn wind_direction_is_a_compass_bearing_from_the_vector() {
        let mut wind = WindFacts::default();
        let mut cov = WIND_COV_DATA::default();
        (cov.wind_x, cov.wind_y, cov.wind_z) = (0.0, -3.0, 0.5);
        wind.apply(&MavMessage::WIND_COV(cov));
        assert_eq!((wind.direction, wind.speed, wind.vertical_speed, wind.seen), (270.0, 3.0, 0.5, [true; 3]));
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
        let bytes = crate::samplelog::bytes();
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
