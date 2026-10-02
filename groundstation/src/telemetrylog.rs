#![allow(deprecated)]
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use mavlink::dialects::ardupilotmega::MavMessage;

const TEMP_FOLDER: &str = "TelemetryTemp";
const TEMP_EXTENSION: &str = "mavlink";
const SAVED_EXTENSION: &str = "tlog";
const SAVE_NAME_FORMAT: &str = "%Y-%m-%d %H-%M-%S";
const SIGNED_FLAG: u8 = 0x01;
const MAVLINK_V2_STX: u8 = 0xfd;
const ARMED_FLAG: u8 = 128;

#[derive(Default)]
struct Recording {
    open: Option<(PathBuf, File)>,
    vehicle_was_armed: bool,
    suspended: bool,
}

static RECORDING: Mutex<Recording> = Mutex::new(Recording { open: None, vehicle_was_armed: false, suspended: false });

fn setting_bool(path: &str, unset: bool) -> bool {
    crate::settingsstore::raw_setting(path).and_then(|v| v.as_bool()).unwrap_or(unset)
}

fn temp_folder() -> Option<PathBuf> {
    crate::settingsstore::folder().map(|folder| folder.join(TEMP_FOLDER))
}

fn save_folder() -> Option<PathBuf> {
    crate::settingsstore::telemetry_save_path().map(PathBuf::from).filter(|path| !path.as_os_str().is_empty())
}

fn stamped(bytes: &[u8]) -> Vec<u8> {
    let micros = (chrono::Utc::now().timestamp_micros().max(0)) as u64;
    micros.to_be_bytes().into_iter().chain(bytes.iter().copied()).collect()
}

pub fn unsigned_frame(raw: &[u8], header: &mavlink::MavHeader, message: &MavMessage) -> Vec<u8> {
    let signed = raw.first() == Some(&MAVLINK_V2_STX) && raw.get(2).is_some_and(|flags| flags & SIGNED_FLAG != 0);
    match signed {
        false => raw.to_vec(),
        true => {
            let mut buffer = Vec::new();
            match mavlink::write_v2_msg(&mut buffer, *header, message) {
                Ok(_) => buffer,
                Err(_) => raw.to_vec(),
            }
        }
    }
}

fn starts_logging(message: &MavMessage) -> bool {
    matches!(message, MavMessage::HEARTBEAT(_) | MavMessage::HIGH_LATENCY(_) | MavMessage::HIGH_LATENCY2(_))
}

fn start(recording: &mut Recording) {
    if recording.open.is_some() || recording.suspended || !setting_bool("settings.mavlinkSettings.telemetrySave", true) || setting_bool("settings.appSettings.disableAllPersistence", false) {
        return;
    }
    let Some(folder) = temp_folder() else { return };
    if std::fs::create_dir_all(&folder).is_err() {
        return;
    }
    let path = folder.join(format!("FlightData{}.{TEMP_EXTENSION}", chrono::Utc::now().timestamp_micros()));
    match File::create(&path) {
        Ok(file) => {
            recording.open = Some((path, file));
            check_save_path();
        }
        Err(error) => {
            log::warn!("MAVLink Logging failed. Could not open {}: {error}", path.display());
            crate::noticeboard::post(crate::noticeboard::MESSAGE, "MAVLink", &format!("Opening Flight Data file for writing failed. Unable to write to {}. Please choose a different file location.", path.display()));
            recording.suspended = true;
        }
    }
}

fn write(recording: &mut Recording, bytes: &[u8]) {
    let Some((path, file)) = recording.open.as_mut() else { return };
    if file.write_all(&stamped(bytes)).is_err() {
        crate::noticeboard::post(crate::noticeboard::MESSAGE, "MAVLink", &format!("MAVLink Logging failed. Could not write to file {}, logging disabled.", path.display()));
        stop(recording);
        recording.suspended = true;
    }
}

pub fn received(frame: &crate::transport::Frame) {
    if frame.replay {
        return;
    }
    let mut recording = RECORDING.lock().unwrap_or_else(PoisonError::into_inner);
    if !matches!(frame.message, MavMessage::SETUP_SIGNING(_)) {
        let unsigned = unsigned_frame(&frame.raw, &frame.header, &frame.message);
        write(&mut recording, &unsigned);
    }
    if let MavMessage::HEARTBEAT(heartbeat) = &frame.message
        && recording.open.is_some()
        && heartbeat.base_mode.bits() & ARMED_FLAG != 0
    {
        recording.vehicle_was_armed = true;
    }
    if starts_logging(&frame.message) {
        start(&mut recording);
    }
}

pub fn sent(bytes: &[u8]) {
    if crate::logreplay::playing() {
        return;
    }
    let mut recording = RECORDING.lock().unwrap_or_else(PoisonError::into_inner);
    write(&mut recording, bytes);
}

static VEHICLES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn last_vehicle_left(before: usize, count: usize) -> bool {
    before > 0 && count == 0
}

pub fn vehicles(count: usize) {
    if last_vehicle_left(VEHICLES.swap(count, std::sync::atomic::Ordering::Relaxed), count) {
        stop(&mut RECORDING.lock().unwrap_or_else(PoisonError::into_inner));
    }
}

fn stop(recording: &mut Recording) {
    if let Some((path, file)) = recording.open.take() {
        drop(file);
        let empty = std::fs::metadata(&path).map(|m| m.len() == 0).unwrap_or(true);
        let keep = !empty
            && (recording.vehicle_was_armed || setting_bool("settings.mavlinkSettings.telemetrySaveNotArmed", false))
            && setting_bool("settings.mavlinkSettings.telemetrySave", true)
            && !setting_bool("settings.appSettings.disableAllPersistence", false);
        match keep {
            true => save(&path),
            false => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    recording.vehicle_was_armed = false;
}

pub fn saved_name(folder: &Path, now: chrono::DateTime<chrono::Local>) -> PathBuf {
    let stamp = now.format(SAVE_NAME_FORMAT).to_string();
    std::iter::once(format!("{stamp}.{SAVED_EXTENSION}"))
        .chain((1..).map(|index| format!("{stamp}.{index}.{SAVED_EXTENSION}")))
        .map(|name| folder.join(name))
        .find(|candidate| !candidate.exists())
        .unwrap_or_else(|| folder.join(format!("{stamp}.{SAVED_EXTENSION}")))
}

fn check_save_path() {
    let problem = match save_folder() {
        None => Some("Unable to save telemetry log. Application save directory is not set.".to_string()),
        Some(folder) if !folder.is_dir() => Some(format!("Unable to save telemetry log. Telemetry save directory \"{}\" does not exist.", folder.display())),
        Some(_) => None,
    };
    if let Some(problem) = problem {
        crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &problem);
    }
}

fn save(temp: &Path) {
    let Some(folder) = save_folder() else {
        crate::noticeboard::post(crate::noticeboard::MESSAGE, "", "Unable to save telemetry log. Application save directory is not set.");
        let _ = std::fs::remove_file(temp);
        return;
    };
    if !folder.is_dir() {
        crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &format!("Unable to save telemetry log. Telemetry save directory \"{}\" does not exist.", folder.display()));
        let _ = std::fs::remove_file(temp);
        return;
    }
    let target = saved_name(&folder, chrono::Local::now());
    let moved = std::fs::rename(temp, &target).or_else(|_| std::fs::copy(temp, &target).map(|_| ()));
    match moved {
        Ok(()) => log::info!("Telemetry log saved to {}", target.display()),
        Err(error) => {
            crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &format!("Unable to save telemetry log. Error opening destination '{}': '{error}'.", target.display()));
        }
    }
    let _ = std::fs::remove_file(temp);
}

pub fn recover_lost() {
    let Some(folder) = temp_folder() else { return };
    let Ok(entries) = std::fs::read_dir(&folder) else { return };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == TEMP_EXTENSION))
        .for_each(|path| match std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) {
            0 => {
                let _ = std::fs::remove_file(&path);
            }
            _ => save(&path),
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn the_log_stops_only_when_the_last_vehicle_goes_as_mavlink_protocol_does() {
        assert!(last_vehicle_left(1, 0));
        assert!(!last_vehicle_left(0, 0), "a gimbal or GCS heartbeat opens the log without making a vehicle, and that is no vehicle leaving");
        assert!(!last_vehicle_left(1, 2));
    }

    #[test]
    fn saved_logs_are_named_by_time_and_numbered_on_a_clash() {
        let folder = std::env::temp_dir().join(format!("qgc-tlog-names-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let now = chrono::Local.with_ymd_and_hms(2026, 10, 2, 9, 5, 7).unwrap();
        let first = saved_name(&folder, now);
        assert_eq!(first.file_name().unwrap(), "2026-10-02 09-05-07.tlog");
        std::fs::write(&first, b"x").unwrap();
        assert_eq!(saved_name(&folder, now).file_name().unwrap(), "2026-10-02 09-05-07.1.tlog");
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_signed_frame_is_logged_without_its_signature() {
        use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavMessage};
        let header = mavlink::MavHeader { system_id: 1, component_id: 1, sequence: 7 };
        let message = MavMessage::HEARTBEAT(HEARTBEAT_DATA::default());
        let mut plain = Vec::new();
        mavlink::write_v2_msg(&mut plain, header, &message).unwrap();
        assert_eq!(unsigned_frame(&plain, &header, &message), plain, "an unsigned frame is logged as received");
        let mut signed = plain.clone();
        signed[2] |= SIGNED_FLAG;
        signed.extend([0u8; 13]);
        assert_eq!(unsigned_frame(&signed, &header, &message), plain);
    }
}
