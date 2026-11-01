const LONGEST_TICK_MS: u64 = 250;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct LiveClock {
    last_ms: Option<u64>,
    live_ms: u64,
}

impl LiveClock {
    pub fn read(&mut self, now_ms: u64) -> u64 {
        let seen = self.live_ms;
        self.live_ms = self.at(now_ms);
        self.last_ms = Some(now_ms);
        seen
    }

    pub fn at(&self, now_ms: u64) -> u64 {
        self.live_ms
            + self
                .last_ms
                .map_or(0, |last| now_ms.saturating_sub(last).min(LONGEST_TICK_MS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stall_counts_as_one_tick_and_each_tick_reads_the_time_before_it() {
        let mut clock = LiveClock::default();
        let seen: Vec<u64> = [0, 100, 200, 3_200, 3_300, 3_400]
            .iter()
            .map(|at| clock.read(*at))
            .collect();
        assert_eq!(seen, [0, 0, 100, 200, 450, 550]);
        assert_eq!(
            clock.at(3_450),
            700,
            "between ticks the clock runs on from the last one"
        );
    }
}
