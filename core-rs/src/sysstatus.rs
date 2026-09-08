use std::collections::BTreeMap;

const NAMES: &[(u32, &str)] = &[
    (1 << 0, "Gyro"), (1 << 1, "Accelerometer"), (1 << 2, "Magnetometer"), (1 << 3, "Barometer"), (1 << 4, "Airspeed Sensor"),
    (1 << 5, "GPS"), (1 << 6, "Optical Flow"), (1 << 7, "Vision Positioning"), (1 << 8, "Rangefinder"), (1 << 9, "External Positioning"),
    (1 << 10, "Rate Control"), (1 << 11, "Attitude Control"), (1 << 12, "Heading Control"), (1 << 13, "Altitude Control"), (1 << 14, "Position Control"),
    (1 << 15, "Motor Outputs"), (1 << 16, "RC Receiver"), (1 << 17, "Gyro 2"), (1 << 18, "Accelerometer 2"), (1 << 19, "Magnetometer 2"),
    (1 << 20, "Geofence"), (1 << 21, "Attitude Estimation"), (1 << 22, "Terrain"), (1 << 23, "Motors Reversed"), (1 << 24, "Logging"),
    (1 << 25, "Battery"), (1 << 26, "Proximity"), (1 << 27, "Satellite Link"), (1 << 28, "Pre-Arm Check"), (1 << 29, "Collision Avoidance"),
    (1 << 30, "Propulsion"),
];

pub fn sensor_name(bit: u32) -> String {
    NAMES.iter().find(|(b, _)| *b == bit).map(|(_, n)| n.to_string()).unwrap_or_else(|| format!("Unknown sensor 0x{bit:X}"))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorInfo {
    pub enabled: bool,
    pub healthy: bool,
}

#[derive(Debug, Default, PartialEq)]
pub struct SysStatusSensors {
    sensors: BTreeMap<u32, SensorInfo>,
}

pub struct Ordered {
    pub names: Vec<String>,
    pub status: Vec<&'static str>,
    pub healthy: Vec<bool>,
    pub enabled: Vec<bool>,
}

impl SysStatusSensors {
    pub fn update(&mut self, present: u32, enabled: u32, health: u32) -> bool {
        let next: BTreeMap<u32, SensorInfo> = (0..32)
            .map(|bit| 1u32 << bit)
            .filter(|mask| present & mask != 0)
            .map(|mask| (mask, SensorInfo { enabled: enabled & mask != 0, healthy: health & mask != 0 }))
            .collect();
        let changed = next != self.sensors;
        self.sensors = next;
        changed
    }

    fn ordered_pairs(&self) -> Vec<(u32, SensorInfo)> {
        let bucket = |info: &SensorInfo| match (info.enabled, info.healthy) {
            (false, _) => 2,
            (true, true) => 1,
            (true, false) => 0,
        };
        let mut listed: Vec<(u32, SensorInfo)> = self.sensors.iter().map(|(k, v)| (*k, *v)).collect();
        listed.sort_by_key(|(mask, info)| (bucket(info), *mask));
        listed
    }

    pub fn ordered(&self) -> Ordered {
        let pairs = self.ordered_pairs();
        Ordered {
            names: pairs.iter().map(|(mask, _)| sensor_name(*mask)).collect(),
            status: pairs.iter().map(|(_, i)| if !i.enabled { "Disabled" } else if i.healthy { "Normal" } else { "Error" }).collect(),
            healthy: pairs.iter().map(|(_, i)| i.enabled && i.healthy).collect(),
            enabled: pairs.iter().map(|(_, i)| i.enabled).collect(),
        }
    }

    pub fn unhealthy_bits(&self) -> u32 {
        self.sensors.iter().filter(|(_, i)| i.enabled && !i.healthy).fold(0, |acc, (mask, _)| acc | mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GYRO: u32 = 1;
    const GPS: u32 = 1 << 5;
    const GEOFENCE: u32 = 1 << 20;

    #[test]
    fn matches_sys_status_sensor_info_test() {
        let mut sensors = SysStatusSensors::default();
        assert!(sensors.update(GPS | GYRO | GEOFENCE, GPS | GYRO, GYRO));
        let ordered = sensors.ordered();
        assert_eq!(ordered.names, vec!["GPS", "Gyro", "Geofence"]);
        assert_eq!(ordered.healthy, vec![false, true, false]);
        assert_eq!(ordered.enabled, vec![true, true, false]);
        assert_eq!(ordered.status, vec!["Error", "Normal", "Disabled"]);
        assert_eq!(sensors.unhealthy_bits(), GPS);
        assert!(!sensors.update(GPS | GYRO | GEOFENCE, GPS | GYRO, GYRO));
        assert!(sensors.update(GYRO, GYRO, GYRO));
        assert_eq!(sensors.ordered().names, vec!["Gyro"]);
    }

    #[test]
    fn unknown_bits_still_get_a_name() {
        assert_eq!(sensor_name(1 << 31), "Unknown sensor 0x80000000");
        assert_eq!(sensor_name(1 << 21), "Attitude Estimation");
    }
}
