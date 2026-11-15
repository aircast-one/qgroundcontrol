use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use muxide::api::{MuxerBuilder, VideoCodec};
use muxide::fragmented::FragmentedMuxer;

use crate::annexb::{self, Codec};

const TIMESCALE: u64 = 90_000;
const SEGMENT: i64 = 90_000;
const LONGEST_SEGMENT: i64 = 10 * 90_000;
const STALL: i64 = 45_000;
const FRAME_STEP: i64 = 3_000;
const LARGEST_GAP: i64 = 10 * 90_000;
const REORDERED_WITHIN: i64 = 45_000;
const REORDER_DEPTH: usize = 4;
const KEYFRAMES_WITHOUT_PARAMETERS: u32 = 3;
const FRAME_RATE_UNUSED_BY_FRAGMENTS: f64 = 30.0;

const FORMAT_CHANGED: &str = "The camera changed its picture format, so the recording stopped.";
const NO_PARAMETERS: &str =
    "The camera sends no picture parameters (SPS/PPS), so its video cannot be recorded.";

#[derive(Debug, PartialEq)]
pub struct Written {
    pub file: PathBuf,
    pub frames: u64,
    pub bytes: u64,
    pub seconds: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Parameters {
    vps: Option<Vec<u8>>,
    sps: Option<Vec<u8>>,
    pps: BTreeMap<u32, Vec<u8>>,
}

#[derive(Clone, Copy)]
struct Clock {
    base: i64,
    offset: i64,
    last: i64,
}

struct Pending {
    at: i64,
    sample: Vec<u8>,
    key: bool,
}

struct Started {
    muxer: FragmentedMuxer,
    codec: Codec,
    parameters: Parameters,
}

pub struct Recording {
    path: PathBuf,
    file: File,
    size: (u32, u32),
    started: Option<Started>,
    seen: Parameters,
    keyframes_without_parameters: u32,
    clock: Option<Clock>,
    pending: VecDeque<Pending>,
    presented: BinaryHeap<Reverse<i64>>,
    decoded: i64,
    flushed: i64,
    previous: Option<i64>,
    frames: u64,
    bytes: u64,
}

pub fn inside_folder(folder: &Path, file: &Path) -> Result<PathBuf, String> {
    let lexically_inside = file.is_absolute()
        && file
            .components()
            .all(|part| !matches!(part, Component::ParentDir | Component::CurDir))
        && file.starts_with(folder);
    let parent = file
        .parent()
        .filter(|_| lexically_inside)
        .ok_or_else(|| format!("{} is outside the video folder.", file.display()))?;
    let canonical = |path: &Path| path.canonicalize().map_err(|error| error.to_string());
    let folder = canonical(folder)?;
    let leaves = |path: &Path| -> Result<(), String> {
        canonical(path)?
            .starts_with(&folder)
            .then_some(())
            .ok_or_else(|| format!("{} leaves the video folder.", file.display()))
    };
    leaves(
        parent
            .ancestors()
            .find(|ancestor| ancestor.exists())
            .ok_or_else(|| format!("{} has no folder to go in.", file.display()))?,
    )?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    leaves(parent)?;
    Ok(file.to_path_buf())
}

fn pps_id(codec: Codec, pps: &[u8]) -> Option<u32> {
    let header = match codec {
        Codec::H264 => 1,
        Codec::H265 => 2,
    };
    let rbsp = annexb::unescaped(pps.get(header..)?);
    let bits = rbsp
        .iter()
        .flat_map(|byte| (0..8).rev().map(move |shift| (byte >> shift) & 1));
    let zeros = bits.clone().take_while(|bit| *bit == 0).count();
    let value = bits
        .skip(zeros + 1)
        .take(zeros)
        .fold(0u64, |read, bit| (read << 1) | u64::from(bit));
    (zeros < 32)
        .then(|| (1u64 << zeros) - 1 + value)
        .and_then(|id| u32::try_from(id).ok())
}

fn kinds(codec: Codec) -> (Option<u8>, u8, u8) {
    match codec {
        Codec::H264 => (None, 7, 8),
        Codec::H265 => (Some(32), 33, 34),
    }
}

fn parameter_set(codec: Codec, nals: &[&[u8]], kind: u8) -> Option<Vec<u8>> {
    nals.iter()
        .find(|nal| annexb::nal_type(codec, nal) == kind)
        .map(|nal| nal.to_vec())
}

fn sample(codec: Codec, nals: &[&[u8]], header_pps: &[u8]) -> Vec<u8> {
    let (_, _, pps) = kinds(codec);
    let in_band = match codec {
        Codec::H264 => 7..=9,
        Codec::H265 => 32..=35,
    };
    nals.iter()
        .filter(|nal| {
            let kind = annexb::nal_type(codec, nal);
            !in_band.contains(&kind) || (kind == pps && **nal != header_pps)
        })
        .flat_map(|nal| {
            [
                &u32::try_from(nal.len()).unwrap_or(0).to_be_bytes()[..],
                nal,
            ]
            .concat()
        })
        .collect()
}

impl Parameters {
    fn updated(&self, codec: Codec, nals: &[&[u8]]) -> Self {
        let (vps, sps, pps) = kinds(codec);
        Self {
            vps: vps
                .and_then(|kind| parameter_set(codec, nals, kind))
                .or_else(|| self.vps.clone()),
            sps: parameter_set(codec, nals, sps).or_else(|| self.sps.clone()),
            pps: self
                .pps
                .clone()
                .into_iter()
                .chain(
                    nals.iter()
                        .filter(|nal| annexb::nal_type(codec, nal) == pps)
                        .filter_map(|nal| pps_id(codec, nal).map(|id| (id, nal.to_vec()))),
                )
                .collect(),
        }
    }

    fn complete(&self, codec: Codec) -> bool {
        self.sps.is_some() && !self.pps.is_empty() && (codec == Codec::H264 || self.vps.is_some())
    }

    fn header_pps(&self) -> &[u8] {
        self.pps.values().next().map_or(&[], Vec::as_slice)
    }

    fn still(&self, now: &Self) -> bool {
        self.vps == now.vps
            && self.sps == now.sps
            && self
                .pps
                .iter()
                .all(|(id, pps)| now.pps.get(id) == Some(pps))
    }
}

fn muxer(
    codec: Codec,
    parameters: &Parameters,
    (width, height): (u32, u32),
) -> Result<FragmentedMuxer, String> {
    let video = match codec {
        Codec::H264 => VideoCodec::H264,
        Codec::H265 => VideoCodec::H265,
    };
    let builder = MuxerBuilder::new(std::io::sink())
        .video(video, width, height, FRAME_RATE_UNUSED_BY_FRAGMENTS)
        .with_sps(parameters.sps.clone().ok_or(NO_PARAMETERS)?)
        .with_pps(parameters.header_pps().to_vec());
    let builder = match &parameters.vps {
        Some(vps) => builder.with_vps(vps.clone()),
        None => builder,
    };
    builder
        .new_with_fragment()
        .map_err(|error| format!("The recording could not start: {error}"))
}

impl Clock {
    fn starting(timestamp: u64) -> Self {
        Self {
            base: i64::try_from(timestamp).unwrap_or(i64::MAX),
            offset: 0,
            last: -FRAME_STEP,
        }
    }

    fn placed(self, timestamp: u64) -> (Self, i64) {
        let stamped = i64::try_from(timestamp).unwrap_or(i64::MAX) - self.base + self.offset;
        let behind = self.last - stamped;
        let reordered = behind > 0 && behind < REORDERED_WITHIN;
        let jumped = !reordered && (behind >= 0 || -behind > LARGEST_GAP);
        let at = if jumped {
            self.last + FRAME_STEP
        } else {
            stamped
        };
        (
            Self {
                offset: self.offset + (at - stamped),
                last: self.last.max(at),
                ..self
            },
            at,
        )
    }
}

fn refused(reason: &str) -> std::io::Error {
    std::io::Error::other(reason.to_string())
}

impl Recording {
    pub fn create(path: &Path, size: (u32, u32)) -> std::io::Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            file: OpenOptions::new().write(true).create_new(true).open(path)?,
            size,
            started: None,
            seen: Parameters::default(),
            keyframes_without_parameters: 0,
            clock: None,
            pending: VecDeque::new(),
            presented: BinaryHeap::new(),
            decoded: 0,
            flushed: 0,
            previous: None,
            frames: 0,
            bytes: 0,
        })
    }

    fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.file.write_all(bytes)?;
        self.bytes += bytes.len() as u64;
        Ok(())
    }

    pub fn written(&self) -> Written {
        Written {
            file: self.path.clone(),
            frames: self.frames,
            bytes: self.bytes,
            seconds: self
                .clock
                .map_or(0.0, |clock| clock.last as f64 / TIMESCALE as f64),
        }
    }

    fn start(&mut self, codec: Codec, key: bool) -> std::io::Result<()> {
        if self.started.is_some() || !key {
            return Ok(());
        }
        if !self.seen.complete(codec) {
            self.keyframes_without_parameters += 1;
            return match self.keyframes_without_parameters >= KEYFRAMES_WITHOUT_PARAMETERS {
                true => Err(refused(NO_PARAMETERS)),
                false => Ok(()),
            };
        }
        let mut muxer = muxer(codec, &self.seen, self.size).map_err(|reason| refused(&reason))?;
        let init = muxer.init_segment();
        self.started = Some(Started {
            muxer,
            codec,
            parameters: self.seen.clone(),
        });
        self.append(&init)
    }

    pub fn write(
        &mut self,
        codec: Codec,
        nals: &[&[u8]],
        key: bool,
        timestamp: u64,
    ) -> std::io::Result<()> {
        self.seen = self.seen.updated(codec, nals);
        if let Some(started) = &self.started
            && (started.codec != codec || !started.parameters.still(&self.seen))
        {
            return Err(refused(FORMAT_CHANGED));
        }
        self.start(codec, key)?;
        if self.started.is_none() {
            return Ok(());
        }
        let (clock, at) = self
            .clock
            .unwrap_or_else(|| Clock::starting(timestamp))
            .placed(timestamp);
        self.clock = Some(clock);
        self.frames += 1;
        let header_pps = self
            .started
            .as_ref()
            .map(|started| started.parameters.header_pps().to_vec())
            .unwrap_or_default();
        self.pending.push_back(Pending {
            at,
            sample: sample(codec, nals, &header_pps),
            key,
        });
        self.presented.push(Reverse(at));
        match self.pending.len() > REORDER_DEPTH {
            true => self.decode_oldest(),
            false => Ok(()),
        }
    }

    fn decode_oldest(&mut self) -> std::io::Result<()> {
        let Some(frame) = self.pending.pop_front() else {
            return Ok(());
        };
        let decoded = self
            .presented
            .pop()
            .map_or(frame.at, |Reverse(earliest)| earliest)
            .max(self.decoded);
        let after_stall = self
            .previous
            .is_some_and(|previous| decoded - previous > STALL);
        let due = decoded - self.flushed >= if frame.key { SEGMENT } else { LONGEST_SEGMENT };
        self.previous = Some(decoded);
        self.decoded = decoded;
        if due && !after_stall {
            self.flushed = decoded;
            if let Some(segment) = self
                .started
                .as_mut()
                .and_then(|started| started.muxer.flush_segment())
            {
                self.append(&segment)?;
            }
        }
        let stamp = |time: i64| u64::try_from(time).unwrap_or(0);
        self.started.as_mut().map_or(Ok(()), |started| {
            started
                .muxer
                .write_video(stamp(frame.at), stamp(decoded), &frame.sample, frame.key)
                .map_err(|error| std::io::Error::other(error.to_string()))
        })
    }

    pub fn finish(mut self) -> std::io::Result<Written> {
        (0..self.pending.len()).try_for_each(|_| self.decode_oldest())?;
        if let Some(segment) = self
            .started
            .as_mut()
            .and_then(|started| started.muxer.flush_segment())
        {
            self.append(&segment)?;
        }
        let written = self.written();
        if written.frames == 0 {
            drop(self.file);
            std::fs::remove_file(&written.file)?;
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annexb::START;

    const SPS: [u8; 7] = [0x67, 0x42, 0xc0, 0x1f, 0xda, 0x05, 0x02];
    const PPS: [u8; 4] = [0x68, 0xce, 0x3c, 0x80];

    fn unit(parts: &[&[u8]]) -> Vec<u8> {
        parts
            .iter()
            .flat_map(|part| [&START[..], part].concat())
            .collect()
    }

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("aircast-{name}-{}.mp4", std::process::id()))
    }

    #[test]
    fn a_recording_starts_at_a_keyframe_and_is_a_fragmented_mp4() {
        let path = scratch("record");
        let key = unit(&[&SPS, &PPS, &[0x65, 0x88, 0x84]]);
        let delta = unit(&[&[0x41, 0x9a, 0x02]]);
        let mut recording = Recording::create(&path, (1280, 720)).unwrap();
        recording
            .write(Codec::H264, &annexb::nals(&delta), false, 0)
            .unwrap();
        recording
            .write(Codec::H264, &annexb::nals(&key), true, 3_000)
            .unwrap();
        recording
            .write(Codec::H264, &annexb::nals(&delta), false, 6_000)
            .unwrap();
        let written = recording.finish().unwrap();
        assert_eq!(
            (written.file.as_path(), written.frames),
            (path.as_path(), 2),
            "the delta before the first keyframe is not recorded"
        );
        assert_eq!(
            written.bytes,
            std::fs::metadata(&path).unwrap().len(),
            "the summary counts every byte in the file"
        );
        assert!(
            Recording::create(&path, (1280, 720)).is_err(),
            "a recording never overwrites a file that is already there"
        );
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let has = |tag: &[u8]| bytes.windows(4).any(|window| window == tag);
        assert_eq!(&bytes[4..8], b"ftyp");
        assert!(has(b"moov") && has(b"moof") && has(b"mdat"));
        assert_eq!(
            sample(Codec::H264, &annexb::nals(&key), &PPS),
            [0, 0, 0, 3, 0x65, 0x88, 0x84],
            "parameter sets live in the init segment, not the samples"
        );
    }

    #[test]
    fn parameter_sets_sent_apart_from_the_keyframe_still_start_the_recording() {
        let path = scratch("apart");
        let mut recording = Recording::create(&path, (1280, 720)).unwrap();
        recording
            .write(Codec::H264, &annexb::nals(&unit(&[&SPS, &PPS])), false, 0)
            .unwrap();
        recording
            .write(Codec::H264, &annexb::nals(&unit(&[&[0x65, 0x88]])), true, 0)
            .unwrap();
        assert_eq!(recording.finish().unwrap().frames, 1);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_camera_without_parameter_sets_or_one_that_changes_format_stops_the_recording() {
        let path = scratch("refused");
        let mut bare = Recording::create(&path, (1280, 720)).unwrap();
        let idr = unit(&[&[0x65, 0x88]]);
        let refusals: Vec<bool> = (0..KEYFRAMES_WITHOUT_PARAMETERS)
            .map(|frame| {
                bare.write(
                    Codec::H264,
                    &annexb::nals(&idr),
                    true,
                    u64::from(frame) * 3_000,
                )
                .is_err()
            })
            .collect();
        assert_eq!(refusals, [false, false, true]);
        std::fs::remove_file(&path).unwrap();

        let mut changing = Recording::create(&path, (1280, 720)).unwrap();
        changing
            .write(
                Codec::H264,
                &annexb::nals(&unit(&[&SPS, &PPS, &[0x65, 0x88]])),
                true,
                0,
            )
            .unwrap();
        let wider = unit(&[&[0x67, 0x64, 0x00, 0x28, 0xac], &PPS, &[0x65, 0x88]]);
        assert_eq!(
            changing
                .write(Codec::H264, &annexb::nals(&wider), true, 3_000)
                .map_err(|error| error.to_string()),
            Err(FORMAT_CHANGED.to_string())
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn frames_are_placed_by_the_camera_clock_and_a_jump_continues_from_the_last_frame() {
        let (clock, first) = Clock::starting(1_000_000).placed(1_000_000);
        let (clock, burst) = clock.placed(1_000_000 + 1_500);
        let (clock, after_restart) = clock.placed(42);
        let (_, next) = clock.placed(42 + 3_000);
        assert_eq!(
            [first, burst, after_restart, next],
            [0, 1_500, 1_500 + FRAME_STEP, 1_500 + 2 * FRAME_STEP],
            "a burst after a stall keeps the camera's spacing, and a new session's clock is joined on"
        );
    }

    #[test]
    fn b_frames_keep_their_presentation_times_and_the_recording_its_length() {
        let shown_in_decode_order = [0, 3, 1, 2, 6, 4, 5, 9, 7, 8];
        let placed: Vec<i64> = shown_in_decode_order
            .iter()
            .scan(Clock::starting(500_000), |clock, frame| {
                let (next, at) = clock.placed(500_000 + frame * 3_000);
                *clock = next;
                Some(at)
            })
            .collect();
        assert_eq!(
            placed,
            shown_in_decode_order.map(|frame| i64::try_from(frame).unwrap() * 3_000),
            "a frame shown before the one decoded ahead of it is not a jump"
        );
        let path = scratch("bframes");
        let key = unit(&[&SPS, &PPS, &[0x65, 0x88]]);
        let delta = unit(&[&[0x41, 0x9a]]);
        let mut recording = Recording::create(&path, (1280, 720)).unwrap();
        (0..10u64)
            .flat_map(|group| shown_in_decode_order.map(|frame| group * 10 + frame))
            .try_for_each(|frame| {
                let nals = if frame % 30 == 0 { &key } else { &delta };
                recording.write(
                    Codec::H264,
                    &annexb::nals(nals),
                    frame % 30 == 0,
                    500_000 + frame * 3_000,
                )
            })
            .unwrap();
        let written = recording.finish().unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(written.frames, 100);
        assert!(
            (written.seconds - 99.0 * 3_000.0 / 90_000.0).abs() < 0.01,
            "the camera's 3.3 s stay 3.3 s rather than growing with every reordered frame ({}s)",
            written.seconds
        );
    }

    #[test]
    fn a_stall_stays_inside_a_fragment_so_the_file_keeps_its_length() {
        let path = scratch("stall");
        let key = unit(&[&SPS, &PPS, &[0x65, 0x88]]);
        let delta = unit(&[&[0x41, 0x9a]]);
        let moofs = |path: &Path| {
            std::fs::read(path)
                .unwrap()
                .windows(4)
                .filter(|window| *window == b"moof")
                .count()
        };
        let mut recording = Recording::create(&path, (1280, 720)).unwrap();
        let resumed = 40 * 3_000 + 4 * 90_000;
        let decoded = REORDER_DEPTH as u64;
        let frames: Vec<(bool, u64)> = std::iter::once((true, 0))
            .chain((1..40).map(|frame| (false, frame * 3_000)))
            .chain([(true, resumed)])
            .chain((1..=decoded).map(|frame| (false, resumed + frame * 3_000)))
            .collect();
        frames.iter().for_each(|&(is_key, at)| {
            let nals = if is_key { &key } else { &delta };
            recording
                .write(Codec::H264, &annexb::nals(nals), is_key, at)
                .unwrap();
        });
        assert_eq!(
            moofs(&path),
            0,
            "the keyframe after a 4 s stall does not start a fragment, since muxide would close the gap there"
        );
        (decoded + 1..=30).for_each(|frame| {
            recording
                .write(
                    Codec::H264,
                    &annexb::nals(&delta),
                    false,
                    resumed + frame * 3_000,
                )
                .unwrap();
        });
        recording
            .write(Codec::H264, &annexb::nals(&key), true, resumed + 31 * 3_000)
            .unwrap();
        (32..32 + decoded).for_each(|frame| {
            recording
                .write(
                    Codec::H264,
                    &annexb::nals(&delta),
                    false,
                    resumed + frame * 3_000,
                )
                .unwrap();
        });
        assert_eq!(
            moofs(&path),
            1,
            "the next keyframe a second on flushes as usual"
        );
        let written = recording.finish().unwrap();
        assert!(
            written.seconds > 5.0,
            "the stall is kept in the recording's length"
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_camera_with_several_picture_parameter_sets_keeps_recording() {
        let path = scratch("pps");
        let second_pps = [0x68, 0x53, 0x88, 0x80];
        assert_eq!(
            (pps_id(Codec::H264, &PPS), pps_id(Codec::H264, &second_pps)),
            (Some(0), Some(1))
        );
        let mut recording = Recording::create(&path, (1280, 720)).unwrap();
        recording
            .write(
                Codec::H264,
                &annexb::nals(&unit(&[&SPS, &PPS, &[0x65, 0x88]])),
                true,
                0,
            )
            .unwrap();
        let other = unit(&[&second_pps, &[0x41, 0x9a]]);
        assert!(
            recording
                .write(Codec::H264, &annexb::nals(&other), false, 3_000)
                .is_ok(),
            "a new PPS id is not a format change"
        );
        assert_eq!(
            sample(Codec::H264, &annexb::nals(&other), &PPS)[..8],
            [0, 0, 0, 4, 0x68, 0x53, 0x88, 0x80],
            "a PPS the file header does not hold travels in the sample"
        );
        recording.finish().unwrap();
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_recording_that_never_got_a_frame_leaves_no_file() {
        let path = scratch("empty");
        let recording = Recording::create(&path, (1280, 720)).unwrap();
        assert_eq!(recording.finish().unwrap().frames, 0);
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_inside_the_video_folder_cannot_lead_a_recording_out_of_it() {
        let base = std::env::temp_dir().join(format!("aircast-video-link-{}", std::process::id()));
        let (folder, outside) = (base.join("Video"), base.join("outside"));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, folder.join("link")).unwrap();
        assert!(inside_folder(&folder, &folder.join("link/new/clip.mp4")).is_err());
        assert!(
            !outside.join("new").exists(),
            "no folder is made outside before the request is refused"
        );
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_recording_is_written_only_inside_the_video_folder() {
        let folder =
            std::env::temp_dir().join(format!("aircast-video-folder-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let wanted = folder.join("2026-11-15_12.00.00.mp4");
        assert_eq!(inside_folder(&folder, &wanted).unwrap(), wanted);
        assert!(inside_folder(&folder, &folder.join("../escape.mp4")).is_err());
        assert!(inside_folder(&folder, Path::new("/etc/aircast.mp4")).is_err());
        assert!(inside_folder(&folder, Path::new("relative.mp4")).is_err());
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
