use mavlink::dialects::ardupilotmega::MavMessage;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BatteryFacts {
    pub function: u32,
    pub kind: u32,
    pub temperature: Option<f64>,
    pub voltage: Option<f64>,
    pub current: Option<f64>,
    pub mah_consumed: Option<f64>,
    pub percent_remaining: Option<f64>,
    pub instant_power: Option<f64>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Batteries {
    pub by_id: BTreeMap<u8, BatteryFacts>,
}

fn total_voltage(cells: &[u16]) -> Option<f64> {
    cells
        .iter()
        .take_while(|cell| **cell != u16::MAX)
        .map(|cell| *cell as f64 / 1000.0)
        .reduce(|sum, cell| sum + cell)
}

fn unless<T: PartialEq>(raw: T, sentinel: T, scale: impl Fn(T) -> f64) -> Option<f64> {
    (raw != sentinel).then(|| scale(raw))
}

impl Batteries {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        match message {
            MavMessage::HIGH_LATENCY2(d) => {
                self.by_id.entry(0).or_default().percent_remaining = unless(d.battery, -1, f64::from);
                true
            }
            MavMessage::BATTERY_STATUS(d) => {
                let voltage = total_voltage(&d.voltages);
                let current = unless(d.current_battery, -1, |raw| raw as f64 / 100.0);
                *self.by_id.entry(d.id).or_default() = BatteryFacts {
                    function: d.battery_function as u32,
                    kind: d.mavtype as u32,
                    temperature: unless(d.temperature, i16::MAX, |raw| raw as f64 / 100.0),
                    voltage,
                    current,
                    mah_consumed: unless(d.current_consumed, -1, f64::from),
                    percent_remaining: unless(d.battery_remaining, -1, f64::from),
                    instant_power: voltage.zip(current).map(|(v, a)| v * a),
                };
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::{BATTERY_STATUS_DATA, HIGH_LATENCY2_DATA, MavBatteryFunction};

    #[test]
    fn battery_status_sums_cells_and_scales_like_the_fact_group() {
        let mut batteries = Batteries::default();
        let mut data = BATTERY_STATUS_DATA::default();
        data.id = 1;
        data.voltages = [4200, 4100, 4000, u16::MAX, 5, 5, 5, 5, 5, 5];
        data.current_battery = 1250;
        data.current_consumed = 340;
        data.battery_remaining = 77;
        data.temperature = i16::MAX;
        data.battery_function = MavBatteryFunction::MAV_BATTERY_FUNCTION_PROPULSION;
        assert!(batteries.apply(&MavMessage::BATTERY_STATUS(data)));
        let facts = &batteries.by_id[&1];
        assert!((facts.voltage.unwrap() - 12.3).abs() < 1e-9);
        assert_eq!(facts.current, Some(12.5));
        assert!((facts.instant_power.unwrap() - 153.75).abs() < 1e-9);
        assert_eq!((facts.mah_consumed, facts.percent_remaining, facts.temperature), (Some(340.0), Some(77.0), None));
        assert_eq!(facts.function, 2);
        let mut unknown = BATTERY_STATUS_DATA::default();
        unknown.voltages = [u16::MAX; 10];
        unknown.current_battery = -1;
        batteries.apply(&MavMessage::BATTERY_STATUS(unknown));
        assert_eq!(batteries.by_id.keys().copied().collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!((batteries.by_id[&0].voltage, batteries.by_id[&0].instant_power), (None, None));
    }

    #[test]
    fn high_latency2_reports_only_percent_on_battery_zero() {
        let mut batteries = Batteries::default();
        let mut data = HIGH_LATENCY2_DATA::default();
        data.battery = -1;
        batteries.apply(&MavMessage::HIGH_LATENCY2(data.clone()));
        assert_eq!(batteries.by_id[&0].percent_remaining, None);
        data.battery = 42;
        batteries.apply(&MavMessage::HIGH_LATENCY2(data));
        assert_eq!(batteries.by_id[&0].percent_remaining, Some(42.0));
        assert!(!batteries.apply(&MavMessage::HEARTBEAT(Default::default())));
    }

    #[test]
    fn the_sample_log_reports_a_battery() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
        let mut batteries = Batteries::default();
        crate::tlog::for_each(&bytes, |_, _, message| {
            batteries.apply(message);
        });
        let voltages: Vec<f64> = batteries.by_id.values().filter_map(|b| b.voltage).collect();
        assert!(!voltages.is_empty(), "no battery voltage in the sample log");
        assert!(voltages.iter().all(|v| (0.0..100.0).contains(v)));
    }
}
