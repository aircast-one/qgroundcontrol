use crate::annexb::{Codec, START, Unit};

const REORDER_WINDOW: usize = 8;

pub struct Depacketizer {
    codec: Codec,
    next: Option<u16>,
    held: Vec<(u16, Vec<u8>)>,
    timestamp: Option<u32>,
    unit: Vec<u8>,
    fragment: Option<Vec<u8>>,
    packets: u16,
    lost: u16,
    behind: usize,
}

struct Packet<'a> {
    timestamp: u32,
    marker: bool,
    payload: &'a [u8],
}

fn parse(packet: &[u8]) -> Option<Packet<'_>> {
    let first = *packet.first()?;
    (first >> 6 == 2).then_some(())?;
    let csrc_end = 12 + 4 * usize::from(first & 0x0f);
    let start = match first & 0x10 {
        0 => csrc_end,
        _ => {
            let words =
                u16::from_be_bytes(packet.get(csrc_end + 2..csrc_end + 4)?.try_into().ok()?);
            csrc_end + 4 + 4 * usize::from(words)
        }
    };
    let padding = match first & 0x20 {
        0 => 0,
        _ => usize::from(*packet.last()?),
    };
    Some(Packet {
        timestamp: u32::from_be_bytes(packet.get(4..8)?.try_into().ok()?),
        marker: packet[1] & 0x80 != 0,
        payload: packet.get(start..packet.len().checked_sub(padding)?)?,
    })
}

fn aggregated(units: &[u8]) -> Vec<&[u8]> {
    std::iter::successors(Some(units), |rest| {
        let size = usize::from(u16::from_be_bytes(rest.get(..2)?.try_into().ok()?));
        rest.get(2 + size..)
    })
    .filter_map(|rest| {
        let size = usize::from(u16::from_be_bytes(rest.get(..2)?.try_into().ok()?));
        rest.get(2..2 + size)
    })
    .filter(|nal| !nal.is_empty())
    .collect()
}

impl Depacketizer {
    pub fn new(codec: Codec) -> Self {
        Self {
            codec,
            next: None,
            held: Vec::new(),
            timestamp: None,
            unit: Vec::new(),
            fragment: None,
            packets: 0,
            lost: 0,
            behind: 0,
        }
    }

    fn resync(&mut self) {
        self.next = None;
        self.held.clear();
        self.timestamp = None;
        self.unit.clear();
        self.fragment = None;
        self.behind = 0;
    }

    pub fn push(&mut self, packet: &[u8]) -> Vec<Unit> {
        let Some(sequence) = packet
            .get(2..4)
            .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
        else {
            return Vec::new();
        };
        let ahead = self
            .next
            .map_or(0, |next| sequence.wrapping_sub(next) as i16);
        if ahead < 0 {
            self.behind += 1;
            if self.behind <= REORDER_WINDOW {
                return Vec::new();
            }
            self.resync();
        }
        if self.held.iter().any(|(held, _)| *held == sequence) {
            return Vec::new();
        }
        self.behind = 0;
        self.held.push((sequence, packet.to_vec()));
        if self.held.len() > REORDER_WINDOW {
            let next = self.next.unwrap_or(sequence);
            let resumed = self
                .held
                .iter()
                .map(|(held, _)| *held)
                .min_by_key(|held| held.wrapping_sub(next));
            self.lost = self
                .lost
                .saturating_add(resumed.map_or(0, |resumed| resumed.wrapping_sub(next)));
            self.next = resumed;
            self.fragment = None;
        }
        self.drain()
    }

    fn drain(&mut self) -> Vec<Unit> {
        let next = self.next.unwrap_or_else(|| self.held[0].0);
        match self.held.iter().position(|(held, _)| *held == next) {
            None => Vec::new(),
            Some(at) => {
                let (_, packet) = self.held.swap_remove(at);
                self.next = Some(next.wrapping_add(1));
                let units = parse(&packet)
                    .map(|parsed| self.take(&parsed))
                    .unwrap_or_default();
                units.into_iter().chain(self.drain()).collect()
            }
        }
    }

    fn take(&mut self, packet: &Packet<'_>) -> Vec<Unit> {
        let before = match self.timestamp {
            Some(timestamp) if timestamp != packet.timestamp => self.finish(),
            _ => None,
        };
        self.timestamp = Some(packet.timestamp);
        self.packets = self.packets.saturating_add(1);
        let nals = self.depacketize(packet.payload);
        self.unit.extend(
            nals.iter()
                .flat_map(|nal| START.iter().chain(nal.iter()).copied()),
        );
        let after = packet.marker.then(|| self.finish()).flatten();
        before.into_iter().chain(after).collect()
    }

    fn finish(&mut self) -> Option<Unit> {
        self.timestamp = None;
        let unit = Unit {
            codec: self.codec,
            data: std::mem::take(&mut self.unit),
            packets: std::mem::take(&mut self.packets),
            lost: std::mem::take(&mut self.lost),
        };
        (!unit.data.is_empty()).then_some(unit)
    }

    fn depacketize(&mut self, payload: &[u8]) -> Vec<Vec<u8>> {
        let Some(&first) = payload.first() else {
            return Vec::new();
        };
        match self.codec {
            Codec::H264 => match first & 0x1f {
                1..=23 => vec![payload.to_vec()],
                24 => aggregated(&payload[1..])
                    .into_iter()
                    .map(<[u8]>::to_vec)
                    .collect(),
                28 => payload.get(1).map_or_else(Vec::new, |&header| {
                    self.fragmented(header, &[(first & 0xe0) | (header & 0x1f)], &payload[2..])
                }),
                _ => Vec::new(),
            },
            Codec::H265 => match (first >> 1) & 0x3f {
                0..=47 => vec![payload.to_vec()],
                48 => payload.get(2..).map_or_else(Vec::new, |units| {
                    aggregated(units).into_iter().map(<[u8]>::to_vec).collect()
                }),
                49 => match payload.get(..3) {
                    Some(&[_, second, header]) => self.fragmented(
                        header,
                        &[(first & 0x81) | ((header & 0x3f) << 1), second],
                        &payload[3..],
                    ),
                    _ => Vec::new(),
                },
                _ => Vec::new(),
            },
        }
    }

    fn fragmented(&mut self, header: u8, nal_header: &[u8], data: &[u8]) -> Vec<Vec<u8>> {
        let (starts, ends) = (header & 0x80 != 0, header & 0x40 != 0);
        self.fragment = match (starts, self.fragment.take()) {
            (true, _) => Some([nal_header, data].concat()),
            (false, Some(begun)) => Some([begun, data.to_vec()].concat()),
            (false, None) => None,
        };
        match ends {
            true => self.fragment.take().into_iter().collect(),
            false => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(sequence: u16, timestamp: u32, marker: bool, payload: &[u8]) -> Vec<u8> {
        [
            &[0x80, if marker { 0xe0 } else { 0x60 }][..],
            &sequence.to_be_bytes(),
            &timestamp.to_be_bytes(),
            &[0, 0, 0, 1],
            payload,
        ]
        .concat()
    }

    #[test]
    fn a_sender_that_restarts_behind_the_old_sequence_is_followed_after_the_reorder_window() {
        let mut h264 = Depacketizer::new(Codec::H264);
        let frame = |sequence: u16| packet(sequence, u32::from(sequence), true, &[0x41, 0x9a]);
        assert_eq!(h264.push(&frame(40_000)).len(), 1);
        assert_eq!(h264.push(&frame(40_001)).len(), 1);
        let restarted: Vec<usize> = (20_000..20_000 + REORDER_WINDOW as u16 + 3)
            .map(|sequence| h264.push(&frame(sequence)).len())
            .collect();
        assert_eq!(
            restarted[..REORDER_WINDOW],
            [0; REORDER_WINDOW],
            "a few packets from behind are late copies and are dropped"
        );
        assert_eq!(
            restarted[REORDER_WINDOW..],
            [1, 1, 1],
            "more than a reorder window of them is a new sequence, and video resumes"
        );
        assert!(
            h264.push(&frame(40_002)).is_empty(),
            "a stray packet from the old sequence waits in the reorder window instead of playing"
        );
    }

    #[test]
    fn reordered_fragments_rebuild_one_access_unit_and_a_lost_one_is_skipped() {
        let mut h264 = Depacketizer::new(Codec::H264);
        let parameters = packet(10, 90, false, &[0x18, 0, 2, 0x67, 0x42, 0, 2, 0x68, 0xce]);
        let first = packet(11, 90, false, &[0x7c, 0x85, 0xaa]);
        let last = packet(12, 90, true, &[0x7c, 0x45, 0xbb]);
        assert!(h264.push(&parameters).is_empty());
        assert!(
            h264.push(&last).is_empty(),
            "the end waits for the fragment before it"
        );
        assert_eq!(
            h264.push(&first),
            [Unit {
                codec: Codec::H264,
                data: [
                    &START[..],
                    &[0x67, 0x42],
                    &START,
                    &[0x68, 0xce],
                    &START,
                    &[0x65, 0xaa, 0xbb]
                ]
                .concat(),
                packets: 3,
                lost: 0,
            }]
        );
        assert!(h264.push(&first).is_empty(), "a late copy is dropped");
        let units: Vec<Unit> = (14..14 + REORDER_WINDOW as u16 + 1)
            .flat_map(|sequence| {
                h264.push(&packet(
                    sequence,
                    u32::from(sequence) * 100,
                    true,
                    &[0x41, 0x9a],
                ))
            })
            .collect();
        assert_eq!(
            units.len(),
            REORDER_WINDOW + 1,
            "packet 13 never came, so the window moves past it"
        );
        assert_eq!(units[0].lost, 1, "and the next frame reports it lost");

        let mut h265 = Depacketizer::new(Codec::H265);
        assert!(
            h265.push(&packet(1, 9, false, &[0x62, 0x01, 0x93, 0x11]))
                .is_empty()
        );
        assert_eq!(
            h265.push(&packet(2, 9, true, &[0x62, 0x01, 0x53, 0x22]))[0].data,
            [&START[..], &[0x26, 0x01, 0x11, 0x22]].concat()
        );
    }
}
