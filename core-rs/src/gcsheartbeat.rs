use std::sync::{Mutex, PoisonError};

pub const PERIOD_MS: u64 = 1000;

static LAST_MS: Mutex<Option<u64>> = Mutex::new(None);

pub fn due(last: Option<u64>, now_ms: u64) -> bool {
    last.is_none_or(|last| now_ms.saturating_sub(last) >= PERIOD_MS)
}

fn wanted() -> bool {
    crate::settingsstore::raw_setting("settings.mavlinkSettings.sendGCSHeartbeat").and_then(|v| v.as_bool()).unwrap_or(true)
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
    let Some(bytes) = crate::mavout::encode_next(&crate::mavout::Outbound::GcsHeartbeat) else { return };
    let targets: Vec<crate::transport::LinkId> = {
        let transports = crate::linkhost::TRANSPORTS.lock().unwrap_or_else(PoisonError::into_inner);
        transports.open_ids().into_iter().filter(|id| transports.describe(*id).is_some_and(|(_, _, high_latency)| !high_latency)).collect()
    };
    targets.into_iter().for_each(|id| {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, id, &bytes);
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
    fn the_heartbeat_says_a_ground_station_is_active() {
        let bytes = crate::mavout::encode(0, &crate::mavout::Outbound::GcsHeartbeat).unwrap();
        let (_, message) = mavlink::read_v2_msg::<mavlink::dialects::ardupilotmega::MavMessage, _>(&mut mavlink::peek_reader::PeekReader::new(bytes.as_slice())).unwrap();
        let mavlink::dialects::ardupilotmega::MavMessage::HEARTBEAT(beat) = message else { panic!("not a heartbeat") };
        assert_eq!((beat.mavtype as u8, beat.autopilot as u8, beat.base_mode.bits(), beat.system_status as u8), (6, 8, 192, 4), "MAV_TYPE_GCS, MAV_AUTOPILOT_INVALID, MAV_MODE_MANUAL_ARMED, MAV_STATE_ACTIVE, as MultiVehicleManager packs it");
    }
}
