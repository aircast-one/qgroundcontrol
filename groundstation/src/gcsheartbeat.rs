use std::sync::{Mutex, PoisonError};

pub const PERIOD_MS: u64 = 1000;

static LAST_MS: Mutex<Option<u64>> = Mutex::new(None);

pub fn due(last: Option<u64>, now_ms: u64) -> bool {
    last.is_none_or(|last| now_ms.saturating_sub(last) >= PERIOD_MS)
}

pub fn wanted() -> bool {
    crate::settingsstore::raw_setting("settings.mavlinkSettings.sendGCSHeartbeat").and_then(|v| v.as_bool()).unwrap_or(true)
}

pub fn frames(now_ms: u64) -> Vec<Vec<u8>> {
    [crate::mavout::Outbound::GcsHeartbeat, crate::mavout::Outbound::Timesync { target: (0, 0), ts1: crate::linkhealth::ping_stamp(now_ms) }].iter().filter_map(crate::mavout::encode_next).collect()
}

pub fn tick(now_ms: u64) {
    {
        let mut last = LAST_MS.lock().unwrap_or_else(PoisonError::into_inner);
        if !due(*last, now_ms) {
            return;
        }
        *last = Some(now_ms);
    }
    if !wanted() {
        return;
    }
    let frames = frames(now_ms);
    let targets: Vec<crate::transport::LinkId> = {
        let transports = crate::linkhost::TRANSPORTS.lock().unwrap_or_else(PoisonError::into_inner);
        transports.open_ids().into_iter().filter(|id| transports.describe(*id).is_some_and(|(_, _, high_latency)| !high_latency)).collect()
    };
    targets.into_iter().for_each(|id| {
        frames.iter().for_each(|bytes| {
            crate::linkhost::write(&crate::linkhost::TRANSPORTS, id, bytes);
        });
        if frames.len() == 2 {
            crate::linkhealth::pinged(id);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heartbeat_goes_out_once_a_second() {
        assert!(due(None, 0), "the first tick sends at once");
        assert!(!due(Some(1_000), 1_999));
        assert!(due(Some(1_000), 2_000));
    }

    #[test]
    fn an_unset_station_id_is_255_and_a_frame_for_everyone_or_us_is_ours() {
        assert_eq!(crate::mavout::gcs_system(), 255);
        assert!(crate::mavout::for_us(0) && crate::mavout::for_us(255) && !crate::mavout::for_us(254));
    }

    #[test]
    fn each_beat_carries_a_timesync_ping_stamped_with_the_send_time() {
        use mavlink::dialects::ardupilotmega::MavMessage;
        let decoded: Vec<MavMessage> = frames(5_000).iter().map(|bytes| mavlink::read_v2_msg::<MavMessage, _>(&mut mavlink::peek_reader::PeekReader::new(bytes.as_slice())).unwrap().1).collect();
        assert!(matches!(decoded[0], MavMessage::HEARTBEAT(_)), "{decoded:?}");
        let MavMessage::TIMESYNC(ping) = &decoded[1] else { panic!("no ping: {decoded:?}") };
        assert_eq!((ping.tc1, ping.ts1, ping.target_system, ping.target_component), (0, crate::linkhealth::ping_stamp(5_000), 0, 0), "a request (tc1 0) to everyone, echoed back with our send time");
        assert_eq!(crate::linkhealth::round_trip_now(ping.ts1, 5_072), Some(72));
    }

    #[test]
    fn the_heartbeat_says_a_ground_station_is_active() {
        let bytes = crate::mavout::encode(0, &crate::mavout::Outbound::GcsHeartbeat).unwrap();
        let (_, message) = mavlink::read_v2_msg::<mavlink::dialects::ardupilotmega::MavMessage, _>(&mut mavlink::peek_reader::PeekReader::new(bytes.as_slice())).unwrap();
        let mavlink::dialects::ardupilotmega::MavMessage::HEARTBEAT(beat) = message else { panic!("not a heartbeat") };
        assert_eq!((beat.mavtype as u8, beat.autopilot as u8, beat.base_mode.bits(), beat.system_status as u8), (6, 8, 192, 4), "MAV_TYPE_GCS, MAV_AUTOPILOT_INVALID, MAV_MODE_MANUAL_ARMED, MAV_STATE_ACTIVE, as MultiVehicleManager packs it");
    }
}
