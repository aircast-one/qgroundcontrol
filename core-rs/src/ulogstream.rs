pub const SEQUENCE_SIZE: i32 = 1 << 15;
pub const ULOG_MESSAGE_HEADER: usize = 3;
pub const ULOG_FILE_HEADER: usize = 16;
pub const MAX_DROPOUT_UNITS: i32 = 25;
pub const FIRST_MESSAGE_NONE: u8 = 255;

#[derive(Debug, Default)]
pub struct Processor {
    sequence: Option<u16>,
    drops: u64,
    got_header: bool,
    pending: Vec<u8>,
    output: Vec<u8>,
    written: u64,
}

impl Processor {
    pub fn drops(&self) -> u64 {
        self.drops
    }

    pub fn written(&self) -> u64 {
        self.written
    }

    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.output)
    }

    fn check_sequence(&mut self, seq: u16) -> Option<i32> {
        let Some(last) = self.sequence else {
            self.sequence = Some(seq);
            return Some(0);
        };
        let (last, seq_i) = (last as i32, seq as i32);
        if last == seq_i {
            return None;
        }
        if seq_i > last {
            if seq_i - last > SEQUENCE_SIZE {
                return None;
            }
            let drops = seq_i - last - 1;
            self.drops += drops as u64;
            self.sequence = Some(seq);
            return Some(drops);
        }
        if last - seq_i > SEQUENCE_SIZE {
            let drops = (1 << 16) - last - 1 + seq_i;
            self.drops += drops as u64;
            self.sequence = Some(seq);
            return Some(drops);
        }
        None
    }

    fn write(&mut self, bytes: &[u8]) {
        self.output.extend_from_slice(bytes);
        self.written += bytes.len() as u64;
    }

    fn write_whole_messages(&mut self, mut data: Vec<u8>) -> Vec<u8> {
        while data.len() > 2 {
            let length = data[0] as usize + data[1] as usize * 256 + ULOG_MESSAGE_HEADER;
            if length > data.len() {
                break;
            }
            self.write(&data[..length]);
            data.drain(..length);
        }
        data
    }

    pub fn process(&mut self, sequence: u16, first_message: u8, input: &[u8]) -> bool {
        let Some(mut drops) = self.check_sequence(sequence) else { return true };
        let mut data = input.to_vec();
        let mut first_message = first_message;
        if !self.got_header {
            if data.len() < ULOG_FILE_HEADER {
                return false;
            }
            self.write(&data[..ULOG_FILE_HEADER]);
            data.drain(..ULOG_FILE_HEADER);
            self.got_header = true;
        }
        if drops > 0 {
            drops = drops.min(MAX_DROPOUT_UNITS);
            let duration = (drops as u8).wrapping_mul(10);
            self.write(&[2, 0, 79, duration, 0]);
            let pending = std::mem::take(&mut self.pending);
            self.write_whole_messages(pending);
            self.pending.clear();
            if first_message == FIRST_MESSAGE_NONE {
                return true;
            }
            if first_message > 0 {
                data.drain(..(first_message as usize).min(data.len()));
                first_message = 0;
            }
        }
        if first_message == FIRST_MESSAGE_NONE && !self.pending.is_empty() {
            self.pending.extend_from_slice(&data);
            return true;
        }
        if !self.pending.is_empty() {
            let pending = std::mem::take(&mut self.pending);
            self.write(&pending);
            if first_message > 0 {
                let head = (first_message as usize).min(data.len());
                self.write(&data[..head]);
            }
        }
        if first_message > 0 {
            data.drain(..(first_message as usize).min(data.len()));
        }
        self.pending = self.write_whole_messages(data);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(kind: u8, body: &[u8]) -> Vec<u8> {
        let len = body.len() as u16;
        let mut out = vec![len as u8, (len >> 8) as u8, kind];
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn a_stream_with_a_message_split_across_packets_is_reassembled() {
        let mut processor = Processor::default();
        let header = [0x55u8; 16];
        let a = message(b'I', b"hello");
        let b = message(b'D', &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut first: Vec<u8> = header.to_vec();
        first.extend_from_slice(&a);
        first.extend_from_slice(&b[..4]);
        assert!(processor.process(0, 0, &first));
        assert_eq!(processor.take_output(), [header.to_vec(), a.clone()].concat());
        let mut second = b[4..].to_vec();
        let c = message(b'L', b"z");
        second.extend_from_slice(&c);
        assert!(processor.process(1, (b.len() - 4) as u8, &second));
        assert_eq!(processor.take_output(), [b.clone(), c.clone()].concat());
        assert_eq!((processor.drops(), processor.written()), (0, (16 + a.len() + b.len() + c.len()) as u64));
        assert!(!Processor::default().process(0, 0, &[1, 2, 3]));
    }

    #[test]
    fn drops_write_a_dropout_marker_and_duplicates_and_reorders_are_ignored() {
        let mut processor = Processor::default();
        let mut first = vec![0u8; 16];
        first.extend_from_slice(&message(b'I', b"x"));
        processor.process(10, 0, &first);
        processor.take_output();
        assert!(processor.process(10, 0, &[9, 9, 9]));
        assert!(processor.take_output().is_empty(), "a duplicate sequence writes nothing");
        assert!(processor.process(9, 0, &[9, 9, 9]));
        assert!(processor.take_output().is_empty(), "a reordered packet writes nothing");
        let next = message(b'D', b"after gap");
        assert!(processor.process(14, 0, &next));
        let out = processor.take_output();
        assert_eq!(&out[..5], &[2, 0, 79, 30, 0]);
        assert_eq!(&out[5..], next.as_slice());
        assert_eq!(processor.drops(), 3);
        let mut wrapped = Processor::default();
        let mut start = vec![0u8; 16];
        start.extend_from_slice(&message(b'I', b"y"));
        wrapped.process(65534, 0, &start);
        wrapped.take_output();
        assert!(wrapped.process(1, 0, &message(b'D', b"w")));
        assert_eq!(wrapped.drops(), 2);
        assert_eq!(&wrapped.take_output()[..5], &[2, 0, 79, 20, 0]);
        let mut capped = Processor::default();
        let mut s = vec![0u8; 16];
        s.extend_from_slice(&message(b'I', b"c"));
        capped.process(0, 0, &s);
        capped.take_output();
        capped.process(100, 0, &message(b'D', b"far"));
        assert_eq!(capped.take_output()[3], 250);
    }
}
