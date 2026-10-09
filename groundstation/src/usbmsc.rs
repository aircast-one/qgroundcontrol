use std::io::{self, Read, Seek, SeekFrom, Write};
use std::time::Duration;

use crate::cardwrite::BlockDevice;

const CBW_SIGNATURE: u32 = 0x4342_5355;
const CSW_SIGNATURE: u32 = 0x5342_5355;
const CBW_LEN: usize = 31;
const CSW_LEN: usize = 13;
const DIRECTION_IN: u8 = 0x80;
const TEST_UNIT_READY: u8 = 0x00;
const REQUEST_SENSE: u8 = 0x03;
const INQUIRY: u8 = 0x12;
const READ_CAPACITY_10: u8 = 0x25;
const READ_10: u8 = 0x28;
const WRITE_10: u8 = 0x2a;
const SYNCHRONIZE_CACHE_10: u8 = 0x35;
const SENSE_LEN: usize = 18;
const INQUIRY_LEN: usize = 36;
const MAX_TRANSFER: usize = 512 * 1024;
const READY_ATTEMPTS: usize = 20;
#[cfg(not(test))]
const READY_BACKOFF: Duration = Duration::from_millis(250);
#[cfg(test)]
const READY_BACKOFF: Duration = Duration::ZERO;
const NO_MEDIUM: u8 = 0x3a;

pub trait Bulk {
    fn send(&mut self, data: &[u8]) -> io::Result<()>;
    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn clear_halt(&mut self, inbound: bool) -> io::Result<()>;
}

enum Data<'a> {
    None,
    In(&'a mut [u8]),
    Out(&'a [u8]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub vendor: String,
    pub product: String,
}

pub struct UsbDisk<B: Bulk> {
    bulk: B,
    tag: u32,
    block_size: u32,
    blocks: u64,
    pos: u64,
    identity: Identity,
}

impl<B: Bulk> UsbDisk<B> {
    pub fn open(bulk: B) -> io::Result<Self> {
        let mut disk = Self { bulk, tag: 0, block_size: 512, blocks: 0, pos: 0, identity: Identity { vendor: String::new(), product: String::new() } };
        disk.identity = disk.inquiry()?;
        disk.wait_until_ready()?;
        let (blocks, block_size) = disk.read_capacity()?;
        disk.blocks = blocks;
        disk.block_size = block_size;
        Ok(disk)
    }

    pub fn capacity(&self) -> u64 {
        self.blocks * u64::from(self.block_size)
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn into_bulk(self) -> B {
        self.bulk
    }

    fn inquiry(&mut self) -> io::Result<Identity> {
        let mut answer = [0u8; INQUIRY_LEN];
        self.command(&[INQUIRY, 0, 0, 0, INQUIRY_LEN as u8, 0], Data::In(&mut answer))?;
        let text = |range: std::ops::Range<usize>| String::from_utf8_lossy(&answer[range]).trim().to_string();
        Ok(Identity { vendor: text(8..16), product: text(16..32) })
    }

    fn wait_until_ready(&mut self) -> io::Result<()> {
        (0..READY_ATTEMPTS)
            .find_map(|attempt| match self.command(&[TEST_UNIT_READY, 0, 0, 0, 0, 0], Data::None) {
                Ok(()) => Some(Ok(())),
                Err(_) => match self.request_sense() {
                    Ok(NO_MEDIUM) => Some(Err(io::Error::new(io::ErrorKind::NotFound, "no card in the reader"))),
                    _ if attempt + 1 == READY_ATTEMPTS => Some(Err(io::Error::new(io::ErrorKind::TimedOut, "the card reader never became ready"))),
                    _ => {
                        std::thread::sleep(READY_BACKOFF);
                        None
                    }
                },
            })
            .unwrap_or_else(|| Err(io::Error::new(io::ErrorKind::TimedOut, "the card reader never became ready")))
    }

    fn request_sense(&mut self) -> io::Result<u8> {
        let mut answer = [0u8; SENSE_LEN];
        self.command(&[REQUEST_SENSE, 0, 0, 0, SENSE_LEN as u8, 0], Data::In(&mut answer))?;
        Ok(answer[12])
    }

    fn read_capacity(&mut self) -> io::Result<(u64, u32)> {
        let mut answer = [0u8; 8];
        self.command(&[READ_CAPACITY_10, 0, 0, 0, 0, 0, 0, 0, 0, 0], Data::In(&mut answer))?;
        let last_lba = u32::from_be_bytes([answer[0], answer[1], answer[2], answer[3]]);
        let block_size = u32::from_be_bytes([answer[4], answer[5], answer[6], answer[7]]);
        match (last_lba, block_size) {
            (u32::MAX, _) => Err(io::Error::new(io::ErrorKind::Unsupported, "cards larger than 2 TB are not supported")),
            (_, size) if size == 0 || crate::cardwrite::ALIGN % u64::from(size) != 0 => Err(io::Error::new(io::ErrorKind::Unsupported, format!("unsupported block size {size}"))),
            (last, size) => Ok((u64::from(last) + 1, size)),
        }
    }

    fn command(&mut self, cb: &[u8], data: Data) -> io::Result<()> {
        self.tag = self.tag.wrapping_add(1);
        let (length, flags) = match &data {
            Data::None => (0, 0),
            Data::In(buf) => (buf.len(), DIRECTION_IN),
            Data::Out(buf) => (buf.len(), 0),
        };
        self.bulk.send(&cbw(self.tag, length as u32, flags, cb))?;
        match data {
            Data::None => {}
            Data::In(buf) => self.receive_into(buf)?,
            Data::Out(buf) => self.bulk.send(buf)?,
        }
        let csw = self.receive_csw()?;
        let signature = u32::from_le_bytes([csw[0], csw[1], csw[2], csw[3]]);
        let tag = u32::from_le_bytes([csw[4], csw[5], csw[6], csw[7]]);
        match (signature, tag, csw[12]) {
            (CSW_SIGNATURE, t, 0) if t == self.tag => Ok(()),
            (CSW_SIGNATURE, t, status) if t == self.tag => Err(io::Error::other(format!("SCSI command 0x{:02x} failed with status {status}", cb[0]))),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "the card reader answered out of step")),
        }
    }

    fn receive_into(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let mut filled = 0;
        while filled < buf.len() {
            match self.bulk.receive(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                    self.bulk.clear_halt(true)?;
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn receive_csw(&mut self) -> io::Result<[u8; CSW_LEN]> {
        let mut csw = [0u8; CSW_LEN];
        match self.bulk.receive(&mut csw) {
            Ok(CSW_LEN) => Ok(csw),
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                self.bulk.clear_halt(true)?;
                match self.bulk.receive(&mut csw)? {
                    CSW_LEN => Ok(csw),
                    n => Err(io::Error::new(io::ErrorKind::InvalidData, format!("short status from the card reader ({n} bytes)"))),
                }
            }
            Ok(n) => Err(io::Error::new(io::ErrorKind::InvalidData, format!("short status from the card reader ({n} bytes)"))),
            Err(e) => Err(e),
        }
    }

    fn lba(&self, offset: u64) -> io::Result<u32> {
        let block = u64::from(self.block_size);
        match offset % block == 0 {
            true => u32::try_from(offset / block).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "offset beyond 2 TB")),
            false => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("offset {offset} is not on a {block}-byte block"))),
        }
    }

    fn chunk(&self, len: usize) -> io::Result<usize> {
        let block = self.block_size as usize;
        match len % block == 0 {
            true => Ok(len.min(MAX_TRANSFER - MAX_TRANSFER % block)),
            false => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("length {len} is not whole {block}-byte blocks"))),
        }
    }
}

fn cbw(tag: u32, length: u32, flags: u8, cb: &[u8]) -> [u8; CBW_LEN] {
    let mut out = [0u8; CBW_LEN];
    out[0..4].copy_from_slice(&CBW_SIGNATURE.to_le_bytes());
    out[4..8].copy_from_slice(&tag.to_le_bytes());
    out[8..12].copy_from_slice(&length.to_le_bytes());
    out[12] = flags;
    out[14] = cb.len() as u8;
    out[15..15 + cb.len()].copy_from_slice(cb);
    out
}

fn rw10(opcode: u8, lba: u32, blocks: u16) -> [u8; 10] {
    let l = lba.to_be_bytes();
    let b = blocks.to_be_bytes();
    [opcode, 0, l[0], l[1], l[2], l[3], 0, b[0], b[1], 0]
}

impl<B: Bulk> Read for UsbDisk<B> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let remaining = self.capacity().saturating_sub(self.pos) as usize;
        let want = self.chunk(buf.len().min(remaining))?;
        if want == 0 {
            return Ok(0);
        }
        let lba = self.lba(self.pos)?;
        let blocks = (want / self.block_size as usize) as u16;
        self.command(&rw10(READ_10, lba, blocks), Data::In(&mut buf[..want]))?;
        self.pos += want as u64;
        Ok(want)
    }
}

impl<B: Bulk> Write for UsbDisk<B> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let want = self.chunk(buf.len())?;
        if want == 0 {
            return Ok(0);
        }
        match self.pos + want as u64 > self.capacity() {
            true => Err(io::Error::new(io::ErrorKind::WriteZero, "the image does not fit on the card")),
            false => {
                let lba = self.lba(self.pos)?;
                let blocks = (want / self.block_size as usize) as u16;
                self.command(&rw10(WRITE_10, lba, blocks), Data::Out(&buf[..want]))?;
                self.pos += want as u64;
                Ok(want)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<B: Bulk> Seek for UsbDisk<B> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(delta) => self.pos as i64 + delta,
            SeekFrom::End(delta) => self.capacity() as i64 + delta,
        };
        match new < 0 {
            true => Err(io::Error::new(io::ErrorKind::InvalidInput, "seek to a negative position")),
            false => {
                self.pos = new as u64;
                Ok(self.pos)
            }
        }
    }
}

impl<B: Bulk> BlockDevice for UsbDisk<B> {
    fn sync(&mut self) -> io::Result<()> {
        let _ = self.command(&[SYNCHRONIZE_CACHE_10, 0, 0, 0, 0, 0, 0, 0, 0, 0], Data::None);
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::VecDeque;
    use std::io::Cursor;

    use super::*;
    use crate::cardwrite::{AlignedDevice, Params, flash};

    enum Pending {
        Idle,
        Write { offset: usize, remaining: usize, tag: u32 },
    }

    pub struct SimulatedReader {
        pub card: Vec<u8>,
        pub block_size: u32,
        pub not_ready_for: usize,
        pub medium_present: bool,
        pending: Pending,
        outbox: VecDeque<Vec<u8>>,
        pub commands: Vec<u8>,
    }

    impl SimulatedReader {
        pub fn new(card_len: usize) -> Self {
            Self { card: vec![0u8; card_len], block_size: 512, not_ready_for: 0, medium_present: true, pending: Pending::Idle, outbox: VecDeque::new(), commands: Vec::new() }
        }

        fn csw(&mut self, tag: u32, status: u8) {
            let mut out = vec![0u8; CSW_LEN];
            out[0..4].copy_from_slice(&CSW_SIGNATURE.to_le_bytes());
            out[4..8].copy_from_slice(&tag.to_le_bytes());
            out[12] = status;
            self.outbox.push_back(out);
        }

        fn execute(&mut self, cbw: &[u8]) {
            let tag = u32::from_le_bytes([cbw[4], cbw[5], cbw[6], cbw[7]]);
            let cb = &cbw[15..31];
            let block = self.block_size as usize;
            let lba = u32::from_be_bytes([cb[2], cb[3], cb[4], cb[5]]) as usize;
            let blocks = u16::from_be_bytes([cb[7], cb[8]]) as usize;
            self.commands.push(cb[0]);
            match cb[0] {
                INQUIRY => {
                    let mut answer = vec![0u8; INQUIRY_LEN];
                    answer[8..16].copy_from_slice(b"Generic ");
                    answer[16..32].copy_from_slice(b"SD Card Reader  ");
                    self.outbox.push_back(answer);
                    self.csw(tag, 0);
                }
                TEST_UNIT_READY if !self.medium_present || self.not_ready_for > 0 => {
                    self.not_ready_for = self.not_ready_for.saturating_sub(1);
                    self.csw(tag, 1);
                }
                TEST_UNIT_READY | SYNCHRONIZE_CACHE_10 => self.csw(tag, 0),
                REQUEST_SENSE => {
                    let mut answer = vec![0u8; SENSE_LEN];
                    answer[2] = 0x02;
                    answer[12] = if self.medium_present { 0x04 } else { NO_MEDIUM };
                    self.outbox.push_back(answer);
                    self.csw(tag, 0);
                }
                READ_CAPACITY_10 => {
                    let last = (self.card.len() / block - 1) as u32;
                    self.outbox.push_back([last.to_be_bytes(), self.block_size.to_be_bytes()].concat());
                    self.csw(tag, 0);
                }
                READ_10 => {
                    self.outbox.push_back(self.card[lba * block..(lba + blocks) * block].to_vec());
                    self.csw(tag, 0);
                }
                WRITE_10 => self.pending = Pending::Write { offset: lba * block, remaining: blocks * block, tag },
                _ => self.csw(tag, 1),
            }
        }
    }

    impl Bulk for SimulatedReader {
        fn send(&mut self, data: &[u8]) -> io::Result<()> {
            match std::mem::replace(&mut self.pending, Pending::Idle) {
                Pending::Idle => {
                    assert_eq!(data.len(), CBW_LEN, "a command block is always 31 bytes");
                    assert_eq!(u32::from_le_bytes([data[0], data[1], data[2], data[3]]), CBW_SIGNATURE);
                    self.execute(data);
                }
                Pending::Write { offset, remaining, tag } => {
                    self.card[offset..offset + data.len()].copy_from_slice(data);
                    match remaining - data.len() {
                        0 => self.csw(tag, 0),
                        left => self.pending = Pending::Write { offset: offset + data.len(), remaining: left, tag },
                    }
                }
            }
            Ok(())
        }

        fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let chunk = self.outbox.pop_front().ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "nothing to receive"))?;
            let n = chunk.len().min(buf.len());
            buf[..n].copy_from_slice(&chunk[..n]);
            Ok(n)
        }

        fn clear_halt(&mut self, _inbound: bool) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn opening_reads_the_identity_and_capacity() {
        let disk = UsbDisk::open(SimulatedReader::new(8 * 1024 * 1024)).expect("open");
        assert_eq!(disk.capacity(), 8 * 1024 * 1024);
        assert_eq!(disk.identity(), &Identity { vendor: "Generic".into(), product: "SD Card Reader".into() });
    }

    #[test]
    fn a_reader_that_is_slow_to_spin_up_is_waited_for() {
        let reader = SimulatedReader { not_ready_for: 2, ..SimulatedReader::new(1024 * 1024) };
        let disk = UsbDisk::open(reader).expect("open once ready");
        assert_eq!(disk.into_bulk().commands.iter().filter(|c| **c == TEST_UNIT_READY).count(), 3);
    }

    #[test]
    fn an_empty_reader_says_there_is_no_card() {
        let reader = SimulatedReader { medium_present: false, ..SimulatedReader::new(1024 * 1024) };
        let err = UsbDisk::open(reader).err().expect("no medium");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("no card"));
    }

    #[test]
    fn writes_and_reads_round_trip_in_large_transfers() {
        let mut disk = UsbDisk::open(SimulatedReader::new(4 * 1024 * 1024)).expect("open");
        let data: Vec<u8> = (0..3 * 1024 * 1024).map(|i| (i % 253) as u8).collect();
        disk.seek(SeekFrom::Start(4096)).unwrap();
        disk.write_all(&data).unwrap();
        disk.seek(SeekFrom::Start(4096)).unwrap();
        let mut back = vec![0u8; data.len()];
        disk.read_exact(&mut back).unwrap();
        assert_eq!(back, data);
        assert!(disk.into_bulk().commands.iter().filter(|c| **c == WRITE_10).count() >= 6, "a 3 MiB write is split into transfers of at most 512 KiB");
    }

    #[test]
    fn unaligned_access_is_refused_rather_than_corrupting_neighbours() {
        let mut disk = UsbDisk::open(SimulatedReader::new(1024 * 1024)).expect("open");
        disk.seek(SeekFrom::Start(100)).unwrap();
        assert_eq!(disk.write(&[1u8; 512]).unwrap_err().kind(), io::ErrorKind::InvalidInput);
        disk.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(disk.write(&[1u8; 100]).unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn writing_past_the_end_of_the_card_fails() {
        let mut disk = UsbDisk::open(SimulatedReader::new(1024 * 1024)).expect("open");
        disk.seek(SeekFrom::End(-512)).unwrap();
        assert_eq!(disk.write(&[0u8; 1024]).unwrap_err().kind(), io::ErrorKind::WriteZero);
    }

    #[test]
    fn a_whole_image_flashes_through_the_reader() {
        let image = crate::cardwrite::tests::build_image();
        let card_len = image.len() + 2 * crate::cardwrite::BLOCK;
        let mut device = AlignedDevice::new(UsbDisk::open(SimulatedReader::new(card_len)).expect("open"));
        flash(&mut device, &mut Cursor::new(image.clone()), &Params { image_len: image.len() as u64, verify: true }, &crate::cardwrite::tests::cloud_init_config(), &mut |_, _, _| {}).expect("flash through USB mass storage");
        let card = device.into_inner().into_bulk().card;
        assert_eq!(&card[..512], &image[..512], "the partition table lands last and intact");
        assert!(crate::cardwrite::tests::read_fat_file(&card, "user-data").expect("user-data").contains("hostname: aircast"));
    }
}
