use mavlink::dialects::ardupilotmega::MavMessage;

pub const REPORT_EVERY_MS: u64 = 10_000;
const NANOS_PER_MS: i64 = 1_000_000;
const LONGEST_ROUND_TRIP_MS: u64 = 10_000;

pub fn median(values: &[u64]) -> Option<u64> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.get(sorted.len() / 2).copied()
}

pub fn shown(value: Option<u64>) -> String {
    value.map_or_else(|| "none".to_string(), |ms| ms.to_string())
}

pub fn ping_stamp(now_ms: u64) -> i64 {
    i64::try_from(now_ms).unwrap_or(i64::MAX / NANOS_PER_MS) * NANOS_PER_MS
}

pub fn round_trip(ts1: i64, now_ms: u64) -> Option<u64> {
    u64::try_from(ts1 / NANOS_PER_MS).ok().and_then(|sent_ms| now_ms.checked_sub(sent_ms)).filter(|ms| *ms < LONGEST_ROUND_TRIP_MS)
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Stream {
    count: u64,
    last_ms: Option<u64>,
    gap_max_ms: u64,
}

impl Stream {
    fn noted(&self, now_ms: u64) -> Self {
        Self { count: self.count + 1, last_ms: Some(now_ms), gap_max_ms: self.last_ms.map_or(self.gap_max_ms, |last| self.gap_max_ms.max(now_ms.saturating_sub(last))) }
    }

    fn hz(&self, window_ms: u64) -> f64 {
        self.count as f64 * 1000.0 / window_ms.max(1) as f64
    }

    fn restarted(&self) -> Self {
        Self { count: 0, last_ms: self.last_ms, gap_max_ms: 0 }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LinkHealth {
    since_ms: Option<u64>,
    attitude: Stream,
    position: Stream,
    round_trips: Vec<u64>,
}

impl LinkHealth {
    pub fn observe(&mut self, message: &MavMessage, from_autopilot: bool, now_ms: u64) {
        if !from_autopilot {
            return;
        }
        match message {
            MavMessage::ATTITUDE(_) | MavMessage::ATTITUDE_QUATERNION(_) => self.attitude = self.attitude.noted(now_ms),
            MavMessage::GLOBAL_POSITION_INT(_) => self.position = self.position.noted(now_ms),
            MavMessage::TIMESYNC(sync) if sync.tc1 != 0 && crate::mavout::for_us(sync.target_system) => self.round_trips.extend(round_trip(sync.ts1, now_ms)),
            _ => {}
        }
    }

    pub fn report(&mut self, now_ms: u64, vehicle: u8, link: impl std::fmt::Display) -> Option<String> {
        let since_ms = *self.since_ms.get_or_insert(now_ms);
        let window_ms = now_ms.saturating_sub(since_ms);
        (window_ms >= REPORT_EVERY_MS).then(|| {
            let line = format!(
                "event=link_health vehicle={vehicle} link={link} seconds={} rtt_p50_ms={} rtt_max_ms={} pings_answered={} attitude_hz={:.1} attitude_gap_max_ms={} position_hz={:.1} position_gap_max_ms={}",
                window_ms / 1000,
                shown(median(&self.round_trips)),
                shown(self.round_trips.iter().copied().max()),
                self.round_trips.len(),
                self.attitude.hz(window_ms),
                self.attitude.gap_max_ms,
                self.position.hz(window_ms),
                self.position.gap_max_ms,
            );
            *self = Self { since_ms: Some(now_ms), attitude: self.attitude.restarted(), position: self.position.restarted(), round_trips: Vec::new() };
            line
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::{ATTITUDE_DATA, GLOBAL_POSITION_INT_DATA, TIMESYNC_DATA};

    fn reply(ts1: i64, target_system: u8) -> MavMessage {
        MavMessage::TIMESYNC(TIMESYNC_DATA { tc1: 42, ts1, target_system, target_component: 0 })
    }

    #[test]
    fn the_median_is_the_middle_value_whatever_the_order() {
        assert_eq!(median(&[40, 10, 30, 20, 50]), Some(30));
        assert_eq!(median(&[]), None);
    }

    #[test]
    fn a_reply_echoes_our_send_time_so_the_round_trip_needs_no_bookkeeping() {
        assert_eq!(round_trip(ping_stamp(1_000), 1_072), Some(72));
        assert_eq!(round_trip(ping_stamp(2_000), 1_072), None, "a stamp from the future is someone else's clock");
        assert_eq!(round_trip(ping_stamp(1_000), 60_000), None, "nor is one a minute old");
    }

    #[test]
    fn a_window_reports_round_trips_rates_and_the_worst_gap_between_readings() {
        let mut health = LinkHealth::default();
        assert!(health.report(0, 1, 3).is_none(), "the first call opens the window");
        health.observe(&reply(ping_stamp(1_000), 255), true, 1_072);
        health.observe(&reply(ping_stamp(2_000), 0), true, 2_090);
        health.observe(&reply(ping_stamp(3_000), 7), true, 3_010);
        health.observe(&reply(ping_stamp(4_000), 255), false, 4_010);
        let attitude = MavMessage::ATTITUDE(ATTITUDE_DATA::default());
        (0..200u64).for_each(|n| health.observe(&attitude, true, n * 50 + if n == 100 { 300 } else { 0 }));
        (0..30u64).for_each(|n| health.observe(&MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA::default()), true, n * 333));
        health.observe(&attitude, false, 9_999);
        assert!(health.report(9_999, 1, 3).is_none());
        let report = health.report(10_000, 1, 3).expect("a report after ten seconds");
        assert_eq!(
            report,
            "event=link_health vehicle=1 link=3 seconds=10 rtt_p50_ms=90 rtt_max_ms=90 pings_answered=2 attitude_hz=20.0 attitude_gap_max_ms=350 position_hz=3.0 position_gap_max_ms=333",
            "replies meant for another station and readings from other components are not counted"
        );
        assert!(health.report(10_100, 1, 3).is_none(), "the next window starts fresh");
    }
}
