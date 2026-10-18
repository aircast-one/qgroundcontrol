use mavlink::dialects::ardupilotmega::MavMessage;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use crate::stats::{median, shown};
use crate::transport::LinkId;

pub const REPORT_EVERY_MS: u64 = 10_000;
const NANOS_PER_MS: i64 = 1_000_000;
const LONGEST_ROUND_TRIP_MS: u64 = 10_000;

static NONCE: LazyLock<i64> = LazyLock::new(|| {
    i64::from(
        web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos()),
    ) % NANOS_PER_MS
});
static PINGS_SENT: Mutex<BTreeMap<LinkId, u64>> = Mutex::new(BTreeMap::new());
static RECENT_ROUND_TRIP_MS: AtomicU64 = AtomicU64::new(0);

pub fn recent_round_trip_ms() -> Option<u64> {
    Some(RECENT_ROUND_TRIP_MS.load(Ordering::Relaxed)).filter(|ms| *ms > 0)
}

fn smoothed(previous: u64, latest: u64) -> u64 {
    match previous {
        0 => latest.max(1),
        _ => ((previous * 3 + latest) / 4).max(1),
    }
}

pub fn stamp(now_ms: u64, nonce: i64) -> i64 {
    i64::try_from(now_ms).unwrap_or(i64::MAX / NANOS_PER_MS - 1) * NANOS_PER_MS + nonce
}

pub fn answered(ts1: i64, now_ms: u64, nonce: i64) -> Option<u64> {
    (ts1.rem_euclid(NANOS_PER_MS) == nonce)
        .then(|| u64::try_from(ts1.div_euclid(NANOS_PER_MS)).ok())
        .flatten()
        .and_then(|sent_ms| now_ms.checked_sub(sent_ms))
        .filter(|ms| *ms < LONGEST_ROUND_TRIP_MS)
}

pub fn ping_stamp(now_ms: u64) -> i64 {
    stamp(now_ms, *NONCE)
}

pub fn round_trip_now(ts1: i64, now_ms: u64) -> Option<u64> {
    answered(ts1, now_ms, *NONCE)
}

pub fn pinged(link: LinkId) {
    *PINGS_SENT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(link)
        .or_insert(0) += 1;
}

pub fn pings_sent(link: LinkId) -> u64 {
    PINGS_SENT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&link)
        .copied()
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub id: LinkId,
    pub name: String,
    pub kind: String,
    pub radio: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Stream {
    count: u64,
    last_ms: Option<u64>,
    gap_max_ms: u64,
}

impl Stream {
    fn noted(&self, now_ms: u64) -> Self {
        Self {
            count: self.count + 1,
            last_ms: Some(now_ms),
            gap_max_ms: self.last_ms.map_or(self.gap_max_ms, |last| {
                self.gap_max_ms.max(now_ms.saturating_sub(last))
            }),
        }
    }

    fn hz(&self, window_ms: u64) -> f64 {
        self.count as f64 * 1000.0 / window_ms.max(1) as f64
    }

    fn restarted(&self) -> Self {
        Self {
            count: 0,
            last_ms: self.last_ms,
            gap_max_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Window {
    since_ms: u64,
    route: Route,
    pings_at_open: u64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LinkHealth {
    window: Option<Window>,
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
            MavMessage::ATTITUDE(_) | MavMessage::ATTITUDE_QUATERNION(_) => {
                self.attitude = self.attitude.noted(now_ms)
            }
            MavMessage::GLOBAL_POSITION_INT(_) => self.position = self.position.noted(now_ms),
            MavMessage::TIMESYNC(sync)
                if sync.tc1 != 0 && crate::mavout::for_us(sync.target_system) =>
            {
                let answered = round_trip_now(sync.ts1, now_ms);
                answered.iter().for_each(|ms| {
                    RECENT_ROUND_TRIP_MS.store(
                        smoothed(RECENT_ROUND_TRIP_MS.load(Ordering::Relaxed), *ms),
                        Ordering::Relaxed,
                    );
                });
                self.round_trips.extend(answered);
            }
            _ => {}
        }
    }

    pub fn report(
        &mut self,
        now_ms: u64,
        vehicle: u8,
        route: &Route,
        pings_sent: impl Fn(LinkId) -> u64,
    ) -> Option<String> {
        let open = self
            .window
            .get_or_insert_with(|| Window {
                since_ms: now_ms,
                route: route.clone(),
                pings_at_open: pings_sent(route.id),
            })
            .clone();
        let window_ms = now_ms.saturating_sub(open.since_ms);
        (window_ms >= REPORT_EVERY_MS || open.route.id != route.id).then(|| {
            let line = format!(
                "event=link_health vehicle={vehicle} link={} link_name={:?} link_kind={:?} radio={} seconds={} rtt_p50_ms={} rtt_max_ms={} pings_sent={} pings_answered={} attitude_hz={:.1} attitude_gap_max_ms={} position_hz={:.1} position_gap_max_ms={}",
                open.route.id,
                open.route.name,
                open.route.kind,
                open.route.radio,
                window_ms / 1000,
                shown(median(&self.round_trips)),
                shown(self.round_trips.iter().copied().max()),
                pings_sent(open.route.id).saturating_sub(open.pings_at_open),
                self.round_trips.len(),
                self.attitude.hz(window_ms),
                self.attitude.gap_max_ms,
                self.position.hz(window_ms),
                self.position.gap_max_ms,
            );
            *self = Self {
                window: Some(Window { since_ms: now_ms, route: route.clone(), pings_at_open: pings_sent(route.id) }),
                attitude: self.attitude.restarted(),
                position: self.position.restarted(),
                round_trips: Vec::new(),
            };
            line
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::{
        ATTITUDE_DATA, GLOBAL_POSITION_INT_DATA, TIMESYNC_DATA,
    };

    fn reply(ts1: i64, target_system: u8) -> MavMessage {
        MavMessage::TIMESYNC(TIMESYNC_DATA {
            tc1: 42,
            ts1,
            target_system,
            target_component: 0,
        })
    }

    fn route(id: LinkId) -> Route {
        Route {
            id,
            name: "TCP local".to_string(),
            kind: "TCP".to_string(),
            radio: false,
        }
    }

    #[test]
    fn the_recent_round_trip_follows_replies_smoothly() {
        assert_eq!(smoothed(0, 40), 40, "the first reply is taken as it is");
        assert_eq!(
            smoothed(40, 80),
            50,
            "later ones move it a quarter of the way"
        );
        assert_eq!(smoothed(0, 0), 1, "zero stays reserved for no reply yet");
    }

    #[test]
    fn a_reply_echoes_our_send_time_and_nonce_so_the_round_trip_needs_no_bookkeeping() {
        assert_eq!(answered(stamp(1_000, 417), 1_072, 417), Some(72));
        assert_eq!(
            answered(stamp(1_000, 418), 1_072, 417),
            None,
            "another station's ping, echoed back to everyone on a shared link"
        );
        assert_eq!(
            answered(stamp(2_000, 417), 1_072, 417),
            None,
            "a stamp from the future is someone else's clock"
        );
        assert_eq!(
            answered(stamp(1_000, 417), 60_000, 417),
            None,
            "nor is one a minute old"
        );
    }

    #[test]
    fn a_window_reports_round_trips_rates_and_the_worst_gap_between_readings() {
        let mut health = LinkHealth::default();
        let sent = |_: LinkId| 0;
        assert!(
            health.report(0, 1, &route(3), sent).is_none(),
            "the first call opens the window"
        );
        health.observe(&reply(ping_stamp(1_000), 255), true, 1_072);
        health.observe(&reply(ping_stamp(2_000), 0), true, 2_090);
        health.observe(&reply(ping_stamp(3_000), 7), true, 3_010);
        health.observe(&reply(ping_stamp(4_000), 255), false, 4_010);
        let attitude = MavMessage::ATTITUDE(ATTITUDE_DATA::default());
        (0..200u64)
            .for_each(|n| health.observe(&attitude, true, n * 50 + if n == 100 { 300 } else { 0 }));
        (0..30u64).for_each(|n| {
            health.observe(
                &MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA::default()),
                true,
                n * 333,
            )
        });
        health.observe(&attitude, false, 9_999);
        assert!(health.report(9_999, 1, &route(3), |_| 9).is_none());
        let report = health
            .report(10_000, 1, &route(3), |_| 10)
            .expect("a report after ten seconds");
        assert_eq!(
            report,
            "event=link_health vehicle=1 link=3 link_name=\"TCP local\" link_kind=\"TCP\" radio=false seconds=10 rtt_p50_ms=90 rtt_max_ms=90 pings_sent=10 pings_answered=2 attitude_hz=20.0 attitude_gap_max_ms=350 position_hz=3.0 position_gap_max_ms=333",
            "replies meant for another station and readings from other components are not counted"
        );
        assert!(
            health.report(10_100, 1, &route(3), |_| 10).is_none(),
            "the next window starts fresh"
        );
    }

    #[test]
    fn switching_links_closes_the_window_so_two_links_are_never_mixed() {
        let mut health = LinkHealth::default();
        assert!(health.report(0, 1, &route(3), |_| 0).is_none());
        let closed = health
            .report(4_000, 1, &route(5), |link| if link == 3 { 4 } else { 0 })
            .expect("a switch reports at once");
        assert!(
            closed.contains("link=3 ")
                && closed.contains("seconds=4 ")
                && closed.contains("pings_sent=4 "),
            "{closed}"
        );
        let next = health
            .report(14_000, 1, &route(5), |_| 10)
            .expect("the new link's window");
        assert!(
            next.contains("link=5 ") && next.contains("pings_sent=10 "),
            "{next}"
        );
    }
}
