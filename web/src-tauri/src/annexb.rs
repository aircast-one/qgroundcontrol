#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    H264,
    H265,
}

pub const START: [u8; 4] = [0, 0, 0, 1];

pub fn nals(stream: &[u8]) -> Vec<&[u8]> {
    let starts: Vec<usize> = stream
        .windows(3)
        .enumerate()
        .filter(|(_, window)| *window == [0, 0, 1])
        .map(|(at, _)| at + 3)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(index, &start)| {
            let end = starts.get(index + 1).map_or(stream.len(), |next| next - 3);
            let nal = &stream[start..end.max(start)];
            &nal[..nal
                .iter()
                .rposition(|byte| *byte != 0)
                .map_or(0, |last| last + 1)]
        })
        .filter(|nal| !nal.is_empty())
        .collect()
}

pub fn nal_type(codec: Codec, nal: &[u8]) -> u8 {
    match codec {
        Codec::H264 => nal[0] & 0x1f,
        Codec::H265 => (nal[0] >> 1) & 0x3f,
    }
}

pub fn is_key(codec: Codec, nal: &[u8]) -> bool {
    match codec {
        Codec::H264 => nal_type(codec, nal) == 5,
        Codec::H265 => (16..=21).contains(&nal_type(codec, nal)),
    }
}

fn is_sps(codec: Codec, nal: &[u8]) -> bool {
    nal_type(codec, nal)
        == match codec {
            Codec::H264 => 7,
            Codec::H265 => 33,
        }
}

fn unescaped(nal: &[u8]) -> Vec<u8> {
    nal.iter()
        .fold(Vec::with_capacity(nal.len()), |kept, &byte| {
            match (byte, kept.as_slice()) {
                (3, [.., 0, 0]) => kept,
                _ => [kept, vec![byte]].concat(),
            }
        })
}

fn codec_string(codec: Codec, sps: &[u8]) -> Option<String> {
    match codec {
        Codec::H264 => {
            (sps.len() >= 4).then(|| format!("avc1.{:02x}{:02x}{:02x}", sps[1], sps[2], sps[3]))
        }
        Codec::H265 => {
            let rbsp = unescaped(&sps[..sps.len().min(32)]);
            let profile = *rbsp.get(3)?;
            let compatibility = u32::from_be_bytes(rbsp.get(4..8)?.try_into().ok()?).reverse_bits();
            let constraints = rbsp.get(8..14)?;
            let used = constraints
                .iter()
                .rposition(|byte| *byte != 0)
                .map_or(0, |last| last + 1);
            Some(format!(
                "hev1.{}{}.{compatibility:X}.{}{}{}",
                ["", "A", "B", "C"][usize::from(profile >> 6)],
                profile & 0x1f,
                if profile & 0x20 == 0 { 'L' } else { 'H' },
                rbsp.get(14)?,
                constraints[..used]
                    .iter()
                    .map(|byte| format!(".{byte:X}"))
                    .collect::<String>(),
            ))
        }
    }
}

pub fn codec_of(codec: Codec, unit: &[u8]) -> Option<String> {
    nals(unit)
        .into_iter()
        .find(|nal| is_sps(codec, nal))
        .and_then(|sps| codec_string(codec, sps))
}

pub fn keyed(codec: Codec, unit: &[u8]) -> bool {
    nals(unit).into_iter().any(|nal| is_key(codec, nal))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_name_their_codec_and_keyframes_as_webcodecs_expects() {
        let h264 = [
            &START[..],
            &[0x67, 0x42, 0xc0, 0x1f, 0xda],
            &[0, 0, 1, 0x68, 0xce],
            &START,
            &[0x65, 0x88, 0, 0],
        ]
        .concat();
        assert_eq!(
            nals(&h264),
            [
                &[0x67, 0x42, 0xc0, 0x1f, 0xda][..],
                &[0x68, 0xce],
                &[0x65, 0x88]
            ]
        );
        assert_eq!(codec_of(Codec::H264, &h264).as_deref(), Some("avc1.42c01f"));
        assert!(keyed(Codec::H264, &h264));
        assert!(!keyed(Codec::H264, &[&START[..], &[0x41, 0x9a]].concat()));

        let sps = [
            0x42, 0x01, 0x01, 0x01, 0x60, 0, 0, 0, 0x90, 0, 0, 3, 0, 0, 0, 0x5d, 0xa0,
        ];
        let h265 = [&START[..], &sps, &START, &[0x26, 0x01, 0xaf]].concat();
        assert_eq!(
            codec_of(Codec::H265, &h265).as_deref(),
            Some("hev1.1.6.L93.90")
        );
        assert!(keyed(Codec::H265, &h265));
    }
}
