use mavlink::dialects::ardupilotmega::MavMessage;
use mavlink::peek_reader::PeekReader;
use mavlink::{MavHeader, ReadVersion, read_versioned_msg};
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::tlog::frame_length;

pub type LinkId = u32;
const MAX_BUFFER: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Core,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Open,
    Closed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub frames_in: u64,
    pub dropped: u64,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: LinkId,
    pub kind: String,
    pub name: String,
    pub owner: Owner,
    pub state: State,
    pub reason: String,
    pub stats: Stats,
    buffer: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Registry {
    next: LinkId,
    links: BTreeMap<LinkId, Entry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub link: LinkId,
    pub header: MavHeader,
    pub message: MavMessage,
    pub raw: Vec<u8>,
}

fn drain(buffer: &mut Vec<u8>, stats: &mut Stats, link: LinkId) -> Vec<Frame> {
    let mut frames = Vec::new();
    let mut at = 0usize;
    while at < buffer.len() {
        let Some((version, length)) = frame_length(&buffer[at..]) else {
            if buffer.len() - at < 3 {
                break;
            }
            at += 1;
            continue;
        };
        if at + length > buffer.len() {
            break;
        }
        let raw = &buffer[at..at + length];
        match read_versioned_msg::<MavMessage, _>(&mut PeekReader::new(raw), ReadVersion::Single(version)) {
            Ok((header, message)) => {
                stats.frames_in += 1;
                frames.push(Frame { link, header, message, raw: raw.to_vec() });
                at += length;
            }
            Err(_) => {
                stats.dropped += 1;
                at += 1;
            }
        }
    }
    buffer.drain(..at);
    if buffer.len() > MAX_BUFFER {
        let excess = buffer.len() - MAX_BUFFER;
        buffer.drain(..excess);
        stats.dropped += 1;
    }
    frames
}

impl Registry {
    pub fn open(&mut self, owner: Owner, kind: &str, name: &str) -> LinkId {
        self.next += 1;
        let id = self.next;
        self.links.insert(id, Entry { id, kind: kind.to_string(), name: name.to_string(), owner, state: State::Open, reason: String::new(), stats: Stats::default(), buffer: Vec::new() });
        id
    }

    pub fn bytes_in(&mut self, id: LinkId, bytes: &[u8]) -> Vec<Frame> {
        let Some(entry) = self.links.get_mut(&id).filter(|e| e.state == State::Open) else { return Vec::new() };
        entry.stats.bytes_in += bytes.len() as u64;
        entry.buffer.extend_from_slice(bytes);
        drain(&mut entry.buffer, &mut entry.stats, id)
    }

    pub fn wrote(&mut self, id: LinkId, len: usize) -> bool {
        match self.links.get_mut(&id).filter(|e| e.state == State::Open) {
            Some(entry) => {
                entry.stats.bytes_out += len as u64;
                true
            }
            None => false,
        }
    }

    pub fn close(&mut self, id: LinkId, reason: &str) -> bool {
        match self.links.get_mut(&id) {
            Some(entry) => {
                entry.state = State::Closed;
                entry.reason = reason.to_string();
                entry.buffer.clear();
                true
            }
            None => false,
        }
    }

    pub fn remove_closed(&mut self) {
        self.links.retain(|_, e| e.state == State::Open);
    }

    pub fn entry(&self, id: LinkId) -> Option<&Entry> {
        self.links.get(&id)
    }

    pub fn open_ids(&self) -> Vec<LinkId> {
        self.links.values().filter(|e| e.state == State::Open).map(|e| e.id).collect()
    }

    pub fn snapshot(&self) -> Value {
        let links: Vec<Value> = self
            .links
            .values()
            .map(|e| {
                json!({
                    "id": e.id,
                    "kind": e.kind,
                    "name": e.name,
                    "owner": match e.owner { Owner::Core => "core", Owner::Host => "host" },
                    "state": match e.state { State::Open => "open", State::Closed => "closed" },
                    "reason": e.reason,
                    "bytesIn": e.stats.bytes_in,
                    "bytesOut": e.stats.bytes_out,
                    "framesIn": e.stats.frames_in,
                    "dropped": e.stats.dropped,
                })
            })
            .collect();
        json!({ "kind": "object", "class": "Transports", "links": links, "openCount": self.open_ids().len() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames_of_sample() -> Vec<Vec<u8>> {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
        crate::tlog::entries(&bytes, u64::MAX).into_iter().map(|(_, f)| f).collect()
    }

    #[test]
    fn bytes_arriving_in_any_chunking_frame_the_same_messages() {
        let frames = frames_of_sample();
        let stream: Vec<u8> = frames.iter().flatten().copied().collect();
        let expected = frames.len() as u64;
        for chunk in [1usize, 7, 64, 300, 4096, stream.len()] {
            let mut registry = Registry::default();
            let id = registry.open(Owner::Host, "usb", "Pixhawk");
            let seen: usize = stream.chunks(chunk).map(|c| registry.bytes_in(id, c).len()).sum();
            let entry = registry.entry(id).unwrap();
            assert_eq!((seen as u64, entry.stats.frames_in, entry.stats.dropped, entry.stats.bytes_in), (expected, expected, 0, stream.len() as u64), "chunk {chunk}");
        }
    }

    #[test]
    fn garbage_is_dropped_and_the_stream_resyncs() {
        let frames = frames_of_sample();
        let mut registry = Registry::default();
        let id = registry.open(Owner::Core, "udp", "UDP Link");
        let first = registry.bytes_in(id, &frames[0]);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].raw, frames[0]);
        let mut noisy = vec![0xFEu8, 0xFF, 1, 2, 3];
        frames[1..=40].iter().for_each(|f| noisy.extend_from_slice(f));
        let recovered = registry.bytes_in(id, &noisy).len();
        assert!(recovered >= 25, "recovered {recovered}");
        let stats = &registry.entry(id).unwrap().stats;
        assert!(stats.dropped >= 1 && stats.frames_in == 1 + recovered as u64);
        let mut quiet = Registry::default();
        let q = quiet.open(Owner::Host, "usb", "partial");
        assert!(quiet.bytes_in(q, &frames[0][..1]).is_empty());
        assert_eq!(quiet.bytes_in(q, &frames[0][1..]).len(), 1);
    }

    #[test]
    fn closed_links_take_no_bytes_and_the_snapshot_reports_both_owners() {
        let mut registry = Registry::default();
        let core = registry.open(Owner::Core, "tcp", "SITL");
        let host = registry.open(Owner::Host, "bluetooth", "Radio");
        assert!(registry.wrote(core, 12));
        assert!(registry.close(host, "device unplugged"));
        assert!(registry.bytes_in(host, &[0xFD; 20]).is_empty());
        assert!(!registry.wrote(host, 1));
        let snapshot = registry.snapshot();
        assert_eq!(snapshot["openCount"], 1);
        assert_eq!(snapshot["links"][1]["state"], "closed");
        assert_eq!(snapshot["links"][1]["reason"], "device unplugged");
        assert_eq!(snapshot["links"][0]["bytesOut"], 12);
        registry.remove_closed();
        assert_eq!(registry.open_ids(), vec![core]);
        let mut flood = Registry::default();
        let id = flood.open(Owner::Host, "usb", "noise");
        flood.bytes_in(id, &vec![0x00u8; MAX_BUFFER + 100]);
        assert!(flood.entry(id).unwrap().buffer.len() <= MAX_BUFFER);
    }
}
