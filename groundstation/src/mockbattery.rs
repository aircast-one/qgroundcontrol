use mavlink::dialects::ardupilotmega::*;

const PACK_OHMS: f64 = 0.02;
const CELL_CURVE: [(f64, f64); 6] = [
    (0.0, 3.3),
    (0.1, 3.55),
    (0.2, 3.65),
    (0.5, 3.8),
    (0.8, 4.0),
    (1.0, 4.2),
];
const MAX_CELLS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Battery {
    pub consumed_mah: f64,
    pub current_a: f64,
}

impl Battery {
    pub fn drained(self, current_a: f64, dt: f64) -> Battery {
        Battery {
            consumed_mah: self.consumed_mah + current_a * dt / 3.6,
            current_a,
        }
    }

    pub fn left_mah(&self, capacity_mah: f64) -> f64 {
        (capacity_mah - self.consumed_mah).max(0.0)
    }

    pub fn fraction(&self, capacity_mah: f64) -> f64 {
        self.left_mah(capacity_mah) / capacity_mah.max(1.0)
    }

    pub fn percent(&self, capacity_mah: f64) -> i8 {
        (self.fraction(capacity_mah) * 100.0).round() as i8
    }

    pub fn cell_volts(&self, capacity_mah: f64, cells: f64) -> f64 {
        let fraction = self.fraction(capacity_mah);
        let resting = CELL_CURVE
            .windows(2)
            .find(|pair| fraction <= pair[1].0)
            .map_or(CELL_CURVE[CELL_CURVE.len() - 1].1, |pair| {
                pair[0].1
                    + (pair[1].1 - pair[0].1) * (fraction - pair[0].0) / (pair[1].0 - pair[0].0)
            });
        resting - self.current_a * PACK_OHMS / cells.max(1.0)
    }

    pub fn volts(&self, capacity_mah: f64, cells: f64) -> f64 {
        self.cell_volts(capacity_mah, cells) * cells
    }

    pub fn status(
        &self,
        id: u8,
        capacity_mah: f64,
        cells: f64,
        hover_a: f64,
    ) -> BATTERY_STATUS_DATA {
        let percent = self.percent(capacity_mah);
        let cell_mv = (self.cell_volts(capacity_mah, cells) * 1000.0) as u16;
        let drawing = if self.current_a > 0.0 {
            self.current_a
        } else {
            hover_a
        };
        BATTERY_STATUS_DATA {
            id,
            battery_function: MavBatteryFunction::MAV_BATTERY_FUNCTION_ALL,
            mavtype: MavBatteryType::MAV_BATTERY_TYPE_LIPO,
            temperature: if id == 1 { 2100 } else { i16::MAX },
            voltages: std::array::from_fn(|cell| {
                if cell < (cells as usize).min(MAX_CELLS) {
                    cell_mv
                } else {
                    u16::MAX
                }
            }),
            current_battery: (self.current_a * 100.0) as i16,
            current_consumed: self.consumed_mah as i32,
            energy_consumed: -1,
            battery_remaining: percent,
            time_remaining: (self.left_mah(capacity_mah) / 1000.0 / drawing.max(0.1) * 3600.0)
                as i32,
            charge_state: charge_state(percent),
            ..Default::default()
        }
    }
}

pub fn charge_state(percent: i8) -> MavBatteryChargeState {
    match percent {
        p if p > 50 => MavBatteryChargeState::MAV_BATTERY_CHARGE_STATE_OK,
        p if p > 30 => MavBatteryChargeState::MAV_BATTERY_CHARGE_STATE_LOW,
        p if p > 20 => MavBatteryChargeState::MAV_BATTERY_CHARGE_STATE_CRITICAL,
        _ => MavBatteryChargeState::MAV_BATTERY_CHARGE_STATE_EMERGENCY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_drains_by_the_amp_hour_and_sags_under_load() {
        let half = Battery::default().drained(10.0, 900.0);
        assert!(
            (half.consumed_mah - 2500.0).abs() < 1e-6,
            "10 A for 15 minutes is 2.5 Ah"
        );
        assert_eq!(half.percent(5000.0), 50);
        assert!((half.cell_volts(5000.0, 4.0) - (3.8 - 10.0 * PACK_OHMS / 4.0)).abs() < 1e-9);
        assert!(
            Battery::default().volts(5000.0, 4.0) > 16.7
                && Battery {
                    consumed_mah: 5000.0,
                    current_a: 0.0
                }
                .volts(5000.0, 4.0)
                    < 13.3,
            "a full 4S rests at 16.8 V and an empty one at 13.2 V"
        );
        let status = half.status(1, 5000.0, 4.0, 10.0);
        assert_eq!(
            (
                status.voltages[3],
                status.voltages[4],
                status.time_remaining
            ),
            (3750, u16::MAX, 900),
            "four cells reported, and 2.5 Ah left at 10 A is 15 minutes"
        );
    }
}
