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
const PROTO_LOAD_ADDRESS: u8 = 0x24;
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
const SIK_COMMAND_BAUD: u32 = 57_600;
const SIK_BOOTLOADER_BAUD: u32 = 115_200;
const SIK_COMMAND_MODE_TIMEOUT_MS: u64 = 2_000;
const SIK_UPDATE_TIMEOUT_MS: u64 = 1_500;
pub const BOARD_ID_SIK_RADIO_1000: u32 = 78;
pub const BOARD_ID_SIK_RADIO_1060: u32 = 80;

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
    fn set_baud(&mut self, _baud: u32) -> Result<(), String> {
        Ok(())
    }
}

pub type IhxBlocks = Vec<(u16, Vec<u8>)>;

fn hex_byte(text: &str, at: usize) -> Option<u8> {
    text.get(at..at + 2).and_then(|pair| u8::from_str_radix(pair, 16).ok())
}

fn ihx_record(line: &str) -> Result<(u8, u16, Vec<u8>), String> {
    let record = line.strip_prefix(':').ok_or("Incorrectly formatted .ihx file, line does not begin with :")?;
    let short = || "Incorrectly formatted line in .ihx file, line too short".to_string();
    let count = usize::from(hex_byte(record, 0).ok_or_else(short)?);
    let address = u16::from_be_bytes([hex_byte(record, 2).ok_or_else(short)?, hex_byte(record, 4).ok_or_else(short)?]);
    let kind = hex_byte(record, 6).ok_or_else(short)?;
    let bytes: Vec<u8> = (0..count).map(|i| hex_byte(record, 8 + 2 * i)).collect::<Option<_>>().ok_or_else(short)?;
    hex_byte(record, 8 + 2 * count).ok_or_else(short)?;
    match kind {
        0 | 1 => Ok((kind, address, bytes)),
        other => Err(format!("Unsupported record type in file: {other}")),
    }
}

pub fn parse_ihx(text: &str) -> Result<IhxBlocks, String> {
    let records: Vec<(u8, u16, Vec<u8>)> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ihx_record)
        .take_while(|record| !matches!(record, Ok((1, _, _))))
        .collect::<Result<_, _>>()?;
    Ok(records.into_iter().fold(Vec::new(), |mut blocks: IhxBlocks, (_, address, bytes)| {
        match blocks.last_mut().filter(|(start, held)| usize::from(*start) + held.len() == usize::from(address)) {
            Some((_, held)) => held.extend(bytes),
            None => blocks.push((address, bytes)),
        }
        blocks
    }))
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
    sik: bool,
    in_bootloader: bool,
}

fn hex(value: u8) -> String {
    format!("0x{value:02x}")
}

impl<P: Port> Bootloader<P> {
    pub fn new(port: P) -> Self {
        Bootloader { port, info: None, image_crc: 0, sik: false, in_bootloader: false }
    }

    pub fn sik(port: P) -> Self {
        Bootloader { sik: true, ..Bootloader::new(port) }
    }

    fn read_line(&mut self, timeout_ms: u64) -> String {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let mut line = Vec::new();
        while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()).map(|d| d.as_millis() as u64).filter(|left| *left > 0) {
            match self.port.read_exact(1, left).ok().and_then(|b| b.first().copied()) {
                Some(b'\n') if line.last() == Some(&b'\r') => {
                    line.pop();
                    return String::from_utf8_lossy(&line).into_owned();
                }
                Some(byte) => line.push(byte),
                None => break,
            }
        }
        String::new()
    }

    fn command_mode_board_id(&mut self) -> Result<u32, String> {
        self.port.discard_input();
        self.port.set_baud(SIK_COMMAND_BAUD)?;
        self.port.write(b"+++")?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(SIK_COMMAND_MODE_TIMEOUT_MS);
        let mut heard = Vec::new();
        while !String::from_utf8_lossy(&heard).contains("OK") {
            let left = deadline.saturating_duration_since(std::time::Instant::now()).as_millis() as u64;
            match (left, self.port.read_exact(1, left.max(1))) {
                (0, _) | (_, Err(_)) if heard.is_empty() => return Err("Unable to put radio into command mode +++".to_string()),
                (0, _) | (_, Err(_)) => return Err("Radio did not respond to command mode".to_string()),
                (_, Ok(byte)) => heard.extend(byte),
            }
        }
        self.port.discard_input();
        self.port.write(b"ATI2\r\n")?;
        if self.read_line(SIK_COMMAND_MODE_TIMEOUT_MS) != "ATI2" {
            return Err("Radio did not respond to ATI2 command".to_string());
        }
        self.read_line(SIK_COMMAND_MODE_TIMEOUT_MS).trim().parse().map_err(|_| "Radio did not return board id".to_string())
    }

    fn bootloader_board_id(&mut self) -> Result<u32, String> {
        let read = |this: &mut Self| -> Result<u32, String> {
            this.port.write(&[PROTO_GET_DEVICE, PROTO_EOC])?;
            let bytes = this.port.read_exact(2, READ_TIMEOUT_MS)?;
            this.response(RESPONSE_TIMEOUT_MS)?;
            Ok(u32::from(bytes[0]))
        };
        read(self).map_err(|e| format!("Get Board Id: {e}"))
    }

    fn sik_board_info(&mut self) -> Result<BoardInfo, String> {
        self.in_bootloader = self.sync().is_ok();
        let board_id = match self.in_bootloader {
            true => self.bootloader_board_id()?,
            false => self.command_mode_board_id()?,
        };
        Ok(BoardInfo { bootloader_version: 0, board_id, flash_size: 0 })
    }

    pub fn init_flash_sequence(&mut self) -> Result<(), String> {
        if !self.sik || self.in_bootloader {
            return Ok(());
        }
        self.port.write(b"AT&UPDATE\r\n")?;
        self.port.read_exact(1, SIK_UPDATE_TIMEOUT_MS).map_err(|_| "Unable to reboot radio (ready read)".to_string())?;
        self.port.set_baud(SIK_BOOTLOADER_BAUD)?;
        self.sync()?;
        self.in_bootloader = true;
        Ok(())
    }

    fn load_address(&mut self, address: u16) -> Result<(), String> {
        let [high, low] = address.to_be_bytes();
        self.port.write(&[PROTO_LOAD_ADDRESS, low, high, PROTO_EOC]).and_then(|()| self.response(RESPONSE_TIMEOUT_MS))
    }

    pub fn program_ihx(&mut self, blocks: &IhxBlocks, progress: &mut dyn FnMut(usize, usize)) -> Result<(), String> {
        let total: usize = blocks.iter().map(|(_, bytes)| bytes.len()).sum();
        blocks.iter().try_fold(0usize, |sent, (address, bytes)| {
            self.load_address(*address).map_err(|_| format!("Unable to set flash start address: 0x{address:08x}"))?;
            bytes.chunks(PROG_MULTI_MAX).try_fold(sent, |sent, chunk| {
                let frame: Vec<u8> = [PROTO_PROG_MULTI, chunk.len() as u8].into_iter().chain(chunk.iter().copied()).chain([PROTO_EOC]).collect();
                self.port.write(&frame).and_then(|()| self.response(RESPONSE_TIMEOUT_MS)).map_err(|e| format!("Flash failed: {e} at address 0x{address:08x}"))?;
                progress(sent + chunk.len(), total);
                Ok::<usize, String>(sent + chunk.len())
            })
        })
        .map(|_| ())
    }

    pub fn verify_ihx(&mut self, blocks: &IhxBlocks, progress: &mut dyn FnMut(usize, usize)) -> Result<(), String> {
        let total: usize = blocks.iter().map(|(_, bytes)| bytes.len()).sum();
        let checked = blocks.iter().try_fold(0usize, |verified, (address, bytes)| {
            self.load_address(*address).map_err(|_| format!("Unable to set read start address: 0x{address:08x}"))?;
            bytes.chunks(READ_MULTI_MAX).enumerate().try_fold(verified, |verified, (index, expected)| {
                let read = self
                    .port
                    .write(&[PROTO_READ_MULTI, expected.len() as u8, PROTO_EOC])
                    .and_then(|()| self.port.read_exact(expected.len(), READ_TIMEOUT_MS))
                    .and_then(|got| self.response(RESPONSE_TIMEOUT_MS).map(|()| got))
                    .map_err(|e| format!("Read failed: {e} at address: 0x{address:08x}"))?;
                if let Some((at, (want, got))) = expected.iter().zip(&read).enumerate().find(|(_, (want, got))| want != got) {
                    let place = usize::from(*address) + index * READ_MULTI_MAX + at;
                    return Err(format!("Compare failed: expected({}) actual({}) at address: 0x{place:08x}", hex(*want), hex(*got)));
                }
                progress(verified + expected.len(), total);
                Ok(verified + expected.len())
            })
        });
        let _ = self.reboot();
        checked.map(|_| ())
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
        let info = match self.sik {
            true => self.sik_board_info(),
            false => self.read_board_info(),
        }.map_err(|e| format!("Get Board Info: {e}"))?;
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
        match self.sik && !self.in_bootloader {
            true => {
                self.port.discard_input();
                self.port.write(b"ATZ\r\n")
            }
            false => self.port.write(&[PROTO_BOOT, PROTO_EOC]),
        }
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
        pub sik: bool,
        pub command_mode: bool,
        pub baud: u32,
        pending: Vec<u8>,
        out: VecDeque<u8>,
    }

    impl Board {
        pub fn new(bootloader_version: u32, board_id: u32, flash_size: usize) -> Self {
            Board { bootloader_version, board_id, flash: vec![0u8; flash_size], address: 0, booted: false, erased: false, sik: false, command_mode: false, baud: 115_200, pending: Vec::new(), out: VecDeque::new() }
        }

        pub fn sik_radio(board_id: u32, flash_size: usize) -> Self {
            Board { sik: true, command_mode: true, ..Board::new(0, board_id, flash_size) }
        }

        fn command(&mut self) -> Option<usize> {
            let text = String::from_utf8_lossy(&self.pending).into_owned();
            if self.baud != 57_600 {
                let used = self.pending.len();
                self.pending.clear();
                return (used > 0).then_some(used);
            }
            let (used, reply, update) = if let Some(at) = text.find("+++") {
                (at + 3, "OK\r\n".to_string(), false)
            } else if let Some(at) = text.find("ATI2\r\n") {
                (at + 6, format!("ATI2\r\n{}\r\n", self.board_id), false)
            } else if let Some(at) = text.find("AT&UPDATE\r\n") {
                (at + 11, "AT&UPDATE\r\n".to_string(), true)
            } else {
                return None;
            };
            self.pending.drain(..used);
            self.out.extend(reply.into_bytes());
            self.command_mode = !update;
            Some(used)
        }

        pub fn output(&mut self) -> Vec<u8> {
            self.out.drain(..).collect()
        }

        fn ok(&mut self) {
            self.out.extend([PROTO_INSYNC, PROTO_OK]);
        }

        fn answer(&mut self) -> Option<usize> {
            if self.command_mode {
                return self.command();
            }
            let p = &self.pending;
            let used = match *p.first()? {
                PROTO_LOAD_ADDRESS if self.sik && p.len() >= 4 => 4,
                PROTO_GET_DEVICE if self.sik && p.len() >= 2 => 2,
                PROTO_GET_SYNC | PROTO_CHIP_ERASE | PROTO_CHIP_VERIFY | PROTO_GET_CRC | PROTO_BOOT if !self.sik && p.len() >= 2 => 2,
                PROTO_GET_SYNC | PROTO_CHIP_ERASE | PROTO_GET_CRC | PROTO_BOOT if p.len() >= 2 => 2,
                PROTO_GET_DEVICE | PROTO_READ_MULTI if p.len() >= 3 => 3,
                PROTO_PROG_MULTI if p.len() >= 2 && p.len() >= usize::from(p[1]) + 3 => usize::from(p[1]) + 3,
                _ => return None,
            };
            let frame: Vec<u8> = self.pending.drain(..used).collect();
            match frame[0] {
                PROTO_GET_SYNC => self.ok(),
                PROTO_LOAD_ADDRESS if self.sik => {
                    self.address = usize::from(u16::from_le_bytes([frame[1], frame[2]]));
                    self.ok();
                }
                PROTO_GET_DEVICE if self.sik => {
                    self.out.extend([self.board_id as u8, 0]);
                    self.ok();
                }
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

        fn set_baud(&mut self, baud: u32) -> Result<(), String> {
            self.baud = baud;
            Ok(())
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

    const IHX: &str = ":0400000001020304F2\n:02000400050600EF\n:02010000AABB98\n:00000001FF\ngarbage after the end";

    #[test]
    fn an_ihx_file_reads_as_contiguous_blocks_up_to_its_end_record() {
        assert_eq!(parse_ihx(IHX), Ok(vec![(0, vec![1, 2, 3, 4, 5, 6]), (0x100, vec![0xaa, 0xbb])]));
        assert_eq!(parse_ihx("0400"), Err("Incorrectly formatted .ihx file, line does not begin with :".to_string()));
        assert_eq!(parse_ihx(":02000002AABB00"), Err("Unsupported record type in file: 2".to_string()));
        assert_eq!(parse_ihx(":0400000001"), Err("Incorrectly formatted line in .ihx file, line too short".to_string()));
    }

    #[test]
    fn a_sik_radio_is_taken_from_command_mode_into_its_bootloader_and_flashed() {
        let blocks = parse_ihx(IHX).unwrap();
        let mut loader = Bootloader::sik(Board::sik_radio(BOARD_ID_SIK_RADIO_1060, 0x400));
        assert_eq!(loader.board_info().unwrap(), BoardInfo { bootloader_version: 0, board_id: BOARD_ID_SIK_RADIO_1060, flash_size: 0 });
        loader.init_flash_sequence().unwrap();
        loader.erase().unwrap();
        loader.program_ihx(&blocks, &mut |_, _| {}).unwrap();
        loader.verify_ihx(&blocks, &mut |_, _| {}).unwrap();
        let board = loader.into_port();
        assert!(board.booted && !board.command_mode && board.baud == 115_200);
        assert_eq!((&board.flash[..6], &board.flash[0x100..0x102]), (&[1u8, 2, 3, 4, 5, 6][..], &[0xaau8, 0xbb][..]));
    }

    #[test]
    fn a_sik_radio_already_in_its_bootloader_answers_its_id_directly() {
        let mut radio = Board::sik_radio(BOARD_ID_SIK_RADIO_1000, 0x400);
        radio.command_mode = false;
        let mut loader = Bootloader::sik(radio);
        assert_eq!(loader.board_info().unwrap().board_id, BOARD_ID_SIK_RADIO_1000);
        loader.init_flash_sequence().unwrap();
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
        let mut loader = Bootloader { port: board, info: Some(BoardInfo { bootloader_version: 2, board_id: 9, flash_size: 2048 }), ..Bootloader::new(Board::new(2, 9, 0)) };
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
