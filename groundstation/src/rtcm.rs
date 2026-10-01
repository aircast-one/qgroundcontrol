use mavlink::dialects::ardupilotmega::GPS_RTCM_DATA_DATA;

pub const FRAGMENT_BYTES: usize = 180;
const MAX_FRAGMENTS: usize = 4;
const MAX_ASSEMBLED_BYTES: usize = FRAGMENT_BYTES * MAX_FRAGMENTS;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Fragmenter {
    sequence_id: u8,
}

fn flags(fragmented: bool, fragment_id: usize, sequence_id: u8) -> u8 {
    let sequence = (sequence_id & 0x1F) << 3;
    match fragmented {
        true => sequence | 0x01 | (((fragment_id as u8) & 0x03) << 1),
        false => sequence,
    }
}

fn message(flags: u8, chunk: &[u8]) -> GPS_RTCM_DATA_DATA {
    let mut data = [0u8; FRAGMENT_BYTES];
    data[..chunk.len()].copy_from_slice(chunk);
    GPS_RTCM_DATA_DATA { flags, len: chunk.len() as u8, data }
}

impl Fragmenter {
    fn next(&mut self) -> u8 {
        let sequence = self.sequence_id;
        self.sequence_id = self.sequence_id.wrapping_add(1);
        sequence
    }

    pub fn fragments(&mut self, rtcm: &[u8]) -> Vec<GPS_RTCM_DATA_DATA> {
        match rtcm.len() {
            0 => Vec::new(),
            n if n > MAX_ASSEMBLED_BYTES => rtcm.chunks(FRAGMENT_BYTES).map(|chunk| message(flags(false, 0, self.next()), chunk)).collect(),
            n if n <= FRAGMENT_BYTES => vec![message(flags(false, 0, self.next()), rtcm)],
            n => {
                let sequence = self.next();
                let chunks: Vec<&[u8]> = rtcm.chunks(FRAGMENT_BYTES).collect();
                let terminator = (n % FRAGMENT_BYTES == 0 && chunks.len() < MAX_FRAGMENTS).then_some(&[][..]);
                chunks.into_iter().chain(terminator).enumerate().map(|(fragment_id, chunk)| message(flags(true, fragment_id, sequence), chunk)).collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_that_fits_goes_whole_with_only_the_sequence_in_the_flags() {
        let mut fragmenter = Fragmenter::default();
        let first = fragmenter.fragments(&[7u8; 100]);
        assert_eq!(first.len(), 1);
        assert_eq!((first[0].flags, first[0].len), (0, 100));
        assert_eq!(&first[0].data[..100], &[7u8; 100]);
        assert!(first[0].data[100..].iter().all(|b| *b == 0));
        assert_eq!(fragmenter.fragments(&[1u8; 180]).iter().map(|p| (p.flags, p.len)).collect::<Vec<_>>(), [(1 << 3, 180)], "exactly one fragment's worth is not fragmented");
        assert!(fragmenter.fragments(&[]).is_empty());
    }

    #[test]
    fn a_long_message_is_cut_into_numbered_fragments_sharing_one_sequence() {
        let mut fragmenter = Fragmenter::default();
        let payload: Vec<u8> = (0..400u16).map(|i| i as u8).collect();
        let parts = fragmenter.fragments(&payload);
        assert_eq!(parts.iter().map(|p| p.len as usize).collect::<Vec<_>>(), vec![180, 180, 40]);
        assert_eq!(parts.iter().map(|p| p.flags).collect::<Vec<_>>(), vec![0x01, 0x03, 0x05]);
        let joined: Vec<u8> = parts.iter().flat_map(|p| p.data[..p.len as usize].to_vec()).collect();
        assert_eq!(joined, payload);
    }

    #[test]
    fn an_exact_multiple_ends_with_an_empty_terminator_and_an_oversized_one_goes_unfragmented() {
        let mut fragmenter = Fragmenter::default();
        let exact = fragmenter.fragments(&[0u8; 360]);
        assert_eq!(exact.iter().map(|p| (p.flags, p.len)).collect::<Vec<_>>(), [(0x01, 180), (0x03, 180), (0x05, 0)]);
        let full = fragmenter.fragments(&[0u8; 720]);
        assert_eq!(full.len(), 4, "four fragments already fill the reassembly buffer, so no terminator follows");
        let oversized = fragmenter.fragments(&[0u8; 721]);
        assert_eq!(oversized.iter().map(|p| p.flags).collect::<Vec<_>>(), [2 << 3, 3 << 3, 4 << 3, 5 << 3, 6 << 3], "each piece carries its own sequence");
    }

    #[test]
    fn the_sequence_wraps_inside_its_five_bits() {
        let mut fragmenter = Fragmenter::default();
        let flags: Vec<u8> = (0..34).map(|_| fragmenter.fragments(&[0u8; 1])[0].flags).collect();
        assert_eq!((flags[31], flags[32], flags[33]), (31 << 3, 0, 1 << 3));
    }
}
