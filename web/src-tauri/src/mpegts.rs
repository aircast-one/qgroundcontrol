use std::io::Read;
use std::sync::{Arc, Mutex, PoisonError};

use mpeg2ts::es::StreamType;
use mpeg2ts::time::Timestamp;
use mpeg2ts::ts::{Pid, ReadTsPacket, TsPacketReader, TsPayload};

use crate::annexb::{Codec, Unit, unwrapped};

pub const PACKET: usize = 188;
const SYNC: u8 = 0x47;
const HEADER: usize = 4;
const PES_PREFIX: usize = 6;

#[derive(Clone, Default)]
struct Feed(Arc<Mutex<Vec<u8>>>);

impl Read for Feed {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let count = into.len().min(held.len());
        into[..count].copy_from_slice(&held[..count]);
        held.drain(..count);
        Ok(count)
    }
}

pub struct Demuxer {
    feed: Feed,
    reader: TsPacketReader<Feed>,
    pending: Vec<u8>,
    video: Option<(Pid, Codec)>,
    unit: Vec<u8>,
    pts: Option<Timestamp>,
    pes_left: Option<usize>,
    split: bool,
    counter: Option<u8>,
    packets: u16,
    lost: u16,
    clock: Option<u64>,
}

impl Default for Demuxer {
    fn default() -> Self {
        let feed = Feed::default();
        Self {
            reader: TsPacketReader::new(feed.clone()),
            feed,
            pending: Vec::new(),
            video: None,
            unit: Vec::new(),
            pts: None,
            pes_left: None,
            split: false,
            counter: None,
            packets: 0,
            lost: 0,
            clock: None,
        }
    }
}

fn packet_at(bytes: &[u8], from: usize) -> Option<usize> {
    (from..bytes.len())
        .find(|&at| bytes[at] == SYNC && bytes.get(at + PACKET).is_none_or(|next| *next == SYNC))
}

fn aligned(bytes: &[u8]) -> (Vec<&[u8]>, &[u8]) {
    let starts: Vec<usize> =
        std::iter::successors(packet_at(bytes, 0), |&at| packet_at(bytes, at + PACKET)).collect();
    let whole = starts
        .iter()
        .filter(|&&at| at + PACKET <= bytes.len())
        .map(|&at| &bytes[at..at + PACKET])
        .collect();
    let rest = starts
        .last()
        .filter(|&&at| at + PACKET > bytes.len())
        .map_or(&[][..], |&at| &bytes[at..]);
    (whole, rest)
}

fn payload_len(packet: &[u8]) -> usize {
    let adaptation = match packet[3] & 0x20 {
        0 => 0,
        _ => 1 + usize::from(packet[4]),
    };
    (PACKET - HEADER).saturating_sub(adaptation)
}

impl Demuxer {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Unit> {
        let joined = [std::mem::take(&mut self.pending), bytes.to_vec()].concat();
        let (packets, rest) = aligned(&joined);
        self.pending = rest.to_vec();
        packets
            .into_iter()
            .flat_map(|packet| self.packet(packet))
            .collect()
    }

    fn packet(&mut self, packet: &[u8]) -> Vec<Unit> {
        let held = &self.feed.0;
        *held.lock().unwrap_or_else(PoisonError::into_inner) = packet.to_vec();
        let read = self.reader.read_ts_packet();
        held.lock().unwrap_or_else(PoisonError::into_inner).clear();
        let Some(parsed) = read.ok().flatten() else {
            return Vec::new();
        };
        let video = self
            .video
            .filter(|(pid, _)| *pid == parsed.header.pid)
            .map(|(_, codec)| codec);
        if video.is_some() {
            let counter = parsed.header.continuity_counter.as_u8();
            let skipped = match self.counter {
                Some(last) if last != counter => counter.wrapping_sub(last).wrapping_sub(1) & 0x0f,
                _ => 0,
            };
            self.lost = self.lost.saturating_add(u16::from(skipped));
            self.counter = Some(counter);
        }
        match (parsed.payload, video) {
            (Some(TsPayload::Pmt(pmt)), _) => {
                self.video = self.video.or_else(|| {
                    pmt.es_info.iter().find_map(|es| match es.stream_type {
                        StreamType::H264 => Some((es.elementary_pid, Codec::H264)),
                        StreamType::H265 => Some((es.elementary_pid, Codec::H265)),
                        _ => None,
                    })
                });
                Vec::new()
            }
            (Some(TsPayload::PesStart(pes)), Some(codec)) => {
                let continues = pes.header.pts.is_none() || pes.header.pts == self.pts;
                self.split |= continues && self.pts.is_some();
                let before = (!continues && !self.unit.is_empty()).then(|| self.unit(codec));
                self.packets = self.packets.saturating_add(1);
                self.pts = pes.header.pts.or(self.pts);
                self.unit.extend_from_slice(&pes.data);
                self.pes_left = (pes.pes_packet_len > 0).then(|| {
                    (PES_PREFIX + usize::from(pes.pes_packet_len))
                        .saturating_sub(payload_len(packet))
                });
                before.into_iter().chain(self.complete(codec)).collect()
            }
            (Some(TsPayload::PesContinuation(bytes)), Some(codec)) => {
                self.packets = self.packets.saturating_add(1);
                self.unit.extend_from_slice(&bytes);
                self.pes_left = self.pes_left.map(|left| left.saturating_sub(bytes.len()));
                self.complete(codec).into_iter().collect()
            }
            _ => Vec::new(),
        }
    }

    fn complete(&mut self, codec: Codec) -> Option<Unit> {
        (!self.split && self.pes_left == Some(0) && !self.unit.is_empty()).then(|| self.unit(codec))
    }

    fn unit(&mut self, codec: Codec) -> Unit {
        let clock = self.pts.map(|pts| unwrapped(self.clock, pts.as_u64(), 33));
        self.clock = clock.or(self.clock);
        Unit {
            codec,
            data: std::mem::take(&mut self.unit),
            packets: std::mem::take(&mut self.packets),
            lost: std::mem::take(&mut self.lost),
            timestamp: clock.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mpeg2ts::es::StreamId;
    use mpeg2ts::pes::PesHeader;
    use mpeg2ts::ts::payload::{Bytes, Pat, Pes, Pmt};
    use mpeg2ts::ts::{
        ContinuityCounter, EsInfo, ProgramAssociation, TransportScramblingControl, TsHeader,
        TsPacket, TsPacketWriter, VersionNumber, WriteTsPacket,
    };

    fn header(pid: u16) -> TsHeader {
        TsHeader {
            transport_error_indicator: false,
            transport_priority: false,
            pid: Pid::new(pid).unwrap(),
            transport_scrambling_control: TransportScramblingControl::NotScrambled,
            continuity_counter: ContinuityCounter::new(),
        }
    }

    fn written(payload: TsPayload, pid: u16) -> Vec<u8> {
        let mut writer = TsPacketWriter::new(Vec::new());
        writer
            .write_ts_packet(&TsPacket {
                header: header(pid),
                adaptation_field: None,
                payload: Some(payload),
            })
            .unwrap();
        writer.into_stream()
    }

    fn pes(pts: u64, sized: bool, data: &[u8]) -> TsPayload {
        TsPayload::PesStart(Pes {
            header: PesHeader {
                stream_id: StreamId::new(0xe0),
                priority: false,
                data_alignment_indicator: true,
                copyright: false,
                original_or_copy: false,
                pts: Some(Timestamp::new(pts).unwrap()),
                dts: None,
                escr: None,
            },
            pes_packet_len: if sized { 8 + data.len() as u16 } else { 0 },
            data: Bytes::new(data).unwrap(),
        })
    }

    #[test]
    fn a_video_pes_becomes_an_access_unit_when_the_next_one_starts() {
        let pat = written(
            TsPayload::Pat(Pat {
                transport_stream_id: 1,
                version_number: VersionNumber::new(),
                table: vec![ProgramAssociation {
                    program_num: 1,
                    program_map_pid: Pid::new(0x100).unwrap(),
                }],
            }),
            0,
        );
        let pmt = written(
            TsPayload::Pmt(Pmt {
                program_num: 1,
                pcr_pid: None,
                version_number: VersionNumber::new(),
                program_info: vec![],
                es_info: vec![EsInfo {
                    stream_type: StreamType::H265,
                    elementary_pid: Pid::new(0x101).unwrap(),
                    descriptors: vec![],
                }],
            }),
            0x100,
        );
        let first = written(pes(3000, false, &[0, 0, 0, 1, 0x26, 0x01]), 0x101);
        let slice = written(pes(3000, false, &[0, 0, 0, 1, 0x26, 0x01, 0x80]), 0x101);
        let second = written(pes(6000, false, &[0, 0, 0, 1, 0x02, 0x01]), 0x101);
        let mut demuxer = Demuxer::default();
        let stream = [&[0x12, 0x34][..], &pat, &pmt, &first].concat();
        assert!(
            demuxer.push(&stream[..300]).is_empty(),
            "junk before the first sync byte is skipped and a split packet waits"
        );
        assert!(demuxer.push(&stream[300..]).is_empty());
        assert!(
            demuxer.push(&slice).is_empty(),
            "a second slice of the same frame carries the same PTS"
        );
        let units = demuxer.push(&second);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].codec, Codec::H265);
        assert!(units[0].data.starts_with(&[0, 0, 0, 1, 0x26, 0x01]));
        assert_eq!(
            super::super::annexb::nals(&units[0].data).len(),
            2,
            "both slices are in the one access unit"
        );

        let mut whole_frames = Demuxer::default();
        assert!(whole_frames.push(&[pat, pmt].concat()).is_empty());
        assert_eq!(
            whole_frames
                .push(&written(pes(3000, true, &[0, 0, 0, 1, 0x26, 0x01]), 0x101))
                .len(),
            1,
            "a PES that says how long it is goes out as soon as it is whole, without waiting for the next frame"
        );
    }

    #[test]
    fn a_stream_that_loses_bytes_mid_packet_finds_the_next_packet_again() {
        let pat = written(
            TsPayload::Pat(Pat {
                transport_stream_id: 1,
                version_number: VersionNumber::new(),
                table: vec![ProgramAssociation {
                    program_num: 1,
                    program_map_pid: Pid::new(0x100).unwrap(),
                }],
            }),
            0,
        );
        let packets = [&pat[..], &pat, &pat].concat();
        let torn = [&packets[..PACKET], &packets[PACKET + 50..]].concat();
        let (whole, rest) = aligned(&torn);
        assert_eq!(
            (whole, rest.len()),
            (vec![&packets[2 * PACKET..]], 0),
            "after the torn packet, the next packet that a sync byte follows is found again"
        );
        let stray = [&[0x47, 0x47, 1][..], &packets].concat();
        let (whole, rest) = aligned(&stray);
        assert_eq!(
            (whole.len(), rest.len()),
            (3, 0),
            "a stray sync byte is not a packet start"
        );
    }
}
