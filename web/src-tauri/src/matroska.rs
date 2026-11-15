use mkv_element::ClusterBlock;
use mkv_element::io::blocking_impl::WriteTo;
use mkv_element::prelude::*;

use crate::annexb::Codec;

const TICKS_PER_MILLISECOND: i64 = 90;
const MILLISECOND: u64 = 1_000_000;
const CLUSTER_SPAN: i64 = 1_000;
const LONGEST_RELATIVE: i64 = i16::MAX as i64;
const TRACK: u8 = 0x81;
const KEYFRAME: u8 = 0x80;
const VIDEO_TRACK: u64 = 1;
const APP: &str = "Aircast";
const DURATION: [u8; 3] = [0x44, 0x89, 0x88];
const RESERVED_FOR_DURATION: Void = Void { size: 9 };

struct Open {
    start: i64,
    blocks: Vec<ClusterBlock>,
}

pub struct Matroska {
    cluster: Option<Open>,
    duration_at: usize,
}

fn failed(error: mkv_element::Error) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

fn encoded(element: &impl WriteTo) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    element.write_to(&mut bytes).map_err(failed)?;
    Ok(bytes)
}

fn codec_id(codec: Codec) -> &'static str {
    match codec {
        Codec::H264 => "V_MPEG4/ISO/AVC",
        Codec::H265 => "V_MPEGH/ISO/HEVC",
    }
}

pub fn duration(milliseconds: f64) -> Vec<u8> {
    [&DURATION[..], &milliseconds.to_be_bytes()].concat()
}

impl Matroska {
    pub fn start(
        codec: Codec,
        configuration: Vec<u8>,
        (width, height): (u32, u32),
    ) -> std::io::Result<(Self, Vec<u8>)> {
        let ebml = Ebml {
            ebml_max_id_length: EbmlMaxIdLength(4),
            ebml_max_size_length: EbmlMaxSizeLength(8),
            doc_type: Some(DocType("matroska".to_string())),
            doc_type_version: Some(DocTypeVersion(4)),
            doc_type_read_version: Some(DocTypeReadVersion(2)),
            ..Default::default()
        };
        let segment = Header {
            id: Segment::ID,
            size: VInt64::new_unknown(),
        };
        let info = Info {
            void: Some(RESERVED_FOR_DURATION),
            timestamp_scale: TimestampScale(MILLISECOND),
            muxing_app: MuxingApp(APP.to_string()),
            writing_app: WritingApp(APP.to_string()),
            ..Default::default()
        };
        let tracks = Tracks {
            track_entry: vec![TrackEntry {
                track_number: TrackNumber(VIDEO_TRACK),
                track_uid: TrackUid(VIDEO_TRACK),
                track_type: TrackType(1),
                codec_id: CodecId(codec_id(codec).to_string()),
                codec_private: Some(CodecPrivate(configuration.into())),
                video: Some(Video {
                    pixel_width: PixelWidth(width.into()),
                    pixel_height: PixelHeight(height.into()),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        let header = [
            encoded(&ebml)?,
            encoded(&segment)?,
            encoded(&info)?,
            encoded(&tracks)?,
        ]
        .concat();
        let reserved = encoded(&RESERVED_FOR_DURATION)?;
        let duration_at = header
            .windows(reserved.len())
            .position(|window| window == reserved)
            .ok_or_else(|| {
                std::io::Error::other("The recording header has no room for its length.")
            })?;
        Ok((
            Self {
                cluster: None,
                duration_at,
            },
            header,
        ))
    }

    pub fn duration_at(&self) -> usize {
        self.duration_at
    }

    pub fn frame(&mut self, at: i64, key: bool, sample: &[u8]) -> std::io::Result<Vec<u8>> {
        let shown = at.max(0) / TICKS_PER_MILLISECOND;
        let starts = self.cluster.as_ref().is_none_or(|open| {
            (key && shown - open.start >= CLUSTER_SPAN)
                || (shown - open.start).abs() > LONGEST_RELATIVE
        });
        let closed = match starts {
            true => self.cluster.replace(Open {
                start: shown,
                blocks: Vec::new(),
            }),
            false => None,
        };
        let open = self
            .cluster
            .as_mut()
            .ok_or_else(|| std::io::Error::other("The recording has no cluster open."))?;
        let relative = i16::try_from(shown - open.start).unwrap_or_default();
        let flags = if key { KEYFRAME } else { 0 };
        let block = [&[TRACK][..], &relative.to_be_bytes(), &[flags], sample].concat();
        open.blocks
            .push(ClusterBlock::Simple(SimpleBlock(block.into())));
        closed.map_or(Ok(Vec::new()), written)
    }

    pub fn ended(&mut self) -> std::io::Result<Vec<u8>> {
        self.cluster.take().map_or(Ok(Vec::new()), written)
    }
}

fn written(open: Open) -> std::io::Result<Vec<u8>> {
    encoded(&Cluster {
        timestamp: Timestamp(u64::try_from(open.start).unwrap_or_default()),
        blocks: open.blocks,
        ..Default::default()
    })
}
