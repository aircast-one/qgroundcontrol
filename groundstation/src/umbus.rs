pub const UMBUS_SYNC: u8 = 0xA6;
pub const UMBUS_GIMBALS: usize = 4;
pub const UMBUS_CHANNELS: usize = 33;
pub const CHANNEL_CENTER: u16 = 0x8000;
pub const MCU_POLL: [u8; 14] = [
    0xA6, 0x0E, 0x10, 0x04, 0x02, 0x02, 0x06, 0x83, 0xDF, 0x00, 0x00, 0x00, 0x00, 0x2F,
];
pub const APP_HEARTBEAT: [u8; 8] = [0xA6, 0x08, 0x35, 0x04, 0x05, 0x01, 0x80, 0x84];
pub const APP_KEEPALIVE: [u8; 7] = [0xA6, 0x07, 0x2B, 0x04, 0xFF, 0x01, 0xF4];

const CHANNEL_DATA: u8 = 0x57;
const IDLE_CHANNEL_DATA: u8 = 0x77;
const HEARTBEAT_TYPE: u8 = 0x08;
const APP_HEARTBEAT_HEADER: u8 = 0x35;
const MCU_HEARTBEAT_LEN: usize = 7;
const CHANNEL_FRAME_LEN: usize = 87;
const GIMBAL_AT: usize = 6;
const CHANNEL_AT: usize = 18;
const MAXIM_POLY: u8 = 0x8C;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sticks {
    pub gimbals: [i16; UMBUS_GIMBALS],
    pub channels: [u16; UMBUS_CHANNELS],
}

pub fn crc8(init: u8, bytes: &[u8]) -> u8 {
    bytes.iter().fold(init, |crc, byte| {
        (0..8).fold(crc ^ byte, |c, _| {
            if c & 1 == 1 {
                (c >> 1) ^ MAXIM_POLY
            } else {
                c >> 1
            }
        })
    })
}

fn crc_init(kind: u8) -> u8 {
    match kind {
        0x10 => 0x7F,
        0x15 => 0x32,
        _ => 0x00,
    }
}

enum Size {
    Wait,
    Frame(usize),
    Junk,
}

fn size(buffer: &[u8]) -> Size {
    match (buffer.get(1), buffer.get(2)) {
        (None, _) => Size::Wait,
        (Some(&HEARTBEAT_TYPE), None) => Size::Wait,
        (Some(&HEARTBEAT_TYPE), Some(&APP_HEARTBEAT_HEADER)) => Size::Frame(8),
        (Some(&HEARTBEAT_TYPE), Some(_)) => Size::Frame(MCU_HEARTBEAT_LEN),
        (Some(&CHANNEL_DATA | &IDLE_CHANNEL_DATA), _) => Size::Frame(CHANNEL_FRAME_LEN),
        (Some(0x10), _) => Size::Frame(18),
        (Some(0x15), _) => Size::Frame(21),
        (Some(0x07), _) => Size::Frame(7),
        (Some(0x0C), _) => Size::Frame(12),
        (Some(0x0E), _) => Size::Frame(14),
        (Some(_), _) => Size::Junk,
    }
}

fn intact(frame: &[u8]) -> bool {
    match (frame[1], frame.len()) {
        (HEARTBEAT_TYPE, MCU_HEARTBEAT_LEN) => true,
        (kind, len) => crc8(crc_init(kind), &frame[1..len - 1]) == frame[len - 1],
    }
}

pub fn sticks(frame: &[u8]) -> Option<Sticks> {
    let channel_data = matches!(frame.get(1), Some(&CHANNEL_DATA | &IDLE_CHANNEL_DATA))
        && frame.len() == CHANNEL_FRAME_LEN;
    channel_data.then(|| Sticks {
        gimbals: std::array::from_fn(|i| {
            i16::from_le_bytes([frame[GIMBAL_AT + 2 * i], frame[GIMBAL_AT + 2 * i + 1]])
        }),
        channels: std::array::from_fn(|i| {
            u16::from_le_bytes([frame[CHANNEL_AT + 2 * i], frame[CHANNEL_AT + 2 * i + 1]])
        }),
    })
}

#[derive(Debug, Default)]
pub struct Decoder {
    buffer: Vec<u8>,
    pub rejected: u64,
}

impl Decoder {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        self.buffer.extend_from_slice(bytes);
        std::iter::from_fn(|| self.next_frame()).collect()
    }

    fn next_frame(&mut self) -> Option<Vec<u8>> {
        loop {
            let start = self
                .buffer
                .iter()
                .position(|byte| *byte == UMBUS_SYNC)
                .unwrap_or(self.buffer.len());
            self.buffer.drain(..start);
            match size(&self.buffer) {
                Size::Wait => return None,
                Size::Frame(len) if self.buffer.len() < len => return None,
                Size::Frame(len) if intact(&self.buffer[..len]) => {
                    return Some(self.buffer.drain(..len).collect());
                }
                Size::Frame(_) | Size::Junk => {
                    self.rejected += 1;
                    self.buffer.drain(..1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
            .collect()
    }

    const IDLE_APP_CLOSED: &str = "a657100204010300ffff36fe01000cfef4010080008000800080008000800cfef401008000800080008000800080f401000000000cfe0cfe0cfe0080008000800080008000800080008000806400000000000800a401cb";
    const RC_APP_RUNNING: &str = "a657100204010300ffffb8fe0100f6ff9c0000800080008000800080008005007bfe0080008000800080008000800cfe0cfe00000cfe0000f4010080008000800080008000800080008000800cfe000000000100930115";
    const ELRS: &str = "a615c30200ea0a3aeaee1000004e20fffff7d6b07d";
    const MCU_HEARTBEAT: &str = "a6081002040300";
    const EXTENDED: &str = "a61002040600000020000000000000000086";

    #[test]
    fn the_checksum_is_crc8_maxim_with_the_seed_each_frame_type_uses() {
        assert_eq!(
            (crc8(0, &[1]), crc8(0, &[2]), crc8(0, &[0x80])),
            (0x5E, 0xBC, 0x8C),
            "the first entries of the CRC8_Table the RadioMaster library ships"
        );
        [IDLE_APP_CLOSED, RC_APP_RUNNING, ELRS, EXTENDED]
            .iter()
            .map(|frame| hex(frame))
            .for_each(|frame| {
                assert!(
                    intact(&frame),
                    "a frame captured on a real AX12 checks out: {frame:02x?}"
                )
            });
        [
            MCU_POLL.as_slice(),
            APP_HEARTBEAT.as_slice(),
            APP_KEEPALIVE.as_slice(),
        ]
        .iter()
        .for_each(|frame| {
            assert!(
                intact(frame),
                "the frames RadioMaster's app sends the MCU check out too, so the MCU accepts ours"
            )
        });
    }

    #[test]
    fn a_channel_frame_carries_four_gimbals_and_thirty_three_channels() {
        let idle = sticks(&hex(IDLE_APP_CLOSED)).unwrap();
        assert_eq!(
            idle.gimbals,
            [3, -1, -458, 1],
            "throttle (gimbal 2) does not centre, the other three rest near zero"
        );
        assert_eq!(
            (
                idle.channels[0],
                idle.channels[6],
                idle.channels[7],
                idle.channels[14],
                idle.channels[17]
            ),
            (CHANNEL_CENTER, 65036, 500, 500, 65036)
        );
        let flying = sticks(&hex(RC_APP_RUNNING)).unwrap();
        assert_eq!(flying.gimbals[2], -328);
        assert_eq!(
            flying.channels[14], 65036,
            "SA sat the other way in this capture"
        );
        assert_eq!(
            sticks(&hex(ELRS)),
            None,
            "only channel data frames carry sticks"
        );
    }

    #[test]
    fn the_stream_resyncs_past_split_reads_and_garbage() {
        let full = hex(IDLE_APP_CLOSED);
        let spliced: Vec<u8> = full[..20].iter().copied().chain(hex(ELRS)).collect();
        let stream: Vec<u8> = [
            vec![0x00, 0x13, 0xA6],
            full.clone(),
            spliced,
            hex(MCU_HEARTBEAT),
            hex(RC_APP_RUNNING),
            hex(EXTENDED),
        ]
        .concat();
        let mut decoder = Decoder::default();
        let frames: Vec<Vec<u8>> = stream
            .chunks(13)
            .flat_map(|chunk| decoder.feed(chunk))
            .collect();
        let kinds: Vec<u8> = frames.iter().map(|frame| frame[1]).collect();
        assert_eq!(
            kinds,
            vec![0x57, 0x15, 0x08, 0x57, 0x10],
            "a channel frame cut short by a telemetry frame, as the AX12 capture shows, costs that frame only"
        );
        assert!(decoder.rejected > 0);
        assert_eq!(frames.iter().filter_map(|frame| sticks(frame)).count(), 2);
    }
}
