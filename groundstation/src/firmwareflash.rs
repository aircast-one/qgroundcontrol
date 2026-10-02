use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::bootloader::{BoardInfo, Bootloader, Port};

const BAUD: u32 = 115_200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Connecting,
    Erasing,
    Programming,
    Verifying,
    Complete,
    Failed,
}

impl Phase {
    fn token(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Connecting => "connecting",
            Phase::Erasing => "erasing",
            Phase::Programming => "programming",
            Phase::Verifying => "verifying",
            Phase::Complete => "complete",
            Phase::Failed => "failed",
        }
    }

    fn busy(self) -> bool {
        matches!(self, Phase::Connecting | Phase::Erasing | Phase::Programming | Phase::Verifying)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Job {
    pub phase: Phase,
    pub progress: f64,
    pub messages: Vec<String>,
    pub error: Option<String>,
    pub board: Option<BoardInfo>,
    pub port: Option<String>,
    pub file: Option<String>,
}

pub enum Event {
    Phase(Phase),
    Status(String),
    Board(BoardInfo),
    Progress(f64),
}

fn image_for(file: &str, contents: &[u8], board: &BoardInfo, report: &mut dyn FnMut(Event)) -> Result<Vec<u8>, String> {
    let lower = file.to_ascii_lowercase();
    let bytes = match lower.ends_with(".px4") || lower.ends_with(".apj") {
        true => crate::bootloader::parse_px4(&String::from_utf8_lossy(contents), board.board_id).map_err(|e| image_load_failed(&e, report))?.bytes,
        false => contents.iter().copied().chain(std::iter::repeat_n(0xff, (4 - contents.len() % 4) % 4)).collect(),
    };
    match bytes.len() <= board.flash_size as usize {
        true => Ok(bytes),
        false => Err(format!("Image size of {} is too large for board flash size {}", bytes.len(), board.flash_size)),
    }
}

pub const PLUG_IN: &str = "Plug in your device via USB.";
pub const REPLUG: &str = "Now unplug your device and plug it back in to enter bootloader mode.";
pub const FIND_BOARD_INTERVAL_MS: u64 = 500;
pub const SUSPENDED: &str = "Connect not allowed during Firmware Upgrade.";

pub fn suspended() -> bool {
    job().phase.busy()
}

const APM_CHIBIOS: &str = "settings.firmwareUpgradeSettings.apmChibiOS";
pub const FLASH_CANCELLED: &str = "Cancelled. Select a port and press Flash to try again.";
static CANCEL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn cancelled() -> bool {
    CANCEL.load(std::sync::atomic::Ordering::SeqCst)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sighting {
    Absent,
    Running,
    Bootloader,
}

pub fn in_bootloader(description: &str) -> bool {
    description.contains("BL") || description.to_ascii_lowercase().contains("bootloader")
}

pub fn wait_for_bootloader(look: &mut dyn FnMut() -> Sighting, pause: &mut dyn FnMut(), cancelled: &dyn Fn() -> bool, report: &mut dyn FnMut(Event)) -> Result<(), String> {
    let first = look();
    if first == Sighting::Bootloader {
        return Ok(());
    }
    let mut must_leave = first == Sighting::Running;
    report(Event::Status(if must_leave { REPLUG } else { PLUG_IN }.to_string()));
    let arrived = std::iter::repeat(()).take_while(|()| !cancelled()).any(|()| {
        pause();
        match (must_leave, look()) {
            (true, Sighting::Absent) => {
                must_leave = false;
                false
            }
            (true, _) | (false, Sighting::Absent) => false,
            (false, _) => true,
        }
    });
    if arrived { Ok(()) } else { Err(FLASH_CANCELLED.to_string()) }
}

pub fn flash<P: Port>(port: P, file: &str, contents: &[u8], report: &mut dyn FnMut(Event)) -> Result<(), String> {
    flash_from(port, is_ihx(file), &mut |_| Ok((file.to_string(), contents.to_vec())), report)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    File(String),
    Url(String),
    Px4(crate::firmwarecatalog::Build),
    ArduPilot { vehicle: crate::firmwarecatalog::Vehicle, build: crate::firmwarecatalog::Build },
    Sik,
}

pub const SIK_FIRMWARE_URL: &str = "https://px4-travis.s3.amazonaws.com/SiK/stable";
const SIK_OPEN_SETTLE_MS: u64 = 1_000;

pub fn sik_url(board_id: u32) -> Option<String> {
    let image = match board_id {
        crate::bootloader::BOARD_ID_SIK_RADIO_1000 => "radio~hm_trp.ihx",
        crate::bootloader::BOARD_ID_SIK_RADIO_1060 => "radio~hb1060.ihx",
        _ => return None,
    };
    Some(format!("{SIK_FIRMWARE_URL}/{image}"))
}

fn is_ihx(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".ihx")
}

impl Source {
    pub fn sik(&self) -> bool {
        match self {
            Source::Sik => true,
            Source::File(name) | Source::Url(name) => is_ihx(name),
            _ => false,
        }
    }
}

pub fn source(given: &str) -> Result<Source, String> {
    let parts: Vec<&str> = given.split(':').collect();
    match parts.as_slice() {
        [scheme, ..] if matches!(*scheme, "http" | "https") => Ok(Source::Url(given.to_string())),
        ["sik", "stable"] => Ok(Source::Sik),
        ["sik", build] => Err(format!("SiK radio firmware is only published as stable, not {build}")),
        ["px4", build] => crate::firmwarecatalog::Build::parse(build).map(Source::Px4).ok_or_else(|| format!("PX4 builds are stable, beta or dev, not {build}")),
        ["ardupilot", vehicle, build] => match (crate::firmwarecatalog::Vehicle::parse(vehicle), crate::firmwarecatalog::Build::parse(build)) {
            (Some(vehicle), Some(build)) => Ok(Source::ArduPilot { vehicle, build }),
            _ => Err(format!("ArduPilot builds are ardupilot:<copter|heli|plane|rover|sub>:<stable|beta|dev>, not {given}")),
        },
        _ => Ok(Source::File(given.to_string())),
    }
}

static MANIFEST: Mutex<Option<Vec<crate::firmwarecatalog::Entry>>> = Mutex::new(None);

fn manifest() -> Result<Vec<crate::firmwarecatalog::Entry>, String> {
    let mut held = MANIFEST.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(entries) = held.as_ref() {
        return Ok(entries.clone());
    }
    let bytes = crate::firmwarecatalog::download(crate::firmwarecatalog::ARDUPILOT_MANIFEST_URL)?;
    let entries = crate::firmwarecatalog::parse_manifest(&String::from_utf8_lossy(&bytes))?;
    *held = Some(entries.clone());
    Ok(entries)
}

pub fn resolve(source: &Source, board: &BoardInfo, description: &str, report: &mut dyn FnMut(Event)) -> Result<(String, Vec<u8>), String> {
    let url = match source {
        Source::File(path) => {
            report(Event::Status(format!("Using firmware file {path}")));
            return std::fs::read(path).map(|bytes| (path.clone(), bytes)).map_err(|e| format!("Unable to open firmware file {path}: {e}"));
        }
        Source::Url(url) => url.clone(),
        Source::Px4(build) => crate::firmwarecatalog::px4_url(board.board_id, *build).ok_or_else(|| "Unable to find specified firmware for board type".to_string())?,
        Source::Sik => sik_url(board.board_id).ok_or_else(|| "Unable to find specified firmware for board type".to_string())?,
        Source::ArduPilot { vehicle, build } => {
            report(Event::Status("Downloading the ArduPilot firmware list...".into()));
            let chibios = crate::settingsstore::raw_setting(APM_CHIBIOS).and_then(|v| v.as_i64()).unwrap_or(0) == 0;
            crate::firmwarecatalog::apm_url(&manifest()?, board.board_id, *build, *vehicle, chibios, description)?
        }
    };
    report(Event::Status(format!("Downloading firmware from {url}")));
    let bytes = crate::firmwarecatalog::download(&url)?;
    report(Event::Status("Download complete".into()));
    Ok((url, bytes))
}

fn flash_sik<P: Port>(loader: &mut Bootloader<P>, board: &BoardInfo, fetch: &mut dyn FnMut(&BoardInfo) -> Result<(String, Vec<u8>), String>, report: &mut dyn FnMut(Event)) -> Result<(), String> {
    let (_, contents) = fetch(board)?;
    let blocks = crate::bootloader::parse_ihx(&String::from_utf8_lossy(&contents)).map_err(|e| image_load_failed(&e, report))?;
    loader.init_flash_sequence()?;
    report(Event::Phase(Phase::Erasing));
    report(Event::Status("Erasing previous program...".into()));
    loader.erase()?;
    report(Event::Status("Erase complete".into()));
    report(Event::Phase(Phase::Programming));
    report(Event::Status("Programming new version...".into()));
    loader.program_ihx(&blocks, &mut |done, total| report(Event::Progress(done as f64 / total.max(1) as f64)))?;
    report(Event::Phase(Phase::Verifying));
    report(Event::Status("Verifying program...".into()));
    loader.verify_ihx(&blocks, &mut |_, _| {})?;
    report(Event::Status("Rebooting board".into()));
    Ok(())
}

fn image_load_failed(error: &str, report: &mut dyn FnMut(Event)) -> String {
    report(Event::Status(error.to_string()));
    "Image load failed".to_string()
}

fn flash_px4<P: Port>(loader: &mut Bootloader<P>, board: &BoardInfo, fetch: &mut dyn FnMut(&BoardInfo) -> Result<(String, Vec<u8>), String>, report: &mut dyn FnMut(Event)) -> Result<(), String> {
    let (file, contents) = fetch(board)?;
    let image = image_for(&file, &contents, board, report)?;
    report(Event::Phase(Phase::Erasing));
    report(Event::Status("Erasing previous program...".into()));
    loader.erase()?;
    report(Event::Status("Erase complete".into()));
    report(Event::Phase(Phase::Programming));
    report(Event::Status("Programming new version...".into()));
    loader.program(&image, &mut |done, total| report(Event::Progress(done as f64 / total.max(1) as f64)))?;
    report(Event::Phase(Phase::Verifying));
    report(Event::Status("Verifying program...".into()));
    loader.verify(&image, &mut |_, _| {})?;
    report(Event::Status("Rebooting board".into()));
    Ok(())
}

pub fn flash_from<P: Port>(port: P, sik: bool, fetch: &mut dyn FnMut(&BoardInfo) -> Result<(String, Vec<u8>), String>, report: &mut dyn FnMut(Event)) -> Result<(), String> {
    let mut loader = if sik { Bootloader::sik(port) } else { Bootloader::new(port) };
    let board = loader.board_info()?;
    report(Event::Board(board));
    report(Event::Status("Connected to bootloader:".into()));
    report(Event::Status(format!("  Version: {}", board.bootloader_version)));
    report(Event::Status(format!("  Board ID: {}", board.board_id)));
    report(Event::Status(format!("  Flash size: {}", board.flash_size)));
    let flashed = match sik {
        true => flash_sik(&mut loader, &board, fetch, report),
        false => flash_px4(&mut loader, &board, fetch, report),
    };
    flashed.inspect_err(|_| {
        let _ = loader.reboot();
    })
}

static JOB: LazyLock<Mutex<Job>> = LazyLock::new(|| Mutex::new(Job::default()));

fn job() -> MutexGuard<'static, Job> {
    JOB.lock().unwrap_or_else(PoisonError::into_inner)
}

fn apply(event: Event) {
    let mut held = job();
    match event {
        Event::Phase(phase) => held.phase = phase,
        Event::Status(text) => held.messages.push(text),
        Event::Board(board) => held.board = Some(board),
        Event::Progress(fraction) => held.progress = fraction,
    }
}

#[cfg(not(target_os = "android"))]
struct Serial(Box<dyn serialport::SerialPort>);

#[cfg(not(target_os = "android"))]
impl Port for Serial {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        use std::io::Write;
        self.0.write_all(bytes).and_then(|()| self.0.flush()).map_err(|e| format!("Write failed: {e}"))
    }

    fn read_exact(&mut self, count: usize, timeout_ms: u64) -> Result<Vec<u8>, String> {
        use std::io::Read;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let mut bytes = vec![0u8; count];
        let mut filled = 0;
        while filled < count {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                return Err("Timeout waiting for bytes to be available".to_string());
            }
            let _ = self.0.set_timeout(left);
            match self.0.read(&mut bytes[filled..]) {
                Ok(read) => filled += read,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(format!("Read failed: error: {e}")),
            }
        }
        Ok(bytes)
    }

    fn discard_input(&mut self) {
        let _ = self.0.clear(serialport::ClearBuffer::Input);
    }

    fn set_baud(&mut self, baud: u32) -> Result<(), String> {
        self.0.set_baud_rate(baud).map_err(|e| format!("Unable to set baud rate {baud}: {e}"))
    }
}

#[cfg(not(target_os = "android"))]
fn look_at(port: &str) -> Sighting {
    let found = serialport::available_ports().unwrap_or_default().into_iter().find(|p| p.port_name == port);
    match found.map(|p| p.port_type) {
        None => Sighting::Absent,
        Some(serialport::SerialPortType::UsbPort(usb)) if in_bootloader(usb.product.as_deref().unwrap_or_default()) => Sighting::Bootloader,
        Some(_) => Sighting::Running,
    }
}

fn description_of(port: &str) -> String {
    ports().into_iter().find(|p| p["port"] == port).and_then(|p| p["description"].as_str().map(str::to_string)).unwrap_or_default()
}

#[cfg(not(target_os = "android"))]
fn ports() -> Vec<Value> {
    serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| !p.port_name.starts_with("/dev/tty."))
        .map(|p| {
            let description = match &p.port_type {
                serialport::SerialPortType::UsbPort(usb) => usb.product.clone().or_else(|| usb.manufacturer.clone()).unwrap_or_default(),
                _ => String::new(),
            };
            json!({ "port": p.port_name, "bootloader": in_bootloader(&description), "description": description })
        })
        .collect()
}

#[cfg(target_os = "android")]
fn ports() -> Vec<Value> {
    crate::platformserial::ports()
        .into_iter()
        .map(|p| json!({ "port": p.system_location, "bootloader": in_bootloader(&p.description), "description": crate::platformserial::display_name(&p) }))
        .collect()
}

#[cfg(target_os = "android")]
fn look_at(port: &str) -> Sighting {
    match crate::platformserial::ports().into_iter().find(|p| p.system_location == port) {
        None => Sighting::Absent,
        Some(p) if in_bootloader(&p.description) => Sighting::Bootloader,
        Some(_) => Sighting::Running,
    }
}

#[cfg(not(target_os = "android"))]
fn open(port: &str) -> Result<Serial, String> {
    serialport::new(port, BAUD)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .open()
        .map(Serial)
        .map_err(|e| format!("Open failed on port {port}: {e}"))
}

#[cfg(any(target_os = "android", test))]
const FLASH_SERIAL_ID: u32 = 0xfff0_0002;

#[cfg(any(target_os = "android", test))]
type Inbox = std::sync::Arc<(Mutex<(std::collections::VecDeque<u8>, Option<String>)>, std::sync::Condvar)>;

#[cfg(any(target_os = "android", test))]
struct Usb {
    serial: crate::platformserial::PlatformSerial,
    inbox: Inbox,
    port: String,
}

#[cfg(any(target_os = "android", test))]
impl Drop for Usb {
    fn drop(&mut self) {
        self.serial.close();
    }
}

#[cfg(any(target_os = "android", test))]
impl Port for Usb {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        match self.serial.write(bytes) {
            true => Ok(()),
            false => Err("Write failed: the USB device refused the bytes".to_string()),
        }
    }

    fn read_exact(&mut self, count: usize, timeout_ms: u64) -> Result<Vec<u8>, String> {
        let (lock, ready) = &*self.inbox;
        let held = lock.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut held, _) = ready
            .wait_timeout_while(held, std::time::Duration::from_millis(timeout_ms), |(bytes, gone)| bytes.len() < count && gone.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        match (held.0.len() >= count, held.1.clone()) {
            (true, _) => Ok(held.0.drain(..count).collect()),
            (false, Some(reason)) => Err(format!("Read failed: error: {reason}")),
            (false, None) => Err("Timeout waiting for bytes to be available".to_string()),
        }
    }

    fn discard_input(&mut self) {
        self.inbox.0.lock().unwrap_or_else(PoisonError::into_inner).0.clear();
    }

    fn set_baud(&mut self, baud: u32) -> Result<(), String> {
        self.serial.close();
        self.serial = usb_serial(&self.port, baud, self.inbox.clone())?;
        Ok(())
    }
}

#[cfg(target_os = "android")]
fn open(port: &str) -> Result<Usb, String> {
    open_usb(port)
}

#[cfg(any(target_os = "android", test))]
fn open_usb(port: &str) -> Result<Usb, String> {
    let inbox: Inbox = std::sync::Arc::new((Mutex::new((std::collections::VecDeque::new(), None)), std::sync::Condvar::new()));
    let serial = usb_serial(port, BAUD, inbox.clone())?;
    Ok(Usb { serial, inbox, port: port.to_string() })
}

#[cfg(any(target_os = "android", test))]
fn usb_serial(port: &str, baud: u32, fed: Inbox) -> Result<crate::platformserial::PlatformSerial, String> {
    crate::platformserial::PlatformSerial::open(FLASH_SERIAL_ID, port, baud, 8, 1, 0, move |event| {
        let (lock, ready) = &*fed;
        let mut held = lock.lock().unwrap_or_else(PoisonError::into_inner);
        match event {
            crate::platformserial::Event::Bytes(bytes) => held.0.extend(bytes),
            crate::platformserial::Event::Disconnected(reason) => held.1 = Some(reason),
        }
        ready.notify_all();
    })
    .map_err(|e| format!("Open failed on port {port}: {e}"))
}

pub fn for_hardware(chosen: Source, radio: bool) -> Result<Source, String> {
    match (radio, chosen) {
        (true, picked) if !picked.sik() => Ok(Source::Sik),
        (false, picked) if picked.sik() => Err("SiK radio firmware can only be flashed to a SiK radio".to_string()),
        (_, picked) => Ok(picked),
    }
}

pub fn start(port: &str, file: &str) -> Result<(), String> {
    let chosen = for_hardware(source(file)?, crate::corelinks::board_type_at(port) == Some(crate::boards::BoardType::SiKRadio))?;
    if let Source::File(path) = &chosen {
        std::fs::metadata(path).map_err(|e| format!("Unable to open firmware file {path}: {e}"))?;
    }
    {
        let mut held = job();
        if held.phase.busy() {
            return Err("A firmware upgrade is already running".to_string());
        }
        CANCEL.store(false, std::sync::atomic::Ordering::SeqCst);
        *held = Job { phase: Phase::Connecting, port: Some(port.to_string()), file: Some(file.to_string()), ..Job::default() };
    }
    if crate::hub::lock().active().is_none() {
        crate::corelinks::close_links_at(crate::hub::now_ms(), None, "firmware upgrade");
    }
    let port = port.to_string();
    std::thread::Builder::new()
        .name("firmware-flash".into())
        .spawn(move || {
            let waited = wait_for_bootloader(&mut || look_at(&port), &mut || std::thread::sleep(std::time::Duration::from_millis(FIND_BOARD_INTERVAL_MS)), &cancelled, &mut apply);
            let description = description_of(&port);
            let outcome = waited.and_then(|()| open(&port)).and_then(|opened| {
                if chosen.sik() {
                    std::thread::sleep(std::time::Duration::from_millis(SIK_OPEN_SETTLE_MS));
                }
                flash_from(opened, chosen.sik(), &mut |board| resolve(&chosen, board, &description, &mut apply).and_then(|fetched| if cancelled() { Err(FLASH_CANCELLED.to_string()) } else { Ok(fetched) }), &mut apply)
            });
            let mut held = job();
            match outcome {
                Ok(()) => {
                    held.phase = Phase::Complete;
                    held.messages.push("Upgrade complete".into());
                }
                Err(e) if e == FLASH_CANCELLED => {
                    held.phase = Phase::Idle;
                    held.messages.push(e);
                }
                Err(e) => {
                    held.phase = Phase::Failed;
                    held.messages.extend([format!("Error: {e}"), "Upgrade cancelled".to_string()]);
                    held.error = Some(e);
                }
            }
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn view(_backend: &dyn crate::router::Backend, _args: &[String]) -> Value {
    let held = job();
    json!({
        "kind": "object",
        "class": "FirmwareUpgrade",
        "phase": held.phase.token(),
        "busy": held.phase.busy(),
        "cancellable": held.phase == Phase::Connecting,
        "progress": held.progress,
        "messages": held.messages,
        "error": held.error,
        "port": held.port,
        "file": held.file,
        "board": held.board.map(|b| json!({ "bootloaderVersion": b.bootloader_version, "boardId": b.board_id, "flashSize": b.flash_size })),
    })
}

pub fn ports_view(_backend: &dyn crate::router::Backend, _args: &[String]) -> Value {
    json!({ "kind": "object", "class": "FirmwarePorts", "ports": ports() })
}

pub fn invoke(path: &str, args: &str) -> Option<Value> {
    if path == "firmware.cancel" {
        return Some(match job().phase == Phase::Connecting {
            true => {
                CANCEL.store(true, std::sync::atomic::Ordering::SeqCst);
                json!({ "ok": true })
            }
            false => json!({ "ok": false, "reason": "The upgrade can no longer be cancelled once erasing has started." }),
        });
    }
    (path == "firmware.flash").then_some(())?;
    let given: Vec<String> = serde_json::from_str(args).unwrap_or_default();
    let (Some(port), Some(file)) = (given.first(), given.get(1)) else {
        return Some(json!({ "ok": false, "reason": "firmware.flash needs the bootloader's serial port and the firmware file" }));
    };
    Some(match start(port, file) {
        Ok(()) => json!({ "ok": true }),
        Err(reason) => json!({ "ok": false, "reason": reason }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootloader::simulated::Board;

    #[test]
    fn a_bin_file_flashes_through_every_phase_and_reports_qt_status_lines() {
        let mut events = Vec::new();
        let firmware = vec![0x5au8; 300];
        flash(Board::new(5, 50, 1024), "fw.bin", &firmware, &mut |event| events.push(event)).unwrap();
        let phases: Vec<Phase> = events.iter().filter_map(|e| if let Event::Phase(p) = e { Some(*p) } else { None }).collect();
        assert_eq!(phases, [Phase::Erasing, Phase::Programming, Phase::Verifying]);
        let lines: Vec<&str> = events.iter().filter_map(|e| if let Event::Status(s) = e { Some(s.as_str()) } else { None }).collect();
        assert_eq!(lines[..4], ["Connected to bootloader:", "  Version: 5", "  Board ID: 50", "  Flash size: 1024"]);
        assert_eq!(lines[4..], ["Erasing previous program...", "Erase complete", "Programming new version...", "Verifying program...", "Rebooting board"]);
        assert!(events.iter().any(|e| matches!(e, Event::Progress(p) if (*p - 1.0).abs() < f64::EPSILON)));
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn a_px4_file_flashes_over_a_real_serial_device_to_a_board_on_the_other_end() {
        use serialport::SerialPort;
        use std::io::{Read, Write};
        let (mut master, slave) = serialport::TTYPort::pair().unwrap();
        let firmware: Vec<u8> = (0..3000u32).map(|i| (i * 13 % 256) as u8).collect();
        let board_side = std::thread::spawn(move || {
            let mut board = Board::new(5, 50, 8192);
            let _ = master.set_timeout(std::time::Duration::from_millis(50));
            let mut chunk = [0u8; 512];
            while !board.booted {
                if let Ok(read) = master.read(&mut chunk) {
                    board.write(&chunk[..read]).unwrap();
                    master.write_all(&board.output()).unwrap();
                }
            }
            board.flash
        });
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&firmware).unwrap();
        let px4 = serde_json::json!({ "board_id": 50, "image_size": firmware.len(), "image": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, encoder.finish().unwrap()) }).to_string();
        flash(Serial(Box::new(slave)), "fw.px4", px4.as_bytes(), &mut |_| {}).unwrap();
        let flashed = board_side.join().unwrap();
        assert_eq!(&flashed[..firmware.len()], &firmware[..]);
        assert!(flashed[firmware.len()..].iter().all(|b| *b == 0xff));
    }

    fn watched(sightings: &[Sighting]) -> (Result<(), String>, Vec<String>, usize) {
        let mut queue = sightings.iter().copied();
        let pauses = std::cell::Cell::new(0);
        let mut lines = Vec::new();
        let outcome = wait_for_bootloader(&mut || queue.next().unwrap_or(Sighting::Absent), &mut || pauses.set(pauses.get() + 1), &|| pauses.get() >= 1000, &mut |e| {
            if let Event::Status(s) = e {
                lines.push(s);
            }
        });
        (outcome, lines, pauses.get())
    }

    #[test]
    fn a_board_already_in_its_bootloader_is_taken_at_once() {
        assert_eq!(watched(&[Sighting::Bootloader]), (Ok(()), vec![], 0));
    }

    #[test]
    fn a_running_board_must_be_replugged_and_its_return_is_taken() {
        let (outcome, lines, pauses) = watched(&[Sighting::Running, Sighting::Running, Sighting::Absent, Sighting::Absent, Sighting::Running]);
        assert_eq!((outcome, lines, pauses), (Ok(()), vec![REPLUG.to_string()], 4), "back after the unplug counts even before its description says BL, as Qt's found-board loop does");
    }

    #[test]
    fn a_missing_board_is_waited_for_until_the_user_cancels() {
        let (outcome, lines, _) = watched(&[Sighting::Absent, Sighting::Absent, Sighting::Bootloader]);
        assert_eq!((outcome, lines), (Ok(()), vec![PLUG_IN.to_string()]));
        let (outcome, _, pauses) = watched(&[]);
        assert_eq!((outcome, pauses), (Err(FLASH_CANCELLED.to_string()), 1000), "the search has no time limit, as the Qt find-board loop runs until cancel");
        assert!(in_bootloader("PX4 BL FMU v5.x") && in_bootloader("ArduPilot Bootloader") && !in_bootloader("PX4 FMU v5.x"));
    }

    static USB_BOARD: Mutex<Option<Board>> = Mutex::new(None);

    fn usb_open(_id: u32, _port: &str, baud: u32, data_bits: i64, stop_bits: i64, parity: i64) -> bool {
        (baud, data_bits, stop_bits, parity) == (BAUD, 8, 1, 0)
    }

    fn usb_write(id: u32, bytes: &[u8]) -> bool {
        let answer = USB_BOARD.lock().unwrap().as_mut().map(|board| {
            board.write(bytes).unwrap();
            board.output()
        });
        let delivered = answer.filter(|out| !out.is_empty()).map(|out| std::thread::spawn(move || crate::platformserial::received(id, out)));
        delivered.map(|thread| thread.join().is_ok()).unwrap_or(true)
    }

    fn usb_close(_id: u32) {}

    fn usb_ports() -> Vec<crate::boards::PortInfo> {
        Vec::new()
    }

    #[test]
    fn a_bin_file_flashes_over_the_android_usb_hooks_to_a_board_on_the_other_end() {
        crate::platformserial::install(crate::platformserial::Hooks { open: usb_open, write: usb_write, close: usb_close, ports: usb_ports });
        *USB_BOARD.lock().unwrap() = Some(Board::new(5, 50, 2048));
        let firmware: Vec<u8> = (0..700u32).map(|i| (i % 253) as u8).collect();
        flash(open_usb("/dev/bus/usb/001/002").unwrap(), "fw.bin", &firmware, &mut |_| {}).unwrap();
        let board = USB_BOARD.lock().unwrap().take().unwrap();
        assert!(board.booted);
        assert_eq!(&board.flash[..700], &firmware[..]);
    }

    #[test]
    fn a_firmware_source_reads_as_a_file_a_url_or_a_release_to_look_up() {
        use crate::firmwarecatalog::{Build, Vehicle};
        assert_eq!(source("/tmp/fw.px4"), Ok(Source::File("/tmp/fw.px4".into())));
        assert_eq!(source("https://firmware.ardupilot.org/x.apj"), Ok(Source::Url("https://firmware.ardupilot.org/x.apj".into())));
        assert_eq!(source("px4:beta"), Ok(Source::Px4(Build::Beta)));
        assert_eq!(source("px4:dev"), Ok(Source::Px4(Build::Developer)), "the developer build is master, offered behind Advanced");
        assert_eq!(source("ardupilot:heli:dev"), Ok(Source::ArduPilot { vehicle: Vehicle::Heli, build: Build::Developer }));
        assert!(source("ardupilot:boat:stable").is_err());
        let board = BoardInfo { bootloader_version: 5, board_id: 4242, flash_size: 1024 };
        assert_eq!(resolve(&Source::Px4(Build::Stable), &board, "", &mut |_| {}), Err("Unable to find specified firmware for board type".to_string()), "a board PX4 publishes no build for is refused before any download");
    }

    #[test]
    fn a_sik_radio_flashes_an_ihx_through_the_firmware_flow() {
        let ihx = ":0400000001020304F2\n:00000001FF\n";
        let mut lines = Vec::new();
        flash(Board::sik_radio(crate::bootloader::BOARD_ID_SIK_RADIO_1060, 0x100), "radio.ihx", ihx.as_bytes(), &mut |e| {
            if let Event::Status(s) = e {
                lines.push(s);
            }
        })
        .unwrap();
        assert_eq!(lines[..4], ["Connected to bootloader:", "  Version: 0", "  Board ID: 80", "  Flash size: 0"], "a SiK radio reports like a bootloader, as _foundBoardInfo logs it");
        assert_eq!(source("sik:stable"), Ok(Source::Sik));
        assert!(Source::Url("https://x/radio~hb1060.ihx".into()).sik() && !Source::Px4(crate::firmwarecatalog::Build::Stable).sik());
        assert_eq!(sik_url(80).as_deref(), Some("https://px4-travis.s3.amazonaws.com/SiK/stable/radio~hb1060.ihx"));
    }

    #[test]
    fn the_hardware_decides_between_a_sik_radio_and_an_autopilot_as_the_upgrade_thread_does() {
        assert_eq!(for_hardware(Source::Px4(crate::firmwarecatalog::Build::Stable), true), Ok(Source::Sik), "a radio always takes the latest SiK firmware");
        assert_eq!(for_hardware(Source::File("custom.ihx".into()), true), Ok(Source::File("custom.ihx".into())));
        assert!(for_hardware(Source::Sik, false).is_err());
        assert_eq!(for_hardware(Source::Px4(crate::firmwarecatalog::Build::Beta), false), Ok(Source::Px4(crate::firmwarecatalog::Build::Beta)));
    }

    #[test]
    fn an_image_bigger_than_the_flash_is_refused_before_erasing() {
        let mut events = Vec::new();
        let mut board = Board::new(5, 50, 256);
        let outcome = flash(&mut board, "fw.bin", &[0u8; 400], &mut |event| events.push(event));
        assert_eq!(outcome, Err("Image size of 400 is too large for board flash size 256".to_string()));
        assert!(board.booted, "a failed upgrade reboots the board out of its bootloader as PX4FirmwareUpgradeThread does");
        let mut corrupt = Board::new(5, 50, 256);
        assert_eq!(flash(&mut corrupt, "fw.px4", b"not json", &mut |_| {}), Err("Image load failed".to_string()));
        assert!(!events.iter().any(|e| matches!(e, Event::Phase(Phase::Erasing))));
    }
}
