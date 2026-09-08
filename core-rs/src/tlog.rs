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

pub fn entries(bytes: &[u8]) -> Vec<(u64, Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + TIMESTAMP_BYTES < bytes.len() {
        let frame_start = at + TIMESTAMP_BYTES;
        let Some((_, length)) = frame_length(&bytes[frame_start..]) else {
            at += 1;
            continue;
        };
        let Some(frame) = bytes.get(frame_start..frame_start + length) else { break };
        out.push((u64::from_be_bytes(bytes[at..frame_start].try_into().unwrap()), frame.to_vec()));
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
            Ok((header, message)) => visit(timestamp, &header, &message),
            Err(_) => undecodable += 1,
        }
        at = frame_start + length;
    }
    undecodable
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
    });
    summary
}

pub fn tlog_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return json!({ "kind": "null" }) };
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
        damaged.splice(4000..4000, [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let clean = parse(&bytes).frames;
        let after = parse(&damaged).frames;
        assert!(after + 3 >= clean && after <= clean + 1, "clean {clean}, damaged {after}");
        assert_eq!(parse(&[]).frames, 0);
    }
}
