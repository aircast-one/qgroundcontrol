use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::time::{Duration, Instant};

use mbrman::MBR;
use sha2::{Digest, Sha256};

use crate::cardprovision::{ProvisionConfig, ProvisionFile, ProvisionPlan};

pub const BLOCK: usize = 1024 * 1024;
pub const ALIGN: u64 = 4096;
const SECTOR: u64 = 512;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

pub trait BlockDevice: Read + Write + Seek {
    fn sync(&mut self) -> io::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Write,
    Verify,
    Customize,
}

pub struct Params {
    pub image_len: u64,
    pub verify: bool,
}

pub fn flash(device: &mut dyn BlockDevice, image: &mut dyn Read, params: &Params, provision: &ProvisionConfig, progress: &mut dyn FnMut(Stage, u64, u64)) -> Result<(), String> {
    let partition_table = read_partition_table(image)?;
    wipe_stale_backup_gpt(device)?;
    let write_digest = write_after_partition_table(device, image, &partition_table, params.image_len, progress)?;
    device.sync().map_err(|e| ioerr("sync after write", e))?;
    match params.verify {
        true => verify(device, &partition_table, params.image_len, &write_digest, progress)?,
        false => {}
    }
    progress(Stage::Customize, 0, 0);
    customize(device, &partition_table, provision)?;
    device.seek(SeekFrom::Start(0)).map_err(|e| ioerr("seek to block 0", e))?;
    device.write_all(&partition_table).map_err(|e| ioerr("write block 0", e))?;
    device.sync().map_err(|e| ioerr("final sync", e))
}

fn read_partition_table(image: &mut dyn Read) -> Result<Vec<u8>, String> {
    let mut first = vec![0u8; BLOCK];
    let len = read_full(image, &mut first).map_err(|e| ioerr("read image", e))?;
    first.truncate(len);
    Ok(first)
}

fn wipe_stale_backup_gpt(device: &mut dyn BlockDevice) -> Result<(), String> {
    let end = device.seek(SeekFrom::End(0)).map_err(|e| ioerr("seek to end of device", e))?;
    match end <= BLOCK as u64 {
        true => Ok(()),
        false => {
            device.seek(SeekFrom::Start(end - BLOCK as u64)).map_err(|e| ioerr("seek to tail of device", e))?;
            device.write_all(&vec![0u8; BLOCK]).map_err(|e| ioerr("wipe tail of device", e))
        }
    }
}

fn write_after_partition_table(device: &mut dyn BlockDevice, image: &mut dyn Read, partition_table: &[u8], image_len: u64, progress: &mut dyn FnMut(Stage, u64, u64)) -> Result<Vec<u8>, String> {
    device.seek(SeekFrom::Start(BLOCK as u64)).map_err(|e| ioerr("seek past block 0", e))?;
    let mut hash = Sha256::new();
    hash.update(partition_table);
    let mut written = partition_table.len() as u64;
    let mut last_tick = Instant::now();
    let mut buf = vec![0u8; BLOCK];
    progress(Stage::Write, written, image_len);
    loop {
        let n = read_full(image, &mut buf).map_err(|e| ioerr("read image", e))?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
        device.write_all(&buf[..n]).map_err(|e| ioerr("write to device", e))?;
        written += n as u64;
        if last_tick.elapsed() >= PROGRESS_INTERVAL {
            progress(Stage::Write, written, image_len);
            last_tick = Instant::now();
        }
        if n < BLOCK {
            break;
        }
    }
    progress(Stage::Write, written, image_len);
    Ok(hash.finalize().to_vec())
}

fn verify(device: &mut dyn BlockDevice, partition_table: &[u8], image_len: u64, write_digest: &[u8], progress: &mut dyn FnMut(Stage, u64, u64)) -> Result<(), String> {
    device.seek(SeekFrom::Start(BLOCK as u64)).map_err(|e| ioerr("seek for verify", e))?;
    let mut hash = Sha256::new();
    hash.update(partition_table);
    let mut checked = partition_table.len() as u64;
    let mut remaining = image_len.saturating_sub(checked);
    let mut last_tick = Instant::now();
    let mut buf = vec![0u8; BLOCK];
    progress(Stage::Verify, checked, image_len);
    while remaining > 0 {
        let want = remaining.min(BLOCK as u64) as usize;
        read_exact_from(device, &mut buf[..want]).map_err(|e| ioerr("read back from device", e))?;
        hash.update(&buf[..want]);
        checked += want as u64;
        remaining -= want as u64;
        if last_tick.elapsed() >= PROGRESS_INTERVAL {
            progress(Stage::Verify, checked, image_len);
            last_tick = Instant::now();
        }
    }
    progress(Stage::Verify, checked, image_len);
    match hash.finalize().as_slice() == write_digest {
        true => Ok(()),
        false => Err("Verification failed: the card does not match the image".to_string()),
    }
}

fn customize(device: &mut dyn BlockDevice, partition_table: &[u8], provision: &ProvisionConfig) -> Result<(), String> {
    let plan = crate::cardprovision::plan(provision);
    if plan == ProvisionPlan::Noop {
        return Ok(());
    }
    let (offset, length) = fat_partition_extent(partition_table)?;
    {
        let slice = fscommon::StreamSlice::new(&mut *device, offset, offset + length).map_err(|e| ioerr("slice boot partition", e))?;
        let fs = fatfs::FileSystem::new(slice, fatfs::FsOptions::new()).map_err(|e| ioerr("open FAT filesystem", e))?;
        let root = fs.root_dir();
        match plan {
            ProvisionPlan::Noop => {}
            ProvisionPlan::CloudInit { files } => files.iter().try_for_each(|file| write_fat_file(&root, file))?,
            ProvisionPlan::FirstRun { script, cmdline_append } => {
                write_fat_file(&root, &script)?;
                append_cmdline(&root, &cmdline_append)?;
            }
        }
    }
    device.sync().map_err(|e| ioerr("sync after customize", e))
}

fn write_fat_file<T: fatfs::ReadWriteSeek>(root: &fatfs::Dir<'_, T>, file: &ProvisionFile) -> Result<(), String> {
    let mut handle = root.create_file(&file.name).map_err(|e| ioerr(&format!("create {}", file.name), e))?;
    handle.truncate().map_err(|e| ioerr(&format!("truncate {}", file.name), e))?;
    handle.write_all(file.contents.as_bytes()).map_err(|e| ioerr(&format!("write {}", file.name), e))
}

fn append_cmdline<T: fatfs::ReadWriteSeek>(root: &fatfs::Dir<'_, T>, fragment: &str) -> Result<(), String> {
    let current = match root.open_file("cmdline.txt") {
        Ok(mut handle) => {
            let mut text = String::new();
            handle.read_to_string(&mut text).map_err(|e| ioerr("read cmdline.txt", e))?;
            text
        }
        Err(_) => String::new(),
    };
    if current.contains(fragment.trim()) {
        return Ok(());
    }
    let updated = format!("{}{fragment}", current.trim_end_matches(['\r', '\n']));
    let mut handle = root.create_file("cmdline.txt").map_err(|e| ioerr("create cmdline.txt", e))?;
    handle.truncate().map_err(|e| ioerr("truncate cmdline.txt", e))?;
    handle.write_all(updated.as_bytes()).map_err(|e| ioerr("write cmdline.txt", e))
}

fn fat_partition_extent(partition_table: &[u8]) -> Result<(u64, u64), String> {
    let mbr = MBR::read_from(&mut Cursor::new(partition_table), SECTOR as u32).map_err(|e| format!("parse MBR: {e}"))?;
    mbr.iter()
        .map(|(_, part)| part)
        .find(|part| part.is_used() && is_fat(part.sys))
        .map(|part| (u64::from(part.starting_lba) * SECTOR, u64::from(part.sectors) * SECTOR))
        .ok_or_else(|| "no FAT boot partition found in image MBR".to_string())
}

fn is_fat(sys: u8) -> bool {
    matches!(sys, 0x01 | 0x04 | 0x06 | 0x0b | 0x0c | 0x0e)
}

pub fn read_full(reader: &mut dyn Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

fn read_exact_from(reader: &mut dyn Read, buf: &mut [u8]) -> io::Result<()> {
    match read_full(reader, buf)? == buf.len() {
        true => Ok(()),
        false => Err(io::Error::new(io::ErrorKind::UnexpectedEof, "device returned fewer bytes than the image length")),
    }
}

fn ioerr(context: &str, e: io::Error) -> String {
    format!("{context}: {e}")
}

pub struct AlignedDevice<T: Read + Write + Seek> {
    inner: T,
    pos: u64,
}

impl<T: Read + Write + Seek> AlignedDevice<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    fn align_down(n: u64) -> u64 {
        n - (n % ALIGN)
    }

    fn read_span(&mut self, aligned_off: u64, span: &mut [u8]) -> io::Result<()> {
        self.inner.seek(SeekFrom::Start(aligned_off))?;
        let filled = read_full(&mut self.inner, span)?;
        span[filled..].fill(0);
        Ok(())
    }

    fn write_span(&mut self, aligned_off: u64, span: &[u8]) -> io::Result<()> {
        self.inner.seek(SeekFrom::Start(aligned_off))?;
        self.inner.write_all(span)
    }
}

impl<T: Read + Write + Seek> Read for AlignedDevice<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let start = self.pos;
        let end = start + buf.len() as u64;
        let aligned_start = Self::align_down(start);
        let mut span = vec![0u8; (end.div_ceil(ALIGN) * ALIGN - aligned_start) as usize];
        self.read_span(aligned_start, &mut span)?;
        let lo = (start - aligned_start) as usize;
        buf.copy_from_slice(&span[lo..lo + buf.len()]);
        self.pos = end;
        Ok(buf.len())
    }
}

impl<T: Read + Write + Seek> Write for AlignedDevice<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let start = self.pos;
        let end = start + buf.len() as u64;
        let aligned_start = Self::align_down(start);
        let aligned_end = end.div_ceil(ALIGN) * ALIGN;
        if aligned_start == start && aligned_end == end {
            self.write_span(aligned_start, buf)?;
            self.pos = end;
            return Ok(buf.len());
        }
        let align = ALIGN as usize;
        let span_len = (aligned_end - aligned_start) as usize;
        let mut span = vec![0u8; span_len];
        let head_partial = start != aligned_start;
        let tail_partial = end != aligned_end;
        if head_partial {
            self.read_span(aligned_start, &mut span[..align])?;
        }
        if tail_partial && !(head_partial && span_len == align) {
            self.read_span(aligned_end - ALIGN, &mut span[span_len - align..])?;
        }
        let lo = (start - aligned_start) as usize;
        span[lo..lo + buf.len()].copy_from_slice(buf);
        self.write_span(aligned_start, &span)?;
        self.pos = end;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<T: Read + Write + Seek> Seek for AlignedDevice<T> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(delta) => self.pos as i64 + delta,
            SeekFrom::End(delta) => self.inner.seek(SeekFrom::End(0))? as i64 + delta,
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

impl<T: BlockDevice> BlockDevice for AlignedDevice<T> {
    fn sync(&mut self) -> io::Result<()> {
        self.inner.sync()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::cardprovision::{InitFormat, WifiConfig};

    const PSK: &str = "f42c6fc52df0ebef9ebb4b90b38a5f902e83fe1b135a70e23aed762e9710a12e";
    const PART_START_SECTOR: u32 = 2048;
    const FAT_SECTORS: u32 = 16 * 1024;
    const IMAGE_SECTORS: u32 = PART_START_SECTOR + FAT_SECTORS + 2048;

    pub struct MemDevice {
        pub cur: Cursor<Vec<u8>>,
    }

    impl MemDevice {
        pub fn zeroed(len: usize) -> Self {
            Self { cur: Cursor::new(vec![0u8; len]) }
        }
    }

    impl Read for MemDevice {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.cur.read(buf)
        }
    }

    impl Write for MemDevice {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.cur.write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Seek for MemDevice {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.cur.seek(pos)
        }
    }

    impl BlockDevice for MemDevice {
        fn sync(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FlakyDevice {
        cur: Cursor<Vec<u8>>,
        corrupt_at: u64,
        corrupted: bool,
    }

    impl Read for FlakyDevice {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.cur.read(buf)
        }
    }

    impl Write for FlakyDevice {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let pos = self.cur.position();
            let n = self.cur.write(buf)?;
            if !self.corrupted && self.corrupt_at >= pos && self.corrupt_at < pos + n as u64 {
                self.cur.get_mut()[self.corrupt_at as usize] ^= 0xff;
                self.corrupted = true;
            }
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Seek for FlakyDevice {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.cur.seek(pos)
        }
    }

    impl BlockDevice for FlakyDevice {
        fn sync(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    pub fn build_image() -> Vec<u8> {
        let total = IMAGE_SECTORS as usize * SECTOR as usize;
        let mut image = vec![0u8; total];
        let mut scratch = Cursor::new(vec![0u8; total]);
        let mut mbr = MBR::new_from(&mut scratch, SECTOR as u32, [0x12, 0x34, 0x56, 0x78]).expect("new MBR");
        mbr[1] = mbrman::MBRPartitionEntry { boot: 0, first_chs: mbrman::CHS::empty(), sys: 0x0c, last_chs: mbrman::CHS::empty(), starting_lba: PART_START_SECTOR, sectors: FAT_SECTORS };
        let mut mbr_bytes = Cursor::new(vec![0u8; SECTOR as usize]);
        mbr.write_into(&mut mbr_bytes).expect("write MBR");
        image[..SECTOR as usize].copy_from_slice(&mbr_bytes.into_inner());
        let part_off = PART_START_SECTOR as usize * SECTOR as usize;
        let part_len = FAT_SECTORS as usize * SECTOR as usize;
        fatfs::format_volume(fscommon::StreamSlice::new(Cursor::new(&mut image[part_off..part_off + part_len]), 0, part_len as u64).expect("slice"), fatfs::FormatVolumeOptions::new().fat_type(fatfs::FatType::Fat32)).expect("format FAT");
        {
            let fs = fatfs::FileSystem::new(fscommon::StreamSlice::new(Cursor::new(&mut image[part_off..part_off + part_len]), 0, part_len as u64).expect("slice"), fatfs::FsOptions::new()).expect("open seed fs");
            fs.root_dir().create_file("cmdline.txt").expect("create cmdline").write_all(b"console=serial0,115200 root=PARTUUID=abcd rootwait").expect("write cmdline");
        }
        image
    }

    pub fn read_fat_file(image: &[u8], name: &str) -> Option<String> {
        let part_off = PART_START_SECTOR as usize * SECTOR as usize;
        let part_len = FAT_SECTORS as usize * SECTOR as usize;
        let mut owned = image[part_off..part_off + part_len].to_vec();
        let fs = fatfs::FileSystem::new(fscommon::StreamSlice::new(Cursor::new(&mut owned), 0, part_len as u64).ok()?, fatfs::FsOptions::new()).ok()?;
        let mut text = String::new();
        fs.root_dir().open_file(name).ok()?.read_to_string(&mut text).ok()?;
        Some(text)
    }

    pub fn cloud_init_config() -> ProvisionConfig {
        ProvisionConfig { hostname: Some("aircast".into()), wifi: Some(WifiConfig { ssid: "IEEE".into(), password: "password".into(), country: "US".into() }), tailscale: None, access: None, init_format: InitFormat::CloudInit }
    }

    #[test]
    fn flash_writes_verifies_and_customizes() {
        let image = build_image();
        let mut device = MemDevice::zeroed(image.len());
        flash(&mut device, &mut Cursor::new(image.clone()), &Params { image_len: image.len() as u64, verify: true }, &cloud_init_config(), &mut |_, _, _| {}).expect("flash should succeed and verify");
        let written = device.cur.into_inner();
        assert_eq!(&written[..SECTOR as usize], &image[..SECTOR as usize], "MBR on device must match the image");
        let net = read_fat_file(&written, "network-config").expect("network-config exists");
        assert!(net.contains(PSK), "network-config must contain derived PSK");
        assert!(!net.contains("password: \"password\""), "no plaintext passphrase");
        assert!(read_fat_file(&written, "user-data").expect("user-data exists").contains("hostname: aircast"));
        assert!(read_fat_file(&written, "meta-data").is_some());
    }

    #[test]
    fn firstrun_appends_cmdline() {
        let image = build_image();
        let mut device = MemDevice::zeroed(image.len());
        let config = ProvisionConfig { init_format: InitFormat::FirstRun, ..cloud_init_config() };
        flash(&mut device, &mut Cursor::new(image.clone()), &Params { image_len: image.len() as u64, verify: false }, &config, &mut |_, _, _| {}).expect("flash firstrun");
        let written = device.cur.into_inner();
        assert!(read_fat_file(&written, "firstrun.sh").expect("firstrun.sh exists").contains(&format!("psk={PSK}")));
        let cmdline = read_fat_file(&written, "cmdline.txt").expect("cmdline.txt exists");
        assert!(cmdline.contains("console=serial0,115200"), "kept original cmdline");
        assert!(cmdline.contains("systemd.run=/boot/firstrun.sh"), "appended firstrun fragment");
    }

    #[test]
    fn flash_wipes_the_tail_of_a_larger_card() {
        let image = build_image();
        let card_len = image.len() + 4 * BLOCK;
        let mut device = MemDevice::zeroed(card_len);
        let last_sector = card_len - SECTOR as usize;
        device.cur.get_mut()[last_sector..last_sector + 8].copy_from_slice(b"EFI PART");
        flash(&mut device, &mut Cursor::new(image.clone()), &Params { image_len: image.len() as u64, verify: true }, &cloud_init_config(), &mut |_, _, _| {}).expect("flash onto an oversized card");
        let written = device.cur.into_inner();
        assert_eq!(written.len(), card_len, "the card must not grow or shrink");
        assert!(written[card_len - BLOCK..].iter().all(|&b| b == 0), "a previous card's backup GPT must not survive the flash");
    }

    #[test]
    fn verify_catches_corruption() {
        let image = build_image();
        let mut device = FlakyDevice { cur: Cursor::new(vec![0u8; image.len()]), corrupt_at: (PART_START_SECTOR as u64 + 100) * SECTOR, corrupted: false };
        let err = flash(&mut device, &mut Cursor::new(image.clone()), &Params { image_len: image.len() as u64, verify: true }, &cloud_init_config(), &mut |_, _, _| {}).expect_err("verify must fail on a corrupted card");
        assert!(err.contains("Verification failed"), "got: {err}");
    }

    #[test]
    fn progress_reports_every_stage_and_ends_at_the_image_length() {
        let image = build_image();
        let mut device = MemDevice::zeroed(image.len());
        let mut seen: Vec<(Stage, u64, u64)> = Vec::new();
        flash(&mut device, &mut Cursor::new(image.clone()), &Params { image_len: image.len() as u64, verify: true }, &cloud_init_config(), &mut |stage, done, total| seen.push((stage, done, total))).expect("flash");
        let last = |stage: Stage| seen.iter().filter(|(s, _, _)| *s == stage).last().copied();
        assert_eq!(last(Stage::Write), Some((Stage::Write, image.len() as u64, image.len() as u64)));
        assert_eq!(last(Stage::Verify), Some((Stage::Verify, image.len() as u64, image.len() as u64)));
        assert_eq!(last(Stage::Customize), Some((Stage::Customize, 0, 0)));
    }

    #[test]
    fn unaligned_writes_round_trip_like_a_plain_cursor() {
        let size = 4 * ALIGN as usize + 777;
        let mut aligned = AlignedDevice::new(MemDevice::zeroed(size));
        let mut plain = Cursor::new(vec![0u8; size]);
        let ops: &[(u64, usize, u8)] = &[(0, 10, 0x11), (10, 4096, 0x22), (4090, 20, 0x33), (8192, 4096, 0x44), (8000, 500, 0x55), (12345, 1234, 0x66), (size as u64 - 5, 5, 0x77)];
        ops.iter().for_each(|&(off, len, val)| {
            aligned.seek(SeekFrom::Start(off)).unwrap();
            aligned.write_all(&vec![val; len]).unwrap();
            plain.seek(SeekFrom::Start(off)).unwrap();
            plain.write_all(&vec![val; len]).unwrap();
        });
        let mut got = vec![0u8; size];
        aligned.seek(SeekFrom::Start(0)).unwrap();
        aligned.read_exact(&mut got).unwrap();
        assert_eq!(got, plain.into_inner(), "aligned read-back must match plain cursor");
    }

    #[test]
    fn unaligned_reads_match_plain_cursor() {
        let size = 3 * ALIGN as usize + 123;
        let source: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
        let mut aligned = AlignedDevice::new(MemDevice { cur: Cursor::new(source.clone()) });
        [(0u64, 1usize), (1, 5), (4095, 2), (4096, 4096), (5000, 3000), (size as u64 - 10, 10)].iter().for_each(|&(off, len)| {
            aligned.seek(SeekFrom::Start(off)).unwrap();
            let mut got = vec![0u8; len];
            aligned.read_exact(&mut got).unwrap();
            assert_eq!(got, &source[off as usize..off as usize + len], "unaligned read at off={off} len={len}");
        });
    }

    #[test]
    fn seek_from_end_and_current() {
        let size = 2 * ALIGN as usize;
        let mut aligned = AlignedDevice::new(MemDevice::zeroed(size));
        aligned.seek(SeekFrom::End(-100)).unwrap();
        aligned.write_all(&[0xAB; 100]).unwrap();
        aligned.seek(SeekFrom::Start(size as u64 - 100)).unwrap();
        let mut got = vec![0u8; 100];
        aligned.read_exact(&mut got).unwrap();
        assert_eq!(got, vec![0xAB; 100]);
        aligned.seek(SeekFrom::Start(50)).unwrap();
        assert_eq!(aligned.seek(SeekFrom::Current(25)).unwrap(), 75);
    }
}
