use std::io::Read;

use base64::Engine;
use serde_json::Value;

const PROTO_INSYNC: u8 = 0x12;
const PROTO_BAD_SILICON_REV: u8 = 0x14;
const PROTO_EOC: u8 = 0x20;
const PROTO_OK: u8 = 0x10;
const PROTO_FAILED: u8 = 0x11;
const PROTO_INVALID: u8 = 0x13;
const PROTO_GET_SYNC: u8 = 0x21;
const PROTO_GET_DEVICE: u8 = 0x22;
const PROTO_CHIP_ERASE: u8 = 0x23;
const PROTO_CHIP_VERIFY: u8 = 0x24;
const PROTO_PROG_MULTI: u8 = 0x27;
const PROTO_READ_MULTI: u8 = 0x28;
const PROTO_GET_CRC: u8 = 0x29;
const PROTO_BOOT: u8 = 0x30;
const INFO_BL_REV: u8 = 1;
const INFO_BOARD_ID: u8 = 2;
const INFO_FLASH_SIZE: u8 = 4;
const BL_REV_MIN: u32 = 2;
const BL_REV_MAX: u32 = 5;
const PROG_MULTI_MAX: usize = 64;
const READ_MULTI_MAX: usize = 0x28;

pub const BOARD_ID_PX4_FMU_V2: u32 = 9;
pub const BOARD_ID_PX4_FMU_V3: u32 = 255;
const FLASH_SIZE_SMALL: u32 = 1_032_192;
const BOOTLOADER_VERSION_V2_CORRECT_FLASH: u32 = 5;

const ERASE_TIMEOUT_MS: u64 = 20_000;
const VERIFY_TIMEOUT_MS: u64 = 5_000;
const READ_TIMEOUT_MS: u64 = 2_000;
const RESPONSE_TIMEOUT_MS: u64 = 2_000;
const SYNC_ATTEMPTS: usize = 3;

const BOOTLOADER_CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::Algorithm {
    width: 32,
    poly: 0x04c1_1db7,
    init: 0,
    refin: true,
    refout: true,
    xorout: 0,
    check: 0x2dfd_2d88,
    residue: 0,
});

pub fn crc32(bytes: &[u8], state: u32) -> u32 {
    let mut digest = BOOTLOADER_CRC.digest_with_initial(state.reverse_bits());
    digest.update(bytes);
    digest.finalize()
}

pub trait Port {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn read_exact(&mut self, count: usize, timeout_ms: u64) -> Result<Vec<u8>, String>;
    fn discard_input(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardInfo {
    pub bootloader_version: u32,
    pub board_id: u32,
    pub flash_size: u32,
}

pub struct Bootloader<P: Port> {
    port: P,
    info: Option<BoardInfo>,
    image_crc: u32,
}

fn hex(value: u8) -> String {
    format!("0x{value:02x}")
}

impl<P: Port> Bootloader<P> {
    pub fn new(port: P) -> Self {
        Bootloader { port, info: None, image_crc: 0 }
    }

    pub fn into_port(self) -> P {
        self.port
    }

    fn response(&mut self, timeout_ms: u64) -> Result<(), String> {
        let answer = self.port.read_exact(2, timeout_ms).map_err(|e| format!("Get Command Response: {e}"))?;
        match (answer[0], answer[1]) {
            (PROTO_INSYNC, PROTO_OK) => Ok(()),
            (PROTO_INSYNC, PROTO_BAD_SILICON_REV) => Err("This board is using a microcontroller with faulty silicon and an incorrect configuration and should be put out of service.".to_string()),
            (PROTO_INSYNC, code) => {
                let named = match code {
                    PROTO_FAILED => "PROTO_FAILED",
                    PROTO_INVALID => "PROTO_INVALID",
                    _ => "Unknown response code",
                };
                Err(format!("Command failed: {} ({named})", hex(code)))
            }
            (first, second) => Err(format!("Invalid sync response: {} {}", hex(first), hex(second))),
        }
    }

    fn command(&mut self, command: u8, timeout_ms: u64) -> Result<(), String> {
        self.port.write(&[command, PROTO_EOC]).and_then(|()| self.response(timeout_ms)).map_err(|e| format!("Send Command: {e}"))
    }

    fn sync(&mut self) -> Result<(), String> {
        self.port.discard_input();
        match std::iter::repeat_with(|| self.command(PROTO_GET_SYNC, RESPONSE_TIMEOUT_MS)).take(SYNC_ATTEMPTS).try_fold(String::new(), |_, attempt| attempt.map_or_else(Ok, |()| Err(()))) {
            Err(()) => Ok(()),
            Ok(last) => Err(format!("Sync: {last}")),
        }
    }

    fn read_device(&mut self, parameter: u8) -> Result<u32, String> {
        self.port.write(&[PROTO_GET_DEVICE, parameter, PROTO_EOC])?;
        let bytes = self.port.read_exact(4, READ_TIMEOUT_MS)?;
        self.response(RESPONSE_TIMEOUT_MS)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn device(&mut self, parameter: u8) -> Result<u32, String> {
        self.read_device(parameter).map_err(|e| format!("Get Device: {e}"))
    }

    fn read_board_info(&mut self) -> Result<BoardInfo, String> {
        self.sync()?;
        let bootloader_version = self.device(INFO_BL_REV)?;
        if !(BL_REV_MIN..=BL_REV_MAX).contains(&bootloader_version) {
            return Err(format!("Found unsupported bootloader version: {bootloader_version}"));
        }
        let board_id = self.device(INFO_BOARD_ID)?;
        let flash_size = self.device(INFO_FLASH_SIZE)?;
        let fmu_v3 = board_id == BOARD_ID_PX4_FMU_V2 && bootloader_version >= BOOTLOADER_VERSION_V2_CORRECT_FLASH && flash_size > FLASH_SIZE_SMALL;
        Ok(BoardInfo { bootloader_version, board_id: if fmu_v3 { BOARD_ID_PX4_FMU_V3 } else { board_id }, flash_size })
    }

    pub fn board_info(&mut self) -> Result<BoardInfo, String> {
        let info = self.read_board_info().map_err(|e| format!("Get Board Info: {e}"))?;
        self.info = Some(info);
        Ok(info)
    }

    pub fn erase(&mut self) -> Result<(), String> {
        let flash_size = self.info.map_or(0, |info| info.flash_size);
        let timeout = match flash_size > 2000 * 1024 {
            true => ERASE_TIMEOUT_MS + (f64::from(flash_size) / 1e6 * 4000.0) as u64,
            false => ERASE_TIMEOUT_MS,
        };
        self.command(PROTO_CHIP_ERASE, timeout).map_err(|e| format!("Erase failed: {e}"))
    }

    pub fn program(&mut self, image: &[u8], progress: &mut dyn FnMut(usize, usize)) -> Result<(), String> {
        self.image_crc = 0;
        let total = image.len();
        image.chunks(PROG_MULTI_MAX).try_fold(0usize, |sent, chunk| {
            let length = u8::try_from(chunk.len()).unwrap_or(u8::MAX);
            let frame: Vec<u8> = [PROTO_PROG_MULTI, length].into_iter().chain(chunk.iter().copied()).chain([PROTO_EOC]).collect();
            self.port.write(&frame).and_then(|()| self.response(RESPONSE_TIMEOUT_MS)).map_err(|e| format!("Flash failed: {e} at address 0x{sent:08x}"))?;
            self.image_crc = crc32(chunk, self.image_crc);
            progress(sent + chunk.len(), total);
            Ok::<usize, String>(sent + chunk.len())
        })?;
        let flash_size = self.info.map_or(0, |info| info.flash_size) as usize;
        let fill = vec![0xffu8; flash_size.saturating_sub(total)];
        self.image_crc = crc32(&fill, self.image_crc);
        Ok(())
    }

    fn verify_bytes(&mut self, image: &[u8], progress: &mut dyn FnMut(usize, usize)) -> Result<(), String> {
        self.command(PROTO_CHIP_VERIFY, RESPONSE_TIMEOUT_MS)?;
        let total = image.len();
        image.chunks(READ_MULTI_MAX).try_fold(0usize, |verified, expected| {
            let length = u8::try_from(expected.len()).unwrap_or(u8::MAX);
            let read = self
                .port
                .write(&[PROTO_READ_MULTI, length, PROTO_EOC])
                .and_then(|()| self.port.read_exact(expected.len(), READ_TIMEOUT_MS))
                .and_then(|bytes| self.response(RESPONSE_TIMEOUT_MS).map(|()| bytes))
                .map_err(|e| format!("Read failed: {e} at address: 0x{verified:08x}"))?;
            if let Some((at, (want, got))) = expected.iter().zip(&read).enumerate().find(|(_, (want, got))| want != got) {
                return Err(format!("Compare failed: expected({}) actual({}) at address: 0x{:08x}", hex(*want), hex(*got), verified + at));
            }
            progress(verified + expected.len(), total);
            Ok(verified + expected.len())
        })
        .map(|_| ())
    }

    fn verify_crc(&mut self) -> Result<(), String> {
        self.port.write(&[PROTO_GET_CRC, PROTO_EOC])?;
        let bytes = self.port.read_exact(4, VERIFY_TIMEOUT_MS)?;
        self.response(RESPONSE_TIMEOUT_MS)?;
        let board = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        match board == self.image_crc {
            true => Ok(()),
            false => Err(format!("CRC mismatch: board(0x{board:04x}) file(0x{:04x})", self.image_crc)),
        }
    }

    pub fn verify(&mut self, image: &[u8], progress: &mut dyn FnMut(usize, usize)) -> Result<(), String> {
        let checked = match self.info.map_or(0, |info| info.bootloader_version) <= 2 {
            true => self.verify_bytes(image, progress),
            false => self.verify_crc(),
        };
        let _ = self.reboot();
        checked
    }

    pub fn reboot(&mut self) -> Result<(), String> {
        self.port.write(&[PROTO_BOOT, PROTO_EOC])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub board_id: u32,
    pub autopilot: Option<u8>,
    pub bytes: Vec<u8>,
}

pub fn is_compatible(board_id: u32, firmware_id: u32) -> bool {
    board_id == firmware_id || (board_id == BOARD_ID_PX4_FMU_V3 && firmware_id == BOARD_ID_PX4_FMU_V2)
}

fn decompressed(json: &Value, size_key: &str, bytes_key: &str) -> Result<Vec<u8>, String> {
    let size = json.get(size_key).and_then(Value::as_u64).filter(|size| *size > 0).ok_or_else(|| format!("Firmware file has invalid decompressed size for {size_key}"))?;
    let encoded = json.get(bytes_key).and_then(Value::as_str).ok_or_else(|| format!("Could not find compressed bytes for {bytes_key} in Firmware file"))?;
    let compressed = base64::engine::general_purpose::STANDARD.decode(encoded.trim()).map_err(|e| format!("Firmware file {bytes_key} is not base64: {e}"))?;
    let mut bytes = Vec::new();
    flate2::read::ZlibDecoder::new(compressed.as_slice()).read_to_end(&mut bytes).map_err(|e| format!("Firmware file {bytes_key} does not decompress: {e}"))?;
    match bytes.len() as u64 == size {
        true => Ok(bytes),
        false => Err(format!("Size for decompressed {bytes_key} does not match stored size: Expected({size}) Actual({})", bytes.len())),
    }
}

pub fn parse_px4(text: &str, board_id: u32) -> Result<Image, String> {
    let json: Value = serde_json::from_str(text).map_err(|_| "Supplied file is not a valid JSON document".to_string())?;
    let missing = ["board_id", "image", "image_size"].into_iter().find(|key| json.get(key).is_none());
    if let Some(key) = missing {
        return Err(format!("Firmware file missing required key: {key}"));
    }
    let firmware_id = json["board_id"].as_u64().and_then(|id| u32::try_from(id).ok()).ok_or("Firmware file has invalid key: board_id")?;
    if !is_compatible(board_id, firmware_id) {
        return Err(format!("Downloaded firmware board id does not match hardware board id: {firmware_id} != {board_id}"));
    }
    let image = decompressed(&json, "image_size", "image")?;
    let padding = (4 - image.len() % 4) % 4;
    Ok(Image {
        board_id: firmware_id,
        autopilot: json.get("mav_autopilot").and_then(Value::as_u64).and_then(|a| u8::try_from(a).ok()),
        bytes: image.into_iter().chain(std::iter::repeat_n(0xff, padding)).collect(),
    })
}

#[cfg(test)]
pub mod simulated {
    use super::*;
    use std::collections::VecDeque;

    pub struct Board {
        pub bootloader_version: u32,
        pub board_id: u32,
        pub flash: Vec<u8>,
        pub address: usize,
        pub booted: bool,
        pub erased: bool,
        pending: Vec<u8>,
        out: VecDeque<u8>,
    }

    impl Board {
        pub fn new(bootloader_version: u32, board_id: u32, flash_size: usize) -> Self {
            Board { bootloader_version, board_id, flash: vec![0u8; flash_size], address: 0, booted: false, erased: false, pending: Vec::new(), out: VecDeque::new() }
        }

        fn ok(&mut self) {
            self.out.extend([PROTO_INSYNC, PROTO_OK]);
        }

        fn answer(&mut self) -> Option<usize> {
            let p = &self.pending;
            let used = match *p.first()? {
                PROTO_GET_SYNC | PROTO_CHIP_ERASE | PROTO_CHIP_VERIFY | PROTO_GET_CRC | PROTO_BOOT if p.len() >= 2 => 2,
                PROTO_GET_DEVICE | PROTO_READ_MULTI if p.len() >= 3 => 3,
                PROTO_PROG_MULTI if p.len() >= 2 && p.len() >= usize::from(p[1]) + 3 => usize::from(p[1]) + 3,
                _ => return None,
            };
            let frame: Vec<u8> = self.pending.drain(..used).collect();
            match frame[0] {
                PROTO_GET_SYNC => self.ok(),
                PROTO_GET_DEVICE => {
                    let value = match frame[1] {
                        INFO_BL_REV => self.bootloader_version,
                        INFO_BOARD_ID => self.board_id,
                        INFO_FLASH_SIZE => self.flash.len() as u32,
                        _ => 0,
                    };
                    self.out.extend(value.to_le_bytes());
                    self.ok();
                }
                PROTO_CHIP_ERASE => {
                    self.flash.iter_mut().for_each(|b| *b = 0xff);
                    self.address = 0;
                    self.erased = true;
                    self.ok();
                }
                PROTO_CHIP_VERIFY => {
                    self.address = 0;
                    self.ok();
                }
                PROTO_PROG_MULTI => {
                    let data = &frame[2..frame.len() - 1];
                    self.flash[self.address..self.address + data.len()].copy_from_slice(data);
                    self.address += data.len();
                    self.ok();
                }
                PROTO_READ_MULTI => {
                    let count = usize::from(frame[1]);
                    let bytes: Vec<u8> = self.flash[self.address..self.address + count].to_vec();
                    self.address += count;
                    self.out.extend(bytes);
                    self.ok();
                }
                PROTO_GET_CRC => {
                    self.out.extend(crc32(&self.flash, 0).to_le_bytes());
                    self.ok();
                }
                PROTO_BOOT => self.booted = true,
                _ => {}
            }
            Some(used)
        }
    }

    impl Port for Board {
        fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
            self.pending.extend_from_slice(bytes);
            while self.answer().is_some() {}
            Ok(())
        }

        fn read_exact(&mut self, count: usize, _timeout_ms: u64) -> Result<Vec<u8>, String> {
            match self.out.len() >= count {
                true => Ok(self.out.drain(..count).collect()),
                false => Err("Timeout waiting for bytes to be available".to_string()),
            }
        }

        fn discard_input(&mut self) {
            self.out.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::simulated::Board;
    use super::*;
    use std::io::Write;

    fn px4_file(board_id: u32, image: &[u8]) -> String {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(image).unwrap();
        let packed = base64::engine::general_purpose::STANDARD.encode(encoder.finish().unwrap());
        serde_json::json!({ "board_id": board_id, "image_size": image.len(), "image": packed, "mav_autopilot": 12 }).to_string()
    }

    #[test]
    fn the_crc_is_qgcs_table_crc_seeded_with_zero() {
        assert_eq!(crc32(b"123456789", 0), 0x2dfd_2d88);
        assert_eq!(crc32(b"6789", crc32(b"12345", 0)), crc32(b"123456789", 0), "it chains the way the flash loop feeds it chunk by chunk");
    }

    #[test]
    fn a_px4_file_flashes_a_simulated_board_and_its_crc_matches() {
        let firmware: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
        let image = parse_px4(&px4_file(50, &firmware), 50).unwrap();
        assert_eq!(image.bytes.len() % 4, 0, "padded to a whole word with 0xff");
        assert_eq!(image.autopilot, Some(12));
        let mut loader = Bootloader::new(Board::new(5, 50, 4096));
        assert_eq!(loader.board_info().unwrap(), BoardInfo { bootloader_version: 5, board_id: 50, flash_size: 4096 });
        loader.erase().unwrap();
        let mut seen = Vec::new();
        loader.program(&image.bytes, &mut |done, total| seen.push((done, total))).unwrap();
        assert_eq!(seen.last(), Some(&(1000, 1000)));
        loader.verify(&image.bytes, &mut |_, _| {}).unwrap();
        let board = loader.into_port();
        assert!(board.erased && board.booted);
        assert_eq!(&board.flash[..1000], &firmware[..]);
    }

    #[test]
    fn an_old_bootloader_is_verified_byte_by_byte_and_a_bad_byte_is_named() {
        let firmware = vec![0xa5u8; 200];
        let mut loader = Bootloader::new(Board::new(2, 9, 2048));
        loader.board_info().unwrap();
        loader.erase().unwrap();
        loader.program(&firmware, &mut |_, _| {}).unwrap();
        let mut board = loader.into_port();
        board.flash[130] = 0;
        let mut loader = Bootloader { port: board, info: Some(BoardInfo { bootloader_version: 2, board_id: 9, flash_size: 2048 }), image_crc: 0 };
        assert_eq!(loader.verify(&firmware, &mut |_, _| {}), Err("Compare failed: expected(0xa5) actual(0x00) at address: 0x00000082".to_string()));
    }

    #[test]
    fn a_v2_board_reporting_big_flash_is_an_fmu_v3_and_takes_v2_firmware() {
        let mut loader = Bootloader::new(Board::new(5, BOARD_ID_PX4_FMU_V2, 2 * 1024 * 1024));
        assert_eq!(loader.board_info().unwrap().board_id, BOARD_ID_PX4_FMU_V3);
        assert!(parse_px4(&px4_file(9, &[1, 2, 3, 4]), BOARD_ID_PX4_FMU_V3).is_ok());
        assert_eq!(parse_px4(&px4_file(50, &[1, 2, 3, 4]), 9), Err("Downloaded firmware board id does not match hardware board id: 50 != 9".to_string()));
    }

    #[test]
    fn a_silent_port_and_an_unsupported_bootloader_read_as_qt_words_them() {
        struct Silent;
        impl Port for Silent {
            fn write(&mut self, _bytes: &[u8]) -> Result<(), String> {
                Ok(())
            }
            fn read_exact(&mut self, _count: usize, _timeout_ms: u64) -> Result<Vec<u8>, String> {
                Err("Timeout waiting for bytes to be available".to_string())
            }
            fn discard_input(&mut self) {}
        }
        assert_eq!(Bootloader::new(Silent).board_info(), Err("Get Board Info: Sync: Send Command: Get Command Response: Timeout waiting for bytes to be available".to_string()));
        assert_eq!(Bootloader::new(Board::new(6, 50, 1024)).board_info(), Err("Get Board Info: Found unsupported bootloader version: 6".to_string()));
    }
}
