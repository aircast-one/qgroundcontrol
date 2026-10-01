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

fn image_for(file: &str, contents: &[u8], board: &BoardInfo) -> Result<Vec<u8>, String> {
    let lower = file.to_ascii_lowercase();
    let bytes = match lower.ends_with(".px4") || lower.ends_with(".apj") {
        true => crate::bootloader::parse_px4(&String::from_utf8_lossy(contents), board.board_id)?.bytes,
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
pub const FIND_BOARD_ATTEMPTS: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sighting {
    Absent,
    Running,
    Bootloader,
}

pub fn in_bootloader(description: &str) -> bool {
    description.contains("BL") || description.to_ascii_lowercase().contains("bootloader")
}

pub fn wait_for_bootloader(look: &mut dyn FnMut() -> Sighting, pause: &mut dyn FnMut(), report: &mut dyn FnMut(Event)) -> Result<(), String> {
    let first = look();
    if first == Sighting::Bootloader {
        return Ok(());
    }
    let mut must_leave = first == Sighting::Running;
    report(Event::Status(if must_leave { REPLUG } else { PLUG_IN }.to_string()));
    let arrived = (0..FIND_BOARD_ATTEMPTS).any(|_| {
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
    if arrived { Ok(()) } else { Err("Bootloader not found".to_string()) }
}

pub fn flash<P: Port>(port: P, file: &str, contents: &[u8], report: &mut dyn FnMut(Event)) -> Result<(), String> {
    let mut loader = Bootloader::new(port);
    let board = loader.board_info()?;
    report(Event::Board(board));
    report(Event::Status("Connected to bootloader:".into()));
    report(Event::Status(format!("  Version: {}", board.bootloader_version)));
    report(Event::Status(format!("  Board ID: {}", board.board_id)));
    report(Event::Status(format!("  Flash size: {}", board.flash_size)));
    let image = image_for(file, contents, &board)?;
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
    Vec::new()
}

#[cfg(target_os = "android")]
fn look_at(_port: &str) -> Sighting {
    Sighting::Absent
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

#[cfg(target_os = "android")]
enum Unopened {}

#[cfg(target_os = "android")]
impl Port for Unopened {
    fn write(&mut self, _bytes: &[u8]) -> Result<(), String> {
        match *self {}
    }
    fn read_exact(&mut self, _count: usize, _timeout_ms: u64) -> Result<Vec<u8>, String> {
        match *self {}
    }
    fn discard_input(&mut self) {
        match *self {}
    }
}

#[cfg(target_os = "android")]
fn open(_port: &str) -> Result<Unopened, String> {
    Err("Flashing over USB serial is not available on this platform yet".to_string())
}

pub fn start(port: &str, file: &str) -> Result<(), String> {
    let contents = std::fs::read(file).map_err(|e| format!("Unable to open firmware file {file}: {e}"))?;
    {
        let mut held = job();
        if held.phase.busy() {
            return Err("A firmware upgrade is already running".to_string());
        }
        *held = Job { phase: Phase::Connecting, port: Some(port.to_string()), file: Some(file.to_string()), ..Job::default() };
    }
    let (port, file) = (port.to_string(), file.to_string());
    std::thread::Builder::new()
        .name("firmware-flash".into())
        .spawn(move || {
            let waited = wait_for_bootloader(&mut || look_at(&port), &mut || std::thread::sleep(std::time::Duration::from_millis(FIND_BOARD_INTERVAL_MS)), &mut apply);
            let outcome = waited.and_then(|()| open(&port)).and_then(|opened| flash(opened, &file, &contents, &mut apply));
            let mut held = job();
            match outcome {
                Ok(()) => {
                    held.phase = Phase::Complete;
                    held.messages.push("Upgrade complete".into());
                }
                Err(e) => {
                    held.phase = Phase::Failed;
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
        let mut pauses = 0;
        let mut lines = Vec::new();
        let outcome = wait_for_bootloader(&mut || queue.next().unwrap_or(Sighting::Absent), &mut || pauses += 1, &mut |e| {
            if let Event::Status(s) = e {
                lines.push(s);
            }
        });
        (outcome, lines, pauses)
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
    fn a_missing_board_is_waited_for_and_then_given_up_on() {
        let (outcome, lines, _) = watched(&[Sighting::Absent, Sighting::Absent, Sighting::Bootloader]);
        assert_eq!((outcome, lines), (Ok(()), vec![PLUG_IN.to_string()]));
        let (outcome, _, pauses) = watched(&[]);
        assert_eq!((outcome, pauses), (Err("Bootloader not found".to_string()), FIND_BOARD_ATTEMPTS));
        assert!(in_bootloader("PX4 BL FMU v5.x") && in_bootloader("ArduPilot Bootloader") && !in_bootloader("PX4 FMU v5.x"));
    }

    #[test]
    fn an_image_bigger_than_the_flash_is_refused_before_erasing() {
        let mut events = Vec::new();
        let outcome = flash(Board::new(5, 50, 256), "fw.bin", &[0u8; 400], &mut |event| events.push(event));
        assert_eq!(outcome, Err("Image size of 400 is too large for board flash size 256".to_string()));
        assert!(!events.iter().any(|e| matches!(e, Event::Phase(Phase::Erasing))));
    }
}
