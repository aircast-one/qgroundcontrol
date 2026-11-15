use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use muxide::api::{MuxerBuilder, VideoCodec};
use muxide::fragmented::FragmentedMuxer;

use crate::annexb::{self, Codec};

const TIMESCALE: f64 = 90_000.0;
const SEGMENT: Duration = Duration::from_secs(1);
const FRAME_RATE_UNUSED_BY_FRAGMENTS: f64 = 30.0;

pub struct Recording {
    file: File,
    size: (u32, u32),
    muxer: Option<FragmentedMuxer>,
    started: Option<Instant>,
    flushed: Instant,
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
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let canonical = |path: &Path| path.canonicalize().map_err(|error| error.to_string());
    canonical(parent)?
        .starts_with(canonical(folder)?)
        .then(|| file.to_path_buf())
        .ok_or_else(|| format!("{} leaves the video folder.", file.display()))
}

fn parameter_set(codec: Codec, nals: &[&[u8]], kind: u8) -> Option<Vec<u8>> {
    nals.iter()
        .find(|nal| annexb::nal_type(codec, nal) == kind)
        .map(|nal| nal.to_vec())
}

fn sample(codec: Codec, nals: &[&[u8]]) -> Vec<u8> {
    let in_band = match codec {
        Codec::H264 => 7..=9,
        Codec::H265 => 32..=35,
    };
    nals.iter()
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

fn muxer(codec: Codec, nals: &[&[u8]], (width, height): (u32, u32)) -> Option<FragmentedMuxer> {
    let (video, sps, pps, vps) = match codec {
        Codec::H264 => (VideoCodec::H264, 7, 8, None),
        Codec::H265 => (VideoCodec::H265, 33, 34, Some(32)),
    };
    let builder = MuxerBuilder::new(std::io::sink())
        .video(video, width, height, FRAME_RATE_UNUSED_BY_FRAGMENTS)
        .with_sps(parameter_set(codec, nals, sps)?)
        .with_pps(parameter_set(codec, nals, pps)?);
    let builder = match vps {
        Some(kind) => builder.with_vps(parameter_set(codec, nals, kind)?),
        None => builder,
    };
    builder.new_with_fragment().ok()
}

impl Recording {
    pub fn create(path: &Path, size: (u32, u32)) -> std::io::Result<Self> {
        Ok(Self {
            file: OpenOptions::new().write(true).create_new(true).open(path)?,
            size,
            muxer: None,
            started: None,
            flushed: Instant::now(),
        })
    }

    pub fn write(&mut self, codec: Codec, nals: &[&[u8]], key: bool) -> std::io::Result<()> {
        if self.muxer.is_none() && key {
            self.muxer = muxer(codec, nals, self.size);
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
            .write_video(at, at, &sample(codec, nals), key)
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
        let mut recording = Recording::create(&path, (1280, 720)).unwrap();
        recording
            .write(Codec::H264, &annexb::nals(&delta), false)
            .unwrap();
        recording
            .write(Codec::H264, &annexb::nals(&key), true)
            .unwrap();
        recording
            .write(Codec::H264, &annexb::nals(&delta), false)
            .unwrap();
        recording.finish().unwrap();
        assert!(
            Recording::create(&path, (1280, 720)).is_err(),
            "a recording never overwrites a file that is already there"
        );
        let written = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let has = |tag: &[u8]| written.windows(4).any(|window| window == tag);
        assert_eq!(&written[4..8], b"ftyp");
        assert!(has(b"moov") && has(b"moof") && has(b"mdat"));
        assert_eq!(
            sample(Codec::H264, &annexb::nals(&key)),
            [0, 0, 0, 3, 0x65, 0x88, 0x84],
            "parameter sets live in the init segment, not the samples"
        );
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
