use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};

use muxide::api::{MuxerBuilder, VideoCodec};
use muxide::fragmented::FragmentedMuxer;

use crate::annexb::{self, Codec};

const TIMESCALE: f64 = 90_000.0;
const SEGMENT: Duration = Duration::from_secs(1);

pub struct Recording {
    file: File,
    size: (u32, u32),
    muxer: Option<FragmentedMuxer>,
    started: Option<Instant>,
    flushed: Instant,
}

fn parameter_set(codec: Codec, unit: &[u8], kind: u8) -> Option<Vec<u8>> {
    annexb::nals(unit)
        .into_iter()
        .find(|nal| annexb::nal_type(codec, nal) == kind)
        .map(<[u8]>::to_vec)
}

fn sample(codec: Codec, unit: &[u8]) -> Vec<u8> {
    let in_band = match codec {
        Codec::H264 => 7..=9,
        Codec::H265 => 32..=35,
    };
    annexb::nals(unit)
        .into_iter()
        .filter(|nal| !in_band.contains(&annexb::nal_type(codec, nal)))
        .flat_map(|nal| {
            [
                &u32::try_from(nal.len()).unwrap_or(0).to_be_bytes()[..],
                nal,
            ]
            .concat()
        })
        .collect()
}

fn muxer(codec: Codec, unit: &[u8], (width, height): (u32, u32)) -> Option<FragmentedMuxer> {
    let (video, sps, pps, vps) = match codec {
        Codec::H264 => (VideoCodec::H264, 7, 8, None),
        Codec::H265 => (VideoCodec::H265, 33, 34, Some(32)),
    };
    let builder = MuxerBuilder::new(std::io::sink())
        .video(video, width, height, 30.0)
        .with_sps(parameter_set(codec, unit, sps)?)
        .with_pps(parameter_set(codec, unit, pps)?);
    let builder = match vps {
        Some(kind) => builder.with_vps(parameter_set(codec, unit, kind)?),
        None => builder,
    };
    builder.new_with_fragment().ok()
}

impl Recording {
    pub fn create(path: &str, size: (u32, u32)) -> std::io::Result<Self> {
        Ok(Self {
            file: File::create(path)?,
            size,
            muxer: None,
            started: None,
            flushed: Instant::now(),
        })
    }

    pub fn write(&mut self, codec: Codec, unit: &[u8], key: bool) -> std::io::Result<()> {
        if self.muxer.is_none() && key {
            self.muxer = muxer(codec, unit, self.size);
            if let Some(started) = self.muxer.as_mut() {
                self.file.write_all(&started.init_segment())?;
            }
        }
        let Some(muxer) = self.muxer.as_mut() else {
            return Ok(());
        };
        if key && self.flushed.elapsed() >= SEGMENT {
            self.flushed = Instant::now();
            if let Some(segment) = muxer.flush_segment() {
                self.file.write_all(&segment)?;
            }
        }
        let at = (self
            .started
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_secs_f64()
            * TIMESCALE) as u64;
        muxer
            .write_video(at, at, &sample(codec, unit), key)
            .map_err(|error| std::io::Error::other(error.to_string()))
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        match self.muxer.as_mut().and_then(FragmentedMuxer::flush_segment) {
            Some(segment) => self.file.write_all(&segment),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annexb::START;

    #[test]
    fn a_recording_starts_at_a_keyframe_and_is_a_fragmented_mp4() {
        let path = std::env::temp_dir().join(format!("aircast-record-{}.mp4", std::process::id()));
        let key = [
            &START[..],
            &[0x67, 0x42, 0xc0, 0x1f, 0xda, 0x05, 0x02],
            &START,
            &[0x68, 0xce, 0x3c, 0x80],
            &START,
            &[0x65, 0x88, 0x84],
        ]
        .concat();
        let delta = [&START[..], &[0x41, 0x9a, 0x02]].concat();
        let mut recording = Recording::create(path.to_str().unwrap(), (1280, 720)).unwrap();
        recording.write(Codec::H264, &delta, false).unwrap();
        recording.write(Codec::H264, &key, true).unwrap();
        recording.write(Codec::H264, &delta, false).unwrap();
        recording.finish().unwrap();
        let written = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let has = |tag: &[u8]| written.windows(4).any(|window| window == tag);
        assert_eq!(&written[4..8], b"ftyp");
        assert!(has(b"moov") && has(b"moof") && has(b"mdat"));
        assert_eq!(
            sample(Codec::H264, &key),
            [0, 0, 0, 3, 0x65, 0x88, 0x84],
            "parameter sets live in the init segment, not the samples"
        );
    }
}
