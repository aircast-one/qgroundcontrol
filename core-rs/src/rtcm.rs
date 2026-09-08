use mavlink::dialects::ardupilotmega::GPS_RTCM_DATA_DATA;

pub const FRAGMENT_BYTES: usize = 180;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Fragmenter {
    sequence_id: u8,
}

fn message(flags: u8, chunk: &[u8]) -> GPS_RTCM_DATA_DATA {
    let mut data = [0u8; FRAGMENT_BYTES];
    data[..chunk.len()].copy_from_slice(chunk);
    GPS_RTCM_DATA_DATA { flags, len: chunk.len() as u8, data }
}

impl Fragmenter {
    pub fn fragments(&mut self, rtcm: &[u8]) -> Vec<GPS_RTCM_DATA_DATA> {
        let sequence = (self.sequence_id & 0x1F) << 3;
        self.sequence_id = self.sequence_id.wrapping_add(1);
        if rtcm.len() < FRAGMENT_BYTES {
            return vec![message(sequence, rtcm)];
        }
        rtcm.chunks(FRAGMENT_BYTES)
            .enumerate()
            .map(|(fragment_id, chunk)| message(0x01 | ((fragment_id as u8) << 1) | sequence, chunk))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_message_goes_whole_with_only_the_sequence_in_the_flags() {
        let mut fragmenter = Fragmenter::default();
        let first = fragmenter.fragments(&[7u8; 100]);
        assert_eq!(first.len(), 1);
        assert_eq!((first[0].flags, first[0].len), (0, 100));
        assert_eq!(&first[0].data[..100], &[7u8; 100]);
        assert!(first[0].data[100..].iter().all(|b| *b == 0));
        let second = fragmenter.fragments(&[1u8; 10]);
        assert_eq!(second[0].flags, 1 << 3);
    }

    #[test]
    fn a_long_message_is_cut_into_numbered_fragments() {
        let mut fragmenter = Fragmenter::default();
        let payload: Vec<u8> = (0..400u16).map(|i| i as u8).collect();
        let parts = fragmenter.fragments(&payload);
        assert_eq!(parts.iter().map(|p| p.len as usize).collect::<Vec<_>>(), vec![180, 180, 40]);
        assert_eq!(parts.iter().map(|p| p.flags).collect::<Vec<_>>(), vec![0x01, 0x03, 0x05]);
        let joined: Vec<u8> = parts.iter().flat_map(|p| p.data[..p.len as usize].to_vec()).collect();
        assert_eq!(joined, payload);
        assert_eq!(fragmenter.fragments(&[0u8; 180]).len(), 1);
        assert_eq!(fragmenter.fragments(&[0u8; 180])[0].flags, 0x01 | (2 << 3));
    }

    #[test]
    fn the_sequence_wraps_inside_its_five_bits() {
        let mut fragmenter = Fragmenter::default();
        let flags: Vec<u8> = (0..34).map(|_| fragmenter.fragments(&[0u8; 1])[0].flags).collect();
        assert_eq!((flags[31], flags[32], flags[33]), (31 << 3, 0, 1 << 3));
    }
}
