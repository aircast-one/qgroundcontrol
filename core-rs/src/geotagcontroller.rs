use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use little_exif::exif_tag::ExifTag;
use little_exif::filetype::FileExtension;
use little_exif::metadata::Metadata;
use little_exif::rational::uR64;
use serde_json::{Value, json};

const CONTROLLER_ROOT: &str = "geoTag";
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "tiff", "tif", "dng"];
const LOAD_IMAGES_END: f64 = 20.0;
const PARSE_EXIF_END: f64 = 40.0;
const PARSE_LOGS_END: f64 = 60.0;
const CALIBRATE_END: f64 = 80.0;
const TAG_IMAGES_END: f64 = 100.0;
const CANCELLED: &str = "Tagging cancelled";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trigger {
    pub timestamp: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub success: bool,
}

impl Trigger {
    fn valid(&self) -> bool {
        self.success && (-90.0..=90.0).contains(&self.latitude) && (-180.0..=180.0).contains(&self.longitude)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Controller {
    log_file: String,
    image_directory: String,
    save_directory: String,
    error_message: String,
    progress: f64,
    in_progress: bool,
    tagged: i64,
    skipped: i64,
    failed: i64,
    time_offset_secs: f64,
    tolerance_secs: f64,
    preview_mode: bool,
    recursive_scan: bool,
    run: u64,
}

static CONTROLLER: Mutex<Controller> = Mutex::new(Controller {
    log_file: String::new(),
    image_directory: String::new(),
    save_directory: String::new(),
    error_message: String::new(),
    progress: 0.0,
    in_progress: false,
    tagged: 0,
    skipped: 0,
    failed: 0,
    time_offset_secs: 0.0,
    tolerance_secs: 2.0,
    preview_mode: false,
    recursive_scan: false,
    run: 0,
});

fn held() -> MutexGuard<'static, Controller> {
    CONTROLLER.lock().unwrap_or_else(PoisonError::into_inner)
}

mod dataflash {
    use super::Trigger;
    use std::collections::BTreeMap;

    const HEADER: [u8; 2] = [0xA3, 0x95];
    const FMT_TYPE: u8 = 128;
    const FMT_PAYLOAD: usize = 86;

    #[derive(Debug, Clone)]
    pub struct Format {
        length: usize,
        name: String,
        format: Vec<u8>,
        columns: Vec<String>,
    }

    fn char_size(c: u8) -> usize {
        match c {
            b'b' | b'B' | b'M' => 1,
            b'h' | b'H' | b'c' | b'C' | b'g' => 2,
            b'i' | b'I' | b'e' | b'E' | b'L' | b'f' | b'n' => 4,
            b'd' | b'q' | b'Q' => 8,
            b'N' => 16,
            b'Z' | b'a' => 64,
            _ => 0,
        }
    }

    fn text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(&bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())]).into_owned()
    }

    fn half(bits: u16) -> f64 {
        let sign = u32::from(bits & 0x8000) << 16;
        let exponent = u32::from((bits >> 10) & 0x1F);
        let mantissa = u32::from(bits & 0x3FF);
        let raw = match exponent {
            0 => sign,
            31 => sign | 0x7F80_0000 | (mantissa << 13),
            _ => sign | ((exponent + 112) << 23) | (mantissa << 13),
        };
        f64::from(f32::from_bits(raw))
    }

    fn number(data: &[u8], c: u8) -> Option<f64> {
        let at = |n: usize| data.get(..n);
        Some(match c {
            b'b' => f64::from(*at(1)?.first()? as i8),
            b'B' | b'M' => f64::from(*at(1)?.first()?),
            b'h' => f64::from(i16::from_le_bytes(at(2)?.try_into().ok()?)),
            b'H' => f64::from(u16::from_le_bytes(at(2)?.try_into().ok()?)),
            b'c' => f64::from(i16::from_le_bytes(at(2)?.try_into().ok()?)) / 100.0,
            b'C' => f64::from(u16::from_le_bytes(at(2)?.try_into().ok()?)) / 100.0,
            b'i' => f64::from(i32::from_le_bytes(at(4)?.try_into().ok()?)),
            b'I' => f64::from(u32::from_le_bytes(at(4)?.try_into().ok()?)),
            b'e' => f64::from(i32::from_le_bytes(at(4)?.try_into().ok()?)) / 100.0,
            b'E' => f64::from(u32::from_le_bytes(at(4)?.try_into().ok()?)) / 100.0,
            b'L' => f64::from(i32::from_le_bytes(at(4)?.try_into().ok()?)) / 1.0e7,
            b'f' => f64::from(f32::from_le_bytes(at(4)?.try_into().ok()?)),
            b'd' => f64::from_le_bytes(at(8)?.try_into().ok()?),
            b'q' => i64::from_le_bytes(at(8)?.try_into().ok()?) as f64,
            b'Q' => u64::from_le_bytes(at(8)?.try_into().ok()?) as f64,
            b'g' => half(u16::from_le_bytes(at(2)?.try_into().ok()?)),
            _ => return None,
        })
    }

    fn fields(payload: &[u8], format: &Format) -> BTreeMap<String, f64> {
        format
            .format
            .iter()
            .zip(&format.columns)
            .filter(|(c, _)| char_size(**c) > 0)
            .scan(0usize, |offset, (c, column)| {
                let at = *offset;
                *offset += char_size(*c);
                Some((column, payload.get(at..).and_then(|rest| number(rest, *c))))
            })
            .filter_map(|(column, value)| value.map(|v| (column.clone(), v)))
            .collect()
    }

    fn formats(data: &[u8]) -> BTreeMap<u8, Format> {
        let mut known: BTreeMap<u8, Format> = BTreeMap::new();
        let mut pos = 0usize;
        while pos + 3 <= data.len() {
            if data[pos..pos + 2] != HEADER {
                pos += 1;
                continue;
            }
            let kind = data[pos + 2];
            pos += 3;
            if kind == FMT_TYPE {
                let Some(payload) = data.get(pos..pos + FMT_PAYLOAD) else { break };
                let columns = text(&payload[22..86]).trim().split(',').map(str::to_string).collect();
                known.insert(payload[0], Format { length: usize::from(payload[1]), name: text(&payload[2..6]), format: text(&payload[6..22]).into_bytes(), columns });
                pos += FMT_PAYLOAD;
            } else {
                pos += known.get(&kind).map_or(1, |f| f.length.wrapping_sub(3));
            }
        }
        known
    }

    pub fn triggers(data: &[u8]) -> Result<Vec<Trigger>, String> {
        if data.len() < 3 || data[..2] != HEADER {
            return Err("Invalid DataFlash log format".to_string());
        }
        let known = formats(data);
        if known.is_empty() {
            return Err("No message formats found in log".to_string());
        }
        let Some((&cam, _)) = known.iter().find(|(_, f)| f.name == "CAM") else {
            return Err("No CAM (camera) messages found in log".to_string());
        };
        let mut found = Vec::new();
        let mut pos = 0usize;
        while pos + 3 <= data.len() {
            if data[pos..pos + 2] != HEADER {
                pos += 1;
                continue;
            }
            let kind = data[pos + 2];
            pos += 3;
            let Some(format) = known.get(&kind) else { continue };
            let size = format.length.wrapping_sub(3);
            let Some(payload) = data.get(pos..pos + size) else { break };
            if kind == cam {
                let values = fields(payload, format);
                let value = |key: &str| values.get(key).copied();
                let trigger = Trigger {
                    timestamp: value("TimeUS").map_or(0, |us| (us as u64 / 1_000_000) as i64),
                    latitude: value("Lat").unwrap_or(0.0),
                    longitude: value("Lng").unwrap_or(0.0),
                    altitude: value("Alt").or_else(|| value("GPSAlt")).unwrap_or(0.0),
                    success: true,
                };
                if trigger.valid() {
                    found.push(trigger);
                }
            }
            pos += size;
        }
        match found.is_empty() {
            true => Err("No valid camera capture events found in log".to_string()),
            false => Ok(found),
        }
    }
}

pub fn parse_log(file: &str, data: &[u8]) -> Result<Vec<Trigger>, String> {
    let lower = file.to_lowercase();
    match lower {
        _ if lower.ends_with(".bin") => dataflash::triggers(data),
        _ if lower.ends_with(".ulg") => Err("Could not parse ULog".to_string()),
        _ => dataflash::triggers(data),
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Calibration {
    pub image_indices: Vec<usize>,
    pub trigger_indices: Vec<usize>,
    pub unmatched_images: Vec<usize>,
    pub skipped_triggers: i64,
}

pub fn calibrate(image_timestamps: &[i64], triggers: &[Trigger], time_offset: i64, tolerance: i64) -> Calibration {
    let (Some(last_image), Some(last_trigger)) = (image_timestamps.last(), triggers.last()) else { return Calibration::default() };
    let last_image = last_image + time_offset;
    let offsets: Vec<(i64, usize)> = {
        let unsorted: Vec<(i64, usize)> = image_timestamps.iter().enumerate().filter(|(_, t)| **t != 0).map(|(i, t)| (last_image - (t + time_offset), i)).collect();
        let mut sorted = unsorted;
        sorted.sort_by_key(|(offset, image)| (*offset, std::cmp::Reverse(*image)));
        sorted
    };
    let (pairs, skipped, _) = triggers.iter().enumerate().fold((Vec::new(), 0i64, HashSet::new()), |(pairs, skipped, used), (index, trigger)| {
        if !trigger.valid() {
            return (pairs, skipped + 1, used);
        }
        let wanted = last_trigger.timestamp - trigger.timestamp;
        let best = offsets
            .iter()
            .skip_while(|(offset, _)| *offset < wanted - tolerance)
            .take_while(|(offset, _)| *offset <= wanted + tolerance)
            .filter(|(offset, image)| (offset - wanted).abs() <= tolerance && !used.contains(image))
            .fold(None::<(i64, usize)>, |best, (offset, image)| match best {
                Some((diff, _)) if (offset - wanted).abs() >= diff => best,
                _ => Some(((offset - wanted).abs(), *image)),
            });
        match best {
            Some((_, image)) => ([pairs, vec![(image, index)]].concat(), skipped, used.into_iter().chain([image]).collect()),
            None => (pairs, skipped, used),
        }
    });
    let matched: HashSet<usize> = pairs.iter().map(|(image, _)| *image).collect();
    Calibration {
        image_indices: pairs.iter().map(|(image, _)| *image).collect(),
        trigger_indices: pairs.iter().map(|(_, trigger)| *trigger).collect(),
        unmatched_images: (0..image_timestamps.len()).filter(|i| !matched.contains(i)).collect(),
        skipped_triggers: skipped,
    }
}

fn file_type(bytes: &[u8]) -> Option<FileExtension> {
    FileExtension::auto_detect(&mut std::io::Cursor::new(bytes))
}

pub fn capture_time(image: &[u8]) -> Option<i64> {
    let metadata = Metadata::new_from_vec(&image.to_vec(), file_type(image)?).ok()?;
    let text = metadata.get_tag(&ExifTag::CreateDate(String::new())).find_map(|tag| match tag {
        ExifTag::CreateDate(text) => Some(text.trim_end_matches('\0').to_string()),
        _ => None,
    })?;
    let naive = chrono::NaiveDateTime::parse_from_str(&text, "%Y:%m:%d %H:%M:%S").ok()?;
    Some(naive.and_local_timezone(chrono::Local).earliest()?.timestamp())
}

fn degrees_minutes_seconds(value: f64) -> Vec<uR64> {
    let absolute = value.abs();
    let degrees = absolute as i64;
    let minutes_f = (absolute - degrees as f64) * 60.0;
    let minutes = minutes_f as i64;
    let seconds = (minutes_f - minutes as f64) * 60.0;
    vec![
        uR64 { nominator: degrees as u32, denominator: 1 },
        uR64 { nominator: minutes as u32, denominator: 1 },
        uR64 { nominator: (seconds * 1000.0) as i64 as u32, denominator: 1000 },
    ]
}

pub fn tagged(image: &[u8], trigger: &Trigger) -> Option<Vec<u8>> {
    let kind = file_type(image)?;
    let mut metadata = Metadata::new_from_vec(&image.to_vec(), kind).unwrap_or_else(|_| Metadata::new());
    [
        ExifTag::GPSVersionID(vec![2, 3, 0, 0]),
        ExifTag::GPSLatitudeRef(if trigger.latitude >= 0.0 { "N" } else { "S" }.to_string()),
        ExifTag::GPSLatitude(degrees_minutes_seconds(trigger.latitude)),
        ExifTag::GPSLongitudeRef(if trigger.longitude >= 0.0 { "E" } else { "W" }.to_string()),
        ExifTag::GPSLongitude(degrees_minutes_seconds(trigger.longitude)),
        ExifTag::GPSAltitudeRef(vec![u8::from(trigger.altitude < 0.0)]),
        ExifTag::GPSAltitude(vec![uR64 { nominator: (trigger.altitude.abs() * 100.0) as i64 as u32, denominator: 100 }]),
    ]
    .into_iter()
    .for_each(|tag| metadata.set_tag(tag));
    let mut written = image.to_vec();
    metadata.write_to_vec(&mut written, kind).ok()?;
    Some(written)
}

fn images_in(directory: &Path, recursive: bool) -> Vec<PathBuf> {
    let entries: Vec<PathBuf> = std::fs::read_dir(directory).into_iter().flatten().flatten().map(|e| e.path()).collect();
    let named = |p: &Path| p.file_name().and_then(|n| n.to_str()).map(str::to_string).unwrap_or_default();
    let here = entries.iter().filter(|p| {
        let symlink = std::fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(true);
        let extension = p.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
        p.is_file() && !symlink && !named(p).starts_with('.') && IMAGE_EXTENSIONS.contains(&extension.as_str())
    });
    let below = entries.iter().filter(|p| recursive && p.is_dir() && !named(p).starts_with('.')).flat_map(|d| images_in(d, true));
    let mut found: Vec<PathBuf> = here.cloned().chain(below).collect();
    found.sort_by_key(|p| named(p));
    found
}

fn update(run: u64, change: impl FnOnce(&mut Controller)) -> bool {
    let mut state = held();
    let current = state.run == run && state.in_progress;
    if current {
        change(&mut state);
    }
    current
}

fn fail(run: u64, message: &str) {
    update(run, |state| {
        state.in_progress = false;
        state.error_message = message.to_string();
    });
}

fn stage_progress(start: f64, end: f64, done: usize, total: usize) -> f64 {
    if total == 0 { start } else { start + (end - start) * done as f64 / total as f64 }
}

struct Job {
    log_file: String,
    image_directory: String,
    save_directory: String,
    time_offset: i64,
    tolerance: i64,
    preview: bool,
    recursive: bool,
}

fn tag_run(run: u64, job: &Job) -> Result<(i64, i64, i64), String> {
    let alive = || held().run == run;
    let images = images_in(Path::new(&job.image_directory), job.recursive);
    if images.is_empty() {
        return Err("The image directory doesn't contain supported images. Supported formats: JPEG, TIFF, DNG".to_string());
    }
    update(run, |s| s.progress = LOAD_IMAGES_END);
    let timestamps: Vec<i64> = images
        .iter()
        .enumerate()
        .map(|(i, path)| {
            update(run, |s| s.progress = stage_progress(LOAD_IMAGES_END, PARSE_EXIF_END, i + 1, images.len()));
            std::fs::read(path).ok().and_then(|bytes| capture_time(&bytes)).unwrap_or(0)
        })
        .collect();
    if !alive() {
        return Err(CANCELLED.to_string());
    }
    if timestamps.iter().all(|t| *t == 0) {
        return Err("Could not read EXIF data from any images".to_string());
    }
    update(run, |s| s.progress = PARSE_EXIF_END);
    let log = std::fs::read(&job.log_file).map_err(|_| "Geotagging failed. Couldn't open log file.".to_string())?;
    if log.is_empty() {
        return Err("Geotagging failed. Log file is empty.".to_string());
    }
    let triggers = parse_log(&job.log_file, &log)?;
    update(run, |s| s.progress = PARSE_LOGS_END);
    let calibration = calibrate(&timestamps, &triggers, job.time_offset, job.tolerance);
    let skipped = calibration.skipped_triggers + calibration.unmatched_images.len() as i64;
    if calibration.image_indices.is_empty() {
        return Err("Calibration failed: No matching triggers found for images.".to_string());
    }
    update(run, |s| s.progress = CALIBRATE_END);
    let output = match job.save_directory.is_empty() {
        true => Path::new(&job.image_directory).join("TAGGED"),
        false => PathBuf::from(&job.save_directory),
    };
    if !job.preview && std::fs::create_dir_all(&output).is_err() {
        return Err(format!("Geotagging failed. Couldn't create output directory: {}", output.display()));
    }
    let pairs: Vec<(usize, usize)> = calibration.image_indices.iter().copied().zip(calibration.trigger_indices.iter().copied()).collect();
    let outcomes: Vec<bool> = pairs
        .iter()
        .enumerate()
        .map(|(done, (image, trigger))| {
            update(run, |s| s.progress = stage_progress(CALIBRATE_END, TAG_IMAGES_END, done + 1, pairs.len()));
            let path = &images[*image];
            let written = || -> Option<()> {
                let bytes = std::fs::read(path).ok()?;
                if job.preview {
                    return Some(());
                }
                let geotagged = tagged(&bytes, &triggers[*trigger])?;
                let target = output.join(path.file_name()?);
                let staged = target.with_extension(format!("{}.part", target.extension().and_then(|e| e.to_str()).unwrap_or("")));
                std::fs::write(&staged, geotagged).ok()?;
                std::fs::rename(&staged, &target).ok()
            };
            written().is_some()
        })
        .collect();
    if !alive() {
        return Err(CANCELLED.to_string());
    }
    let failed = outcomes.iter().filter(|ok| !**ok).count() as i64;
    if failed > 0 && failed == outcomes.len() as i64 {
        return Err("All images failed to tag".to_string());
    }
    Ok((pairs.len() as i64 - failed, skipped, failed))
}

pub fn start() {
    let job = {
        let mut state = held();
        if state.in_progress {
            return;
        }
        state.error_message.clear();
        state.progress = 0.0;
        state.tagged = 0;
        state.skipped = 0;
        state.failed = 0;
        let refusal = match () {
            _ if state.image_directory.is_empty() => Some("Please select an image directory."),
            _ if state.log_file.is_empty() => Some("Please select a log file."),
            _ if !Path::new(&state.image_directory).exists() => Some("Cannot find the image directory."),
            _ if !state.save_directory.is_empty() && !Path::new(&state.save_directory).exists() => Some("Cannot find the save directory."),
            _ => None,
        };
        if let Some(message) = refusal {
            state.error_message = message.to_string();
            return;
        }
        state.run += 1;
        state.in_progress = true;
        (
            state.run,
            Job {
                log_file: state.log_file.clone(),
                image_directory: state.image_directory.clone(),
                save_directory: state.save_directory.clone(),
                time_offset: state.time_offset_secs as i64,
                tolerance: state.tolerance_secs as i64,
                preview: state.preview_mode,
                recursive: state.recursive_scan,
            },
        )
    };
    let (run, job) = job;
    std::thread::spawn(move || match tag_run(run, &job) {
        Ok((tagged, skipped, failed)) => {
            update(run, |state| {
                state.in_progress = false;
                state.tagged = tagged;
                state.skipped = skipped;
                state.failed = failed;
                state.progress = 100.0;
                if failed > 0 {
                    state.error_message = format!("{failed} image(s) failed to tag");
                }
            });
        }
        Err(message) => fail(run, &message),
    });
}

pub fn cancel() {
    let mut state = held();
    if state.in_progress {
        state.run += 1;
        state.in_progress = false;
        state.error_message = CANCELLED.to_string();
    }
}

fn set_path(field: &str, raw: &str) {
    let path = crate::geotagjob::local_path(raw);
    let place = Path::new(&path);
    let mut state = held();
    let invalid_directory = || !place.exists() || !place.is_dir();
    state.error_message = match field {
        "logFile" if path.is_empty() => "Empty Filename.".to_string(),
        "logFile" if !place.exists() || !place.is_file() => "Invalid Filename.".to_string(),
        "logFile" => {
            state.log_file = path.clone();
            String::new()
        }
        _ if path.is_empty() || invalid_directory() => "Invalid Directory.".to_string(),
        "imageDirectory" => {
            state.image_directory = path.clone();
            let already = state.save_directory.is_empty() && Path::new(&state.image_directory).join("TAGGED").exists();
            if already { "Images have already been tagged. Existing images will be removed.".to_string() } else { String::new() }
        }
        _ => {
            state.save_directory = path.clone();
            if images_in(place, false).is_empty() { String::new() } else { "The save folder already contains images.".to_string() }
        }
    };
}

fn object() -> Value {
    let state = held().clone();
    json!({
        "kind": "object",
        "class": "GeoTagController",
        "logFile": state.log_file,
        "imageDirectory": state.image_directory,
        "saveDirectory": state.save_directory,
        "errorMessage": state.error_message,
        "progress": state.progress,
        "inProgress": state.in_progress,
        "taggedCount": state.tagged,
        "skippedCount": state.skipped,
        "failedCount": state.failed,
        "timeOffsetSecs": state.time_offset_secs,
        "toleranceSecs": state.tolerance_secs,
        "previewMode": state.preview_mode,
        "recursiveScan": state.recursive_scan,
    })
}

fn served() -> bool {
    !crate::qthost::present()
}

pub fn get(path: &str) -> Option<Value> {
    served().then_some(())?;
    let whole = object();
    match path {
        CONTROLLER_ROOT => Some(whole),
        _ => Some(json!({ "kind": "value", "value": whole.get(path.strip_prefix("geoTag.")?)?.clone() })),
    }
}

pub fn set(path: &str, value: &str) -> Option<Value> {
    served().then_some(())?;
    let field = path.strip_prefix("geoTag.")?;
    let given = serde_json::from_str::<Value>(value).ok().map(|v| v.get("value").cloned().unwrap_or(v))?;
    match (field, &given) {
        ("logFile" | "imageDirectory" | "saveDirectory", Value::String(raw)) => set_path(field, raw),
        ("timeOffsetSecs", _) => held().time_offset_secs = given.as_f64()?,
        ("toleranceSecs", _) => held().tolerance_secs = given.as_f64()?.clamp(0.1, 60.0),
        ("previewMode", Value::Bool(on)) => held().preview_mode = *on,
        ("recursiveScan", Value::Bool(on)) => held().recursive_scan = *on,
        _ => return None,
    }
    Some(json!({ "ok": true }))
}

pub fn invoke(path: &str) -> Option<Value> {
    served().then_some(())?;
    match path {
        "geoTag.startTagging" => start(),
        "geoTag.cancelTagging" => cancel(),
        _ => return None,
    }
    Some(json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger(timestamp: i64) -> Trigger {
        Trigger { timestamp, latitude: -35.0, longitude: 149.0, altitude: 600.0, success: true }
    }

    #[test]
    fn images_and_triggers_are_matched_counting_back_from_the_last_of_each() {
        let images = [1000, 1005, 1007, 1030];
        let triggers = [trigger(57), trigger(62), trigger(64), trigger(87)];
        let matched = calibrate(&images, &triggers, 0, 2);
        assert_eq!((matched.image_indices, matched.trigger_indices), (vec![0, 1, 2, 3], vec![0, 1, 2, 3]));
        let broken = calibrate(&[1000, 1005, 0], &triggers[..2], 0, 2);
        assert!(broken.image_indices.is_empty(), "an image without a capture time that sorts last anchors the images at zero, as GeoTagCalibrator does");
        let tied = calibrate(&[1000, 1000], &[trigger(57)], 0, 2);
        assert_eq!(tied.image_indices, vec![1], "QMultiMap hands back the image inserted last among equal offsets");
        let invalid = calibrate(&images, &[Trigger { latitude: 91.0, ..trigger(57) }, trigger(62)], 0, 2);
        assert_eq!(invalid.skipped_triggers, 1);
    }

    #[test]
    fn coordinates_are_written_as_libexif_rationals() {
        let dms = degrees_minutes_seconds(-35.3632620);
        assert_eq!((dms[0].nominator, dms[1].nominator, dms[2].nominator, dms[2].denominator), (35, 21, 47743, 1000));
    }
}
