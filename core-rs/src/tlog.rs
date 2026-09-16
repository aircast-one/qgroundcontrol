use std::collections::BTreeMap;

use mavlink::dialects::ardupilotmega::MavMessage;
use mavlink::peek_reader::PeekReader;
use mavlink::{MavlinkVersion, Message, ReadVersion, read_versioned_msg};
use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const TIMESTAMP_BYTES: usize = 8;

#[derive(Debug, Default, PartialEq)]
pub struct Summary {
    pub frames: usize,
    pub undecodable: usize,
    pub by_name: BTreeMap<String, usize>,
    pub first_timestamp_us: Option<u64>,
    pub last_timestamp_us: Option<u64>,
    pub system_ids: Vec<u8>,
    pub vehicle_system_ids: Vec<u8>,
    pub by_system: BTreeMap<u8, BTreeMap<String, usize>>,
}

pub fn frame_length(bytes: &[u8]) -> Option<(MavlinkVersion, usize)> {
    match (bytes.first()?, bytes.get(1)?, bytes.get(2)) {
        (0xFD, len, Some(flags)) => Some((MavlinkVersion::V2, 12 + *len as usize + if flags & 1 != 0 { 13 } else { 0 })),
        (0xFE, len, _) => Some((MavlinkVersion::V1, 8 + *len as usize)),
        _ => None,
    }
}

pub fn record(timestamp_us: u64, frame: &[u8]) -> Vec<u8> {
    timestamp_us.to_be_bytes().iter().copied().chain(frame.iter().copied()).collect()
}

pub fn parse_timestamp(raw: [u8; 8], now_us: u64) -> u64 {
    let big = u64::from_be_bytes(raw);
    if big > now_us { big.swap_bytes() } else { big }
}

pub fn entries(bytes: &[u8], now_us: u64) -> Vec<(u64, Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + TIMESTAMP_BYTES < bytes.len() {
        let frame_start = at + TIMESTAMP_BYTES;
        let Some((_, length)) = frame_length(&bytes[frame_start..]) else {
            at += 1;
            continue;
        };
        let Some(frame) = bytes.get(frame_start..frame_start + length) else { break };
        let (version, _) = frame_length(frame).unwrap();
        if read_versioned_msg::<MavMessage, _>(&mut PeekReader::new(frame), ReadVersion::Single(version)).is_err() {
            at += 1;
            continue;
        }
        out.push((parse_timestamp(bytes[at..frame_start].try_into().unwrap(), now_us), frame.to_vec()));
        at = frame_start + length;
    }
    out
}

pub fn for_each(bytes: &[u8], mut visit: impl FnMut(u64, &mavlink::MavHeader, &MavMessage)) -> usize {
    let mut undecodable = 0usize;
    let mut at = 0usize;
    while at + TIMESTAMP_BYTES < bytes.len() {
        let frame_start = at + TIMESTAMP_BYTES;
        let Some((version, length)) = frame_length(&bytes[frame_start..]) else {
            at += 1;
            continue;
        };
        let Some(frame) = bytes.get(frame_start..frame_start + length) else { break };
        let timestamp = u64::from_be_bytes(bytes[at..frame_start].try_into().unwrap());
        match read_versioned_msg::<MavMessage, _>(&mut PeekReader::new(frame), ReadVersion::Single(version)) {
            Ok((header, message)) => {
                visit(timestamp, &header, &message);
                at = frame_start + length;
            }
            Err(_) => {
                undecodable += 1;
                at += 1;
            }
        }
    }
    undecodable
}

// The same test hub.rs:1238 applies to a live heartbeat, so a log names the same aircraft replayed
// as it did in flight. A GCS, a gimbal, an onboard controller and an ADS-B transponder all send
// heartbeats carrying their own system id, and QGC's own is in every recording it makes:
// gcsMavlinkSystemID defaults to 255 and sendGCSHeartbeat defaults true, so a log of QGC talking
// to nothing still carries a system id, and a head listing those raw names a vehicle that was
// never there.
fn is_vehicle(header: &mavlink::MavHeader, message: &MavMessage) -> bool {
    let MavMessage::HEARTBEAT(beat) = message else { return false };
    let kind = beat.mavtype as u8;
    let excluded = matches!(kind, crate::hub::TYPE_GCS | crate::hub::TYPE_ONBOARD_CONTROLLER | crate::hub::TYPE_GIMBAL | crate::hub::TYPE_ADSB);
    header.component_id == crate::hub::COMP_AUTOPILOT1
        && !excluded
        && beat.autopilot as u8 != crate::hub::AUTOPILOT_INVALID
        && header.system_id != 0
}

pub fn parse(bytes: &[u8]) -> Summary {
    let mut summary = Summary::default();
    summary.undecodable = for_each(bytes, |timestamp, header, message| {
        summary.frames += 1;
        *summary.by_name.entry(message.message_name().to_string()).or_insert(0) += 1;
        summary.first_timestamp_us.get_or_insert(timestamp);
        summary.last_timestamp_us = Some(timestamp);
        if !summary.system_ids.contains(&header.system_id) {
            summary.system_ids.push(header.system_id);
        }
        *summary.by_system.entry(header.system_id).or_default().entry(message.message_name().to_string()).or_insert(0) += 1;
        if is_vehicle(header, message) && !summary.vehicle_system_ids.contains(&header.system_id) {
            summary.vehicle_system_ids.push(header.system_id);
        }
    });
    summary
}

pub fn tlog_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return crate::read::refused("this needs the path of a telemetry log to read, and none was given") };
    let Ok(bytes) = std::fs::read(path) else { return json!({ "kind": "object", "class": "Tlog", "path": path, "readable": false }) };
    let summary = parse(&bytes);
    let span = match (summary.first_timestamp_us, summary.last_timestamp_us) {
        (Some(a), Some(b)) if b >= a => (b - a) as f64 / 1_000_000.0,
        _ => 0.0,
    };
    json!({
        "kind": "object",
        "class": "Tlog",
        "path": path,
        "readable": true,
        "bytes": bytes.len(),
        "frames": summary.frames,
        "undecodable": summary.undecodable,
        "spanSeconds": span,
        "systemIds": summary.system_ids,
        "vehicleSystemIds": summary.vehicle_system_ids,
        "bySystem": summary.by_system,
        "byName": summary.by_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).expect("the sample tlog at the repo root")
    }

    fn independent_frame_count(bytes: &[u8]) -> usize {
        let (mut i, mut frames) = (0usize, 0usize);
        while i + TIMESTAMP_BYTES < bytes.len() {
            let magic = bytes[i + TIMESTAMP_BYTES];
            let total = match magic {
                0xFD => 12 + bytes[i + TIMESTAMP_BYTES + 1] as usize + if bytes[i + TIMESTAMP_BYTES + 2] & 1 != 0 { 13 } else { 0 },
                0xFE => 8 + bytes[i + TIMESTAMP_BYTES + 1] as usize,
                _ => {
                    i += 1;
                    continue;
                }
            };
            frames += 1;
            i += TIMESTAMP_BYTES + total;
        }
        frames
    }

    #[test]
    fn the_ground_station_in_every_recording_is_not_one_of_its_vehicles() {
        use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavAutopilot, MavType};
        let beat = |kind: MavType, autopilot: MavAutopilot| MavMessage::HEARTBEAT(HEARTBEAT_DATA { mavtype: kind, autopilot, ..Default::default() });
        let from = |system: u8, component: u8| mavlink::MavHeader { system_id: system, component_id: component, sequence: 0 };
        let copter = beat(MavType::MAV_TYPE_QUADROTOR, MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA);

        assert!(is_vehicle(&from(1, 1), &copter));
        assert!(
            !is_vehicle(&from(255, 190), &beat(MavType::MAV_TYPE_GCS, MavAutopilot::MAV_AUTOPILOT_INVALID)),
            "gcsMavlinkSystemID defaults to 255 and sendGCSHeartbeat defaults true, so QGC's own heartbeat is in every log it records - a head listing system ids raw names a vehicle for a recording of the station talking to nothing"
        );
        assert!(!is_vehicle(&from(2, 1), &beat(MavType::MAV_TYPE_GIMBAL, MavAutopilot::MAV_AUTOPILOT_INVALID)), "a gimbal heartbeats with its own system id and is not an aircraft, which a GCS-only filter would have missed");
        assert!(!is_vehicle(&from(3, 1), &beat(MavType::MAV_TYPE_ADSB, MavAutopilot::MAV_AUTOPILOT_INVALID)));
        assert!(!is_vehicle(&from(4, 1), &beat(MavType::MAV_TYPE_ONBOARD_CONTROLLER, MavAutopilot::MAV_AUTOPILOT_INVALID)));
        assert!(!is_vehicle(&from(1, 2), &copter), "hub only accepts a heartbeat from the autopilot component, so a camera on the vehicle's own system id does not make a second aircraft");
        assert!(!is_vehicle(&from(1, 1), &beat(MavType::MAV_TYPE_QUADROTOR, MavAutopilot::MAV_AUTOPILOT_INVALID)));
        assert!(!is_vehicle(&from(0, 1), &copter), "system 0 is the broadcast address rather than an aircraft");
        assert!(!is_vehicle(&from(1, 1), &MavMessage::ATTITUDE(Default::default())), "only a heartbeat identifies what a system IS - telemetry alone never promotes an id to an aircraft, which is the case the inclusion rule decides");
    }

    #[test]
    fn the_per_system_census_accounts_for_every_frame_that_decoded() {
        let summary = parse(&sample());
        let counted: usize = summary.by_system.values().flat_map(|names| names.values()).sum();
        assert_eq!(counted, summary.frames, "every decoded frame belongs to exactly one system, so the census has to add up to the total or it is answering about a subset");
        assert_eq!(summary.by_system.keys().copied().collect::<Vec<u8>>(), { let mut ids = summary.system_ids.clone(); ids.sort(); ids }, "the census covers the same systems the id list names and no others");
        assert!(
            summary.vehicle_system_ids.iter().all(|id| summary.system_ids.contains(id)),
            "a vehicle id the system list does not carry would mean the two were counted from different frames"
        );
        assert!(
            summary.by_system.values().all(|names| names.contains_key("HEARTBEAT")),
            "this is the case the inclusion rule turns on - a system with frames and no heartbeat would read as no aircraft. It does not occur in this log, and bySystem is what lets a head check that against its own corpus rather than take it as reasoned"
        );
    }

    #[test]
    fn every_frame_in_the_sample_log_decodes() {
        let bytes = sample();
        let summary = parse(&bytes);
        let expected = independent_frame_count(&bytes);
        assert!(summary.frames > 1000, "parsed only {} frames", summary.frames);
        assert_eq!(summary.frames + summary.undecodable, expected);
        assert!(summary.undecodable * 100 < expected, "{} of {expected} frames did not decode", summary.undecodable);
        assert!(summary.by_name.contains_key("HEARTBEAT"));
        assert!(summary.first_timestamp_us.unwrap() <= summary.last_timestamp_us.unwrap());
        assert!(!summary.system_ids.is_empty());
    }

    #[test]
    fn garbage_between_frames_is_skipped_not_fatal() {
        let bytes = sample();
        let mut damaged = bytes.clone();
        damaged.splice(4000..4000, [0xFEu8, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7]);
        let clean = parse(&bytes).frames;
        let after = parse(&damaged).frames;
        assert!(after + 3 >= clean && after <= clean + 1, "clean {clean}, damaged {after}");
        assert_eq!(parse(&[]).frames, 0);
    }
}
