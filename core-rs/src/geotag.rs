use std::collections::{BTreeMap, BTreeSet};

use mavlink::dialects::ardupilotmega::{CameraFeedbackFlags, MavMessage};
use serde_json::{Value, json};

use crate::altitude::merge;
use crate::geo;
use crate::router::Backend;
use crate::tlog;

pub const DEPS: &[&str] = &[];

pub const DEFAULT_TOLERANCE_S: f64 = 2.0;
pub const MIN_TOLERANCE_S: f64 = 0.1;
pub const MAX_TOLERANCE_S: f64 = 60.0;

pub const LOAD_IMAGES_END: f64 = 20.0;
pub const PARSE_EXIF_END: f64 = 40.0;
pub const PARSE_LOGS_END: f64 = 60.0;
pub const CALIBRATE_END: f64 = 80.0;
pub const TAG_IMAGES_END: f64 = 100.0;

pub const EPOCH_SCALE_US: u64 = 1_000_000_000_000_000;
const MICROSECONDS_PER_SECOND: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Idle,
    LoadingImages,
    ParsingExif,
    ParsingLogs,
    Calibrating,
    TaggingImages,
}

impl Stage {
    pub fn token(self) -> &'static str {
        match self {
            Stage::Idle => "idle",
            Stage::LoadingImages => "loadingImages",
            Stage::ParsingExif => "parsingExif",
            Stage::ParsingLogs => "parsingLogs",
            Stage::Calibrating => "calibrating",
            Stage::TaggingImages => "taggingImages",
        }
    }

    pub fn span(self) -> Option<(f64, f64)> {
        match self {
            Stage::LoadingImages => Some((0.0, LOAD_IMAGES_END)),
            Stage::ParsingExif => Some((LOAD_IMAGES_END, PARSE_EXIF_END)),
            Stage::ParsingLogs => Some((PARSE_EXIF_END, PARSE_LOGS_END)),
            Stage::Calibrating => Some((PARSE_LOGS_END, CALIBRATE_END)),
            Stage::TaggingImages => Some((CALIBRATE_END, TAG_IMAGES_END)),
            Stage::Idle => None,
        }
    }
}

pub fn stage_progress(stage: Stage, completed: usize, total: usize) -> Option<f64> {
    let (start, end) = stage.span()?;
    match total {
        0 => Some(start),
        total => Some(start + (end - start) * completed.min(total) as f64 / total as f64),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pending,
    Processing,
    Tagged,
    Skipped,
    Failed,
}

impl Status {
    pub fn token(self) -> &'static str {
        match self {
            Status::Pending => "pending",
            Status::Processing => "processing",
            Status::Tagged => "tagged",
            Status::Skipped => "skipped",
            Status::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    ExifTimestampUnreadable,
    NoTriggerWithinTolerance,
    TriggerTakenByCloserImage,
    NoUsableTriggerInLog,
    TriggerTimestampMissing,
    TriggerCoordinateMissing,
    TriggerCaptureFailed,
    TriggerCaptureUnconfirmed,
    ImageUnreadable,
    ExifWriteRefused,
    OutputWriteRefused,
    Cancelled,
}

impl Reason {
    pub fn token(self) -> &'static str {
        match self {
            Reason::ExifTimestampUnreadable => "exifTimestampUnreadable",
            Reason::NoTriggerWithinTolerance => "noTriggerWithinTolerance",
            Reason::TriggerTakenByCloserImage => "triggerTakenByCloserImage",
            Reason::NoUsableTriggerInLog => "noUsableTriggerInLog",
            Reason::TriggerTimestampMissing => "triggerTimestampMissing",
            Reason::TriggerCoordinateMissing => "triggerCoordinateMissing",
            Reason::TriggerCaptureFailed => "triggerCaptureFailed",
            Reason::TriggerCaptureUnconfirmed => "triggerCaptureUnconfirmed",
            Reason::ImageUnreadable => "imageUnreadable",
            Reason::ExifWriteRefused => "exifWriteRefused",
            Reason::OutputWriteRefused => "outputWriteRefused",
            Reason::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    NoImages,
    NoExifTimestamps,
    NoTriggers,
    UnreadableLog,
    NoMatches,
    AllTagsFailed,
    Cancelled,
    OutOfOrder,
}

impl Refusal {
    pub fn token(self) -> &'static str {
        match self {
            Refusal::NoImages => "noImages",
            Refusal::NoExifTimestamps => "noExifTimestamps",
            Refusal::NoTriggers => "noTriggers",
            Refusal::UnreadableLog => "unreadableLog",
            Refusal::NoMatches => "noMatches",
            Refusal::AllTagsFailed => "allTagsFailed",
            Refusal::Cancelled => "cancelled",
            Refusal::OutOfOrder => "outOfOrder",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    NoFeedback,
    Failure,
    Success,
}

impl Capture {
    pub fn token(self) -> &'static str {
        match self {
            Capture::NoFeedback => "noFeedback",
            Capture::Failure => "failure",
            Capture::Success => "success",
        }
    }

    pub fn from_code(code: i8) -> Capture {
        match code {
            1 => Capture::Success,
            0 => Capture::Failure,
            _ => Capture::NoFeedback,
        }
    }
}

pub fn feedback_token(flags: CameraFeedbackFlags) -> &'static str {
    match flags {
        CameraFeedbackFlags::CAMERA_FEEDBACK_PHOTO => "photo",
        CameraFeedbackFlags::CAMERA_FEEDBACK_VIDEO => "video",
        CameraFeedbackFlags::CAMERA_FEEDBACK_BADEXPOSURE => "badExposure",
        CameraFeedbackFlags::CAMERA_FEEDBACK_CLOSEDLOOP => "closedLoop",
        CameraFeedbackFlags::CAMERA_FEEDBACK_OPENLOOP => "openLoop",
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fix {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: Option<f64>,
}

pub fn fix(latitude: f64, longitude: f64, altitude_m: Option<f64>) -> Option<Fix> {
    let on_the_globe = (-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude);
    let located = latitude != 0.0 || longitude != 0.0;
    (on_the_globe && located).then(|| Fix {
        latitude: geo::clamp_latitude(latitude),
        longitude: geo::wrap_longitude(longitude),
        altitude_m: altitude_m.filter(|a| a.is_finite()),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Attitude {
    pub roll_deg: Option<f64>,
    pub pitch_deg: Option<f64>,
    pub yaw_deg: Option<f64>,
}

pub fn attitude(roll_deg: f64, pitch_deg: f64, yaw_deg: f64) -> Attitude {
    Attitude {
        roll_deg: roll_deg.is_finite().then(|| geo::wrap_tilt(roll_deg)),
        pitch_deg: pitch_deg.is_finite().then(|| geo::wrap_tilt(pitch_deg)),
        yaw_deg: yaw_deg.is_finite().then(|| geo::wrap_bearing(yaw_deg)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trigger {
    pub timestamp_s: Option<i64>,
    pub timestamp_utc_s: Option<i64>,
    pub sequence: u32,
    pub fix: Option<Fix>,
    pub ground_distance_m: Option<f64>,
    pub attitude: Attitude,
    pub capture: Capture,
    pub feedback: Option<&'static str>,
}

impl Default for Trigger {
    fn default() -> Self {
        Trigger {
            timestamp_s: None,
            timestamp_utc_s: None,
            sequence: 0,
            fix: None,
            ground_distance_m: None,
            attitude: Attitude::default(),
            capture: Capture::NoFeedback,
            feedback: None,
        }
    }
}

impl Trigger {
    pub fn unusable_reason(&self) -> Option<Reason> {
        match (self.timestamp_s, self.fix, self.capture) {
            (None, _, _) => Some(Reason::TriggerTimestampMissing),
            (_, None, _) => Some(Reason::TriggerCoordinateMissing),
            (_, _, Capture::Failure) => Some(Reason::TriggerCaptureFailed),
            (_, _, Capture::NoFeedback) => Some(Reason::TriggerCaptureUnconfirmed),
            (Some(_), Some(_), Capture::Success) => None,
        }
    }
}

pub fn split_time(time_us: u64, record_us: u64) -> (Option<i64>, Option<i64>) {
    let seconds = |us: u64| (us / MICROSECONDS_PER_SECOND) as i64;
    match (time_us, record_us) {
        (0, 0) => (None, None),
        (0, record) if record >= EPOCH_SCALE_US => (Some(seconds(record)), Some(seconds(record))),
        (0, record) => (Some(seconds(record)), None),
        (time, _) if time >= EPOCH_SCALE_US => (Some(seconds(time)), Some(seconds(time))),
        (time, _) => (Some(seconds(time)), None),
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Log {
    pub triggers: Vec<Trigger>,
    pub undecodable_frames: usize,
}

pub fn triggers_from_tlog(bytes: &[u8]) -> Log {
    triggers_from_tlog_at(bytes, crate::hub::now_us())
}

pub fn triggers_from_tlog_at(bytes: &[u8], now_us: u64) -> Log {
    let mut triggers = Vec::new();
    let undecodable_frames = tlog::for_each(bytes, |raw_record_us, _header, message| {
        let record_us = tlog::parse_timestamp(raw_record_us.to_be_bytes(), now_us);
        match message {
            MavMessage::CAMERA_FEEDBACK(data) => {
                let (timestamp_s, timestamp_utc_s) = split_time(data.time_usec, record_us);
                triggers.push(Trigger {
                    timestamp_s,
                    timestamp_utc_s,
                    sequence: data.img_idx as u32,
                    fix: fix(data.lat as f64 / 1.0e7, data.lng as f64 / 1.0e7, Some(data.alt_msl as f64)),
                    ground_distance_m: data.alt_rel.is_finite().then_some(data.alt_rel as f64),
                    attitude: attitude(data.roll as f64, data.pitch as f64, data.yaw as f64),
                    capture: Capture::Success,
                    feedback: Some(feedback_token(data.flags)),
                });
            }
            MavMessage::CAMERA_TRIGGER(data) => {
                let (timestamp_s, timestamp_utc_s) = split_time(data.time_usec, record_us);
                triggers.push(Trigger { timestamp_s, timestamp_utc_s, sequence: data.seq, ..Trigger::default() });
            }
            _ => {}
        }
    });
    Log { triggers, undecodable_frames }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    pub image: usize,
    pub trigger: usize,
    pub apart_s: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unmatched {
    pub image: usize,
    pub reason: Reason,
    pub nearest_apart_s: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Calibration {
    pub pairs: Vec<Pair>,
    pub unmatched_images: Vec<Unmatched>,
    pub unusable_triggers: Vec<(usize, Reason)>,
    pub clock_skew_s: Option<i64>,
    pub widest_apart_s: Option<i64>,
}

pub fn clamp_tolerance(seconds: f64) -> f64 {
    match seconds.is_finite() {
        true => seconds.clamp(MIN_TOLERANCE_S, MAX_TOLERANCE_S),
        false => DEFAULT_TOLERANCE_S,
    }
}

pub fn whole_seconds(seconds: f64) -> i64 {
    match seconds.is_finite() {
        true => seconds.trunc() as i64,
        false => 0,
    }
}

fn blind(image_timestamps: &[Option<i64>], reason: Reason) -> Vec<Unmatched> {
    image_timestamps
        .iter()
        .enumerate()
        .map(|(image, timestamp)| Unmatched {
            image,
            reason: match timestamp {
                None => Reason::ExifTimestampUnreadable,
                Some(_) => reason,
            },
            nearest_apart_s: None,
        })
        .collect()
}

pub fn calibrate(image_timestamps: &[Option<i64>], triggers: &[Trigger], tolerance_s: i64) -> Calibration {
    let unusable_triggers: Vec<(usize, Reason)> =
        triggers.iter().enumerate().filter_map(|(index, trigger)| trigger.unusable_reason().map(|reason| (index, reason))).collect();

    let last_image = image_timestamps.iter().rev().flatten().next().copied();
    let last_trigger = triggers.iter().rev().filter(|t| t.unusable_reason().is_none()).filter_map(|t| t.timestamp_s).next();
    let (Some(last_image), Some(last_trigger)) = (last_image, last_trigger) else {
        return Calibration {
            pairs: Vec::new(),
            unmatched_images: blind(image_timestamps, Reason::NoUsableTriggerInLog),
            unusable_triggers,
            clock_skew_s: last_image.zip(last_trigger).map(|(image, trigger)| image.saturating_sub(trigger)),
            widest_apart_s: None,
        };
    };

    let by_offset: BTreeMap<i64, Vec<usize>> = image_timestamps.iter().enumerate().filter_map(|(index, ts)| Some((index, (*ts)?))).fold(
        BTreeMap::new(),
        |mut map, (index, timestamp)| {
            map.entry(last_image.saturating_sub(timestamp)).or_default().push(index);
            map
        },
    );

    let tolerance = tolerance_s.max(0);
    let wanted: Vec<(usize, i64)> = triggers
        .iter()
        .enumerate()
        .filter(|(_, trigger)| trigger.unusable_reason().is_none())
        .map(|(index, trigger)| (index, last_trigger.saturating_sub(trigger.timestamp_s.unwrap_or_default())))
        .collect();

    let (pairs, used) = wanted.iter().fold((Vec::new(), BTreeSet::new()), |(mut pairs, mut used), (trigger_index, wanted)| {
        let best = by_offset
            .range(wanted.saturating_sub(tolerance)..=wanted.saturating_add(tolerance))
            .flat_map(|(offset, images)| images.iter().map(move |image| (offset.saturating_sub(*wanted).saturating_abs(), *image)))
            .filter(|(_, image)| !used.contains(image))
            .min();
        if let Some((apart_s, image)) = best {
            pairs.push(Pair { image, trigger: *trigger_index, apart_s });
            used.insert(image);
        }
        (pairs, used)
    });

    let nearest = |offset: i64| wanted.iter().map(|(_, wanted)| offset.saturating_sub(*wanted).saturating_abs()).min();
    let unmatched_images = image_timestamps
        .iter()
        .enumerate()
        .filter(|(index, _)| !used.contains(index))
        .map(|(image, timestamp)| match timestamp {
            None => Unmatched { image, reason: Reason::ExifTimestampUnreadable, nearest_apart_s: None },
            Some(timestamp) => {
                let offset = last_image.saturating_sub(*timestamp);
                let nearest_apart_s = nearest(offset);
                let reason = match nearest_apart_s {
                    Some(apart) if apart <= tolerance => Reason::TriggerTakenByCloserImage,
                    _ => Reason::NoTriggerWithinTolerance,
                };
                Unmatched { image, reason, nearest_apart_s }
            }
        })
        .collect();

    let widest_apart_s = pairs.iter().map(|pair| pair.apart_s).max();
    Calibration { pairs, unmatched_images, unusable_triggers, clock_skew_s: Some(last_image.saturating_sub(last_trigger)), widest_apart_s }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Balance {
    Balanced,
    MissingTriggers(usize),
    MissingImages(usize),
}

impl Balance {
    pub fn token(self) -> &'static str {
        match self {
            Balance::Balanced => "balanced",
            Balance::MissingTriggers(_) => "missingTriggers",
            Balance::MissingImages(_) => "missingImages",
        }
    }

    pub fn count(self) -> usize {
        match self {
            Balance::Balanced => 0,
            Balance::MissingTriggers(count) | Balance::MissingImages(count) => count,
        }
    }
}

pub fn balance(images: usize, usable_triggers: usize) -> Balance {
    match images.cmp(&usable_triggers) {
        std::cmp::Ordering::Greater => Balance::MissingTriggers(images - usable_triggers),
        std::cmp::Ordering::Less => Balance::MissingImages(usable_triggers - images),
        std::cmp::Ordering::Equal => Balance::Balanced,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    pub south_deg: f64,
    pub north_deg: f64,
    pub west_deg: f64,
    pub east_deg: f64,
    pub longitude_span_deg: f64,
}

pub fn extent(fixes: &[Fix]) -> Option<Extent> {
    let first = fixes.first()?;
    let south_deg = fixes.iter().map(|f| f.latitude).fold(first.latitude, f64::min);
    let north_deg = fixes.iter().map(|f| f.latitude).fold(first.latitude, f64::max);
    let longitudes: Vec<f64> = {
        let mut sorted: Vec<f64> = fixes.iter().map(|f| geo::wrap_longitude(f.longitude)).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        sorted
    };
    let gaps: Vec<(f64, usize)> = longitudes
        .windows(2)
        .enumerate()
        .map(|(index, pair)| (pair[1] - pair[0], index + 1))
        .chain(std::iter::once((longitudes[0] + geo::FULL_TURN_DEGREES - longitudes[longitudes.len() - 1], 0)))
        .collect();
    let (widest_gap, after) = gaps.iter().copied().fold((f64::NEG_INFINITY, 0usize), |best, gap| if gap.0 > best.0 { gap } else { best });
    let before = (after + longitudes.len() - 1) % longitudes.len();
    Some(Extent {
        south_deg,
        north_deg,
        west_deg: longitudes[after],
        east_deg: longitudes[before],
        longitude_span_deg: (geo::FULL_TURN_DEGREES - widest_gap).max(0.0),
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageOutcome {
    pub status: Status,
    pub reason: Option<Reason>,
    pub trigger: Option<usize>,
    pub fix: Option<Fix>,
    pub apart_s: Option<i64>,
}

impl Default for ImageOutcome {
    fn default() -> Self {
        ImageOutcome { status: Status::Pending, reason: None, trigger: None, fix: None, apart_s: None }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub images: Vec<ImageOutcome>,
    pub tagged: usize,
    pub failed: usize,
    pub skipped: usize,
    pub pending: usize,
    pub unusable_triggers: Vec<(usize, Reason)>,
    pub balance: Balance,
    pub extent: Option<Extent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    stage: Stage,
    cancelled: bool,
    completed: bool,
    refusal: Option<Refusal>,
    tolerance_s: f64,
    loaded: usize,
    exif_seen: BTreeSet<usize>,
    timestamps: Vec<Option<i64>>,
    triggers: Vec<Trigger>,
    undecodable_frames: usize,
    matched: usize,
    outcomes: Vec<ImageOutcome>,
    unusable_triggers: Vec<(usize, Reason)>,
    clock_skew_s: Option<i64>,
    widest_apart_s: Option<i64>,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            stage: Stage::Idle,
            cancelled: false,
            completed: false,
            refusal: None,
            tolerance_s: DEFAULT_TOLERANCE_S,
            loaded: 0,
            exif_seen: BTreeSet::new(),
            timestamps: Vec::new(),
            triggers: Vec::new(),
            undecodable_frames: 0,
            matched: 0,
            outcomes: Vec::new(),
            unusable_triggers: Vec::new(),
            clock_skew_s: None,
            widest_apart_s: None,
        }
    }
}

impl Session {
    pub fn new() -> Self {
        Session::default()
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn is_busy(&self) -> bool {
        self.stage != Stage::Idle
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn is_completed(&self) -> bool {
        self.completed
    }

    pub fn refusal(&self) -> Option<Refusal> {
        self.refusal
    }

    pub fn tolerance_seconds(&self) -> f64 {
        self.tolerance_s
    }

    pub fn set_tolerance_seconds(&mut self, seconds: f64) {
        self.tolerance_s = clamp_tolerance(seconds);
    }

    pub fn clock_skew_seconds(&self) -> Option<i64> {
        self.clock_skew_s
    }

    pub fn widest_apart_seconds(&self) -> Option<i64> {
        self.widest_apart_s
    }

    pub fn percent(&self) -> Option<f64> {
        let done = |wanted: Status| self.outcomes.iter().filter(|o| o.status == wanted).count();
        match (self.completed, self.stage) {
            (true, _) => Some(TAG_IMAGES_END),
            (_, Stage::LoadingImages) => stage_progress(self.stage, self.loaded, self.timestamps.len()),
            (_, Stage::ParsingExif) => stage_progress(self.stage, self.exif_seen.len(), self.timestamps.len()),
            (_, Stage::TaggingImages) => stage_progress(self.stage, done(Status::Tagged) + done(Status::Failed), self.matched),
            (_, Stage::ParsingLogs | Stage::Calibrating) => stage_progress(self.stage, 0, 0),
            (_, Stage::Idle) => None,
        }
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.stage = Stage::Idle;
        self.refusal = Some(Refusal::Cancelled);
    }

    pub fn start(&mut self, image_count: usize) -> Result<Stage, Refusal> {
        match image_count {
            0 => Err(self.refuse(Refusal::NoImages)),
            count => {
                *self = Session { tolerance_s: self.tolerance_s, timestamps: vec![None; count], outcomes: vec![ImageOutcome::default(); count], stage: Stage::LoadingImages, ..Session::default() };
                Ok(self.stage)
            }
        }
    }

    pub fn record_loaded(&mut self, images: usize) {
        self.loaded = images.min(self.timestamps.len());
    }

    pub fn finish_loading(&mut self) -> Result<Stage, Refusal> {
        self.guard(Stage::LoadingImages)?;
        self.loaded = self.timestamps.len();
        self.stage = Stage::ParsingExif;
        Ok(self.stage)
    }

    pub fn record_exif(&mut self, image: usize, timestamp_s: Option<i64>) {
        if self.cancelled {
            return;
        }
        let Some(slot) = self.timestamps.get_mut(image) else { return };
        *slot = timestamp_s;
        self.exif_seen.insert(image);
        if timestamp_s.is_none() {
            self.outcomes[image] = ImageOutcome { status: Status::Skipped, reason: Some(Reason::ExifTimestampUnreadable), ..ImageOutcome::default() };
        }
    }

    pub fn finish_exif(&mut self) -> Result<Stage, Refusal> {
        self.guard(Stage::ParsingExif)?;
        if self.timestamps.iter().all(Option::is_none) {
            return Err(self.refuse(Refusal::NoExifTimestamps));
        }
        self.stage = Stage::ParsingLogs;
        Ok(self.stage)
    }

    pub fn set_triggers(&mut self, log: Log) -> Result<Stage, Refusal> {
        self.guard(Stage::ParsingLogs)?;
        self.undecodable_frames = log.undecodable_frames;
        match (log.triggers.is_empty(), log.undecodable_frames) {
            (true, 0) => Err(self.refuse(Refusal::NoTriggers)),
            (true, _) => Err(self.refuse(Refusal::UnreadableLog)),
            (false, _) => {
                self.triggers = log.triggers;
                self.stage = Stage::Calibrating;
                Ok(self.stage)
            }
        }
    }

    pub fn calibrate(&mut self) -> Result<Calibration, Refusal> {
        self.guard(Stage::Calibrating)?;
        let calibration = calibrate(&self.timestamps, &self.triggers, whole_seconds(self.tolerance_s));
        self.unusable_triggers = calibration.unusable_triggers.clone();
        self.clock_skew_s = calibration.clock_skew_s;
        self.widest_apart_s = calibration.widest_apart_s;
        self.matched = calibration.pairs.len();
        calibration.unmatched_images.iter().for_each(|unmatched| {
            self.outcomes[unmatched.image] =
                ImageOutcome { status: Status::Skipped, reason: Some(unmatched.reason), apart_s: unmatched.nearest_apart_s, ..ImageOutcome::default() };
        });
        calibration.pairs.iter().for_each(|pair| {
            self.outcomes[pair.image] = ImageOutcome {
                status: Status::Processing,
                reason: None,
                trigger: Some(pair.trigger),
                fix: self.triggers[pair.trigger].fix,
                apart_s: Some(pair.apart_s),
            };
        });
        match calibration.pairs.is_empty() {
            true => Err(self.refuse(Refusal::NoMatches)),
            false => {
                self.stage = Stage::TaggingImages;
                Ok(calibration)
            }
        }
    }

    pub fn record_tag(&mut self, image: usize, outcome: Result<(), Reason>) {
        if self.cancelled {
            return;
        }
        let Some(current) = self.outcomes.get(image).copied() else { return };
        self.outcomes[image] = match outcome {
            Ok(()) => ImageOutcome { status: Status::Tagged, reason: None, ..current },
            Err(reason) => ImageOutcome { status: Status::Failed, reason: Some(reason), ..current },
        };
    }

    pub fn finish(&mut self) -> Result<Report, Refusal> {
        self.guard(Stage::TaggingImages)?;
        let report = self.report();
        if report.tagged == 0 && report.failed > 0 {
            return Err(self.refuse(Refusal::AllTagsFailed));
        }
        self.stage = Stage::Idle;
        self.completed = true;
        Ok(report)
    }

    pub fn report(&self) -> Report {
        let count = |wanted: Status| self.outcomes.iter().filter(|o| o.status == wanted).count();
        Report {
            images: self.outcomes.clone(),
            tagged: count(Status::Tagged),
            failed: count(Status::Failed),
            skipped: count(Status::Skipped),
            pending: count(Status::Pending) + count(Status::Processing),
            unusable_triggers: self.unusable_triggers.clone(),
            balance: balance(self.outcomes.len(), self.triggers.len() - self.unusable_triggers.len()),
            extent: extent(&self.outcomes.iter().filter(|o| o.status == Status::Tagged).filter_map(|o| o.fix).collect::<Vec<_>>()),
        }
    }

    pub fn snapshot(&self) -> Value {
        let report = self.report();
        json!({
            "kind": "object",
            "class": "GeoTag",
            "stage": self.stage.token(),
            "percent": self.percent(),
            "busy": self.is_busy(),
            "cancelled": self.cancelled,
            "completed": self.completed,
            "refusal": self.refusal.map(Refusal::token),
            "toleranceSeconds": self.tolerance_s,
            "toleranceWholeSeconds": whole_seconds(self.tolerance_s),
            "clockSkewSeconds": self.clock_skew_s,
            "widestApartSeconds": self.widest_apart_s,
            "imageCount": self.outcomes.len(),
            "triggerCount": self.triggers.len(),
            "usableTriggerCount": self.triggers.len() - self.unusable_triggers.len(),
            "undecodableFrames": self.undecodable_frames,
            "matchedCount": self.matched,
            "tagged": report.tagged,
            "failed": report.failed,
            "skipped": report.skipped,
            "pending": report.pending,
            "balance": report.balance.token(),
            "balanceCount": report.balance.count(),
            "images": report.images.iter().map(outcome_json).collect::<Vec<_>>(),
            "triggers": self.triggers.iter().enumerate().map(|(index, trigger)| trigger_json(index, trigger)).collect::<Vec<_>>(),
            "unusableTriggers": report.unusable_triggers.iter().map(|(index, reason)| json!({ "index": index, "reason": reason.token() })).collect::<Vec<_>>(),
            "extent": report.extent.map(extent_json),
        })
    }

    fn refuse(&mut self, refusal: Refusal) -> Refusal {
        self.refusal = Some(refusal);
        self.stage = Stage::Idle;
        refusal
    }

    fn guard(&mut self, wanted: Stage) -> Result<(), Refusal> {
        match (self.cancelled, self.stage == wanted) {
            (true, _) => Err(self.refuse(Refusal::Cancelled)),
            (false, false) => Err(self.refuse(Refusal::OutOfOrder)),
            (false, true) => Ok(()),
        }
    }
}

fn outcome_json(outcome: &ImageOutcome) -> Value {
    json!({
        "status": outcome.status.token(),
        "reason": outcome.reason.map(Reason::token),
        "trigger": outcome.trigger,
        "apartSeconds": outcome.apart_s,
        "latitude": outcome.fix.map(|f| f.latitude),
        "longitude": outcome.fix.map(|f| f.longitude),
        "altitudeMeters": outcome.fix.and_then(|f| f.altitude_m),
    })
}

fn trigger_json(index: usize, trigger: &Trigger) -> Value {
    json!({
        "index": index,
        "sequence": trigger.sequence,
        "timestampSeconds": trigger.timestamp_s,
        "timestampUtcSeconds": trigger.timestamp_utc_s,
        "timeBase": match trigger.timestamp_utc_s {
            Some(_) => "epoch",
            None => "sinceBoot",
        },
        "latitude": trigger.fix.map(|f| f.latitude),
        "longitude": trigger.fix.map(|f| f.longitude),
        "altitudeMeters": trigger.fix.and_then(|f| f.altitude_m),
        "groundDistanceMeters": trigger.ground_distance_m,
        "rollDegrees": trigger.attitude.roll_deg,
        "pitchDegrees": trigger.attitude.pitch_deg,
        "yawDegrees": trigger.attitude.yaw_deg,
        "capture": trigger.capture.token(),
        "feedback": trigger.feedback,
        "unusableReason": trigger.unusable_reason().map(Reason::token),
    })
}

fn extent_json(e: Extent) -> Value {
    json!({
        "southDegrees": e.south_deg,
        "northDegrees": e.north_deg,
        "westDegrees": e.west_deg,
        "eastDegrees": e.east_deg,
        "longitudeSpanDegrees": e.longitude_span_deg,
    })
}

fn parse_image_timestamps(args: &[String]) -> Vec<Option<i64>> {
    args.iter().map(|arg| arg.trim().parse::<i64>().ok()).collect()
}

fn run(log: Log, tolerance_s: f64, timestamps: &[Option<i64>]) -> Session {
    let mut session = Session::new();
    session.set_tolerance_seconds(tolerance_s);
    if session.start(timestamps.len()).is_err() {
        return session;
    }
    let _ = session.finish_loading();
    timestamps.iter().enumerate().for_each(|(image, timestamp)| session.record_exif(image, *timestamp));
    let _ = session.finish_exif().and_then(|_| session.set_triggers(log));
    let _ = session.calibrate();
    session
}

pub fn geotag_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else {
        return crate::read::refused("this needs the path of a telemetry log, then optionally a tolerance in seconds and one image timestamp in epoch seconds per image (a dash for an image whose EXIF time is unreadable)");
    };
    let Ok(bytes) = std::fs::read(path) else { return json!({ "kind": "object", "class": "GeoTag", "path": path, "readable": false }) };
    let log = triggers_from_tlog(&bytes);
    let tolerance_s = clamp_tolerance(args.get(1).and_then(|a| a.parse::<f64>().ok()).unwrap_or(DEFAULT_TOLERANCE_S));
    let timestamps = parse_image_timestamps(args.get(2..).unwrap_or_default());
    merge(run(log, tolerance_s, &timestamps).snapshot(), json!({ "path": path, "readable": true, "bytes": bytes.len() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(timestamp_s: i64, latitude: f64, longitude: f64) -> Trigger {
        Trigger {
            timestamp_s: Some(timestamp_s),
            timestamp_utc_s: Some(timestamp_s),
            sequence: timestamp_s as u32,
            fix: fix(latitude, longitude, Some(100.0)),
            ground_distance_m: Some(50.0),
            attitude: attitude(0.0, -30.0, 90.0),
            capture: Capture::Success,
            feedback: Some("closedLoop"),
        }
    }

    fn log(triggers: Vec<Trigger>) -> Log {
        Log { triggers, undecodable_frames: 0 }
    }

    fn calibrated(images: &[Option<i64>], triggers: &[Trigger]) -> Calibration {
        calibrate(images, triggers, whole_seconds(DEFAULT_TOLERANCE_S))
    }

    fn reasons(calibration: &Calibration) -> Vec<(usize, Reason)> {
        calibration.unmatched_images.iter().map(|u| (u.image, u.reason)).collect()
    }

    struct Nothing;

    impl Backend for Nothing {
        fn get(&self, _path: &str) -> String {
            String::new()
        }

        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            String::new()
        }

        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }

        fn invoke(&self, _path: &str, _args: &str) -> String {
            String::new()
        }

        fn watch(&self, _paths: &[String]) {}
    }

    fn walked(images: &[Option<i64>], triggers: Vec<Trigger>) -> Session {
        let mut session = Session::new();
        session.start(images.len()).expect("images to work through");
        session.finish_loading().expect("the load stage to close");
        images.iter().enumerate().for_each(|(image, timestamp)| session.record_exif(image, *timestamp));
        session.finish_exif().expect("a readable timestamp");
        session.set_triggers(log(triggers)).expect("triggers");
        session
    }

    #[test]
    fn matching_normalises_both_sequences_to_an_offset_from_their_own_end() {
        let triggers = [shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1), shot(1020, 47.2, 8.2)];
        let images = [Some(50_000), Some(50_010), Some(50_020)];
        let calibration = calibrated(&images, &triggers);
        assert_eq!(
            calibration.pairs,
            vec![
                Pair { image: 0, trigger: 0, apart_s: 0 },
                Pair { image: 1, trigger: 1, apart_s: 0 },
                Pair { image: 2, trigger: 2, apart_s: 0 }
            ],
            "the camera clock and the log clock share no epoch, so only the distance from the end of each sequence may decide a match"
        );
        assert!(calibration.unmatched_images.is_empty() && calibration.unusable_triggers.is_empty());
    }

    #[test]
    fn the_clock_skew_between_the_two_sequences_is_reported_instead_of_an_offset_knob() {
        let triggers = [shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1)];
        let images = [Some(50_000), Some(50_010)];
        let calibration = calibrated(&images, &triggers);
        assert_eq!(
            calibration.clock_skew_s,
            Some(49_000),
            "the operator whose camera clock is hours off the log clock needs the measured skew, which is the one number an offset knob could never show them"
        );
        assert_eq!(calibration.widest_apart_s, Some(0), "and the widest gap across the matched pairs is how they judge whether the tolerance is wide enough");
        let drifted = calibrate(&[Some(50_000), Some(50_008)], &triggers, 5);
        assert_eq!(drifted.widest_apart_s, Some(2), "a two second drift on one pair is reported as the widest, so a head can say how much slack the run actually needed");
        assert_eq!(
            calibrated(&[None], &triggers).clock_skew_s,
            None,
            "with no readable image time there is no skew to measure, which is not a skew of nought"
        );
    }

    #[test]
    fn a_tolerance_admits_drift_and_the_closest_image_wins_with_the_lowest_index_breaking_a_tie() {
        let triggers = [shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1)];
        let drifted = [Some(50_000), Some(50_009), Some(50_010)];
        assert_eq!(
            calibrate(&drifted, &triggers, 2).pairs,
            vec![Pair { image: 0, trigger: 0, apart_s: 0 }, Pair { image: 2, trigger: 1, apart_s: 0 }],
            "two images sit inside the second trigger's window and the closer one takes it, leaving the drifted one unmatched"
        );
        let beyond = [Some(50_000), Some(50_010)];
        assert_eq!(
            calibrate(&beyond, &[shot(1000, 47.0, 8.0), shot(1007, 47.1, 8.1)], 2).pairs,
            vec![Pair { image: 1, trigger: 1, apart_s: 0 }],
            "three seconds of drift is outside a two second tolerance and must not be matched anyway"
        );
        let tied = [Some(50_000), Some(50_010), Some(50_010)];
        assert_eq!(
            calibrate(&tied, &[shot(1000, 47.0, 8.0)], 2).pairs,
            vec![Pair { image: 1, trigger: 0, apart_s: 0 }],
            "two images stamped at the same second are equally close, so the lower index wins and the ordering of the answer stays the same on every run, which is a deliberate divergence from the Qt QMultiMap walk that landed on the higher index"
        );
    }

    #[test]
    fn an_image_is_matched_at_most_once_and_a_second_trigger_falls_through() {
        let triggers = [shot(1000, 47.0, 8.0), shot(1000, 47.1, 8.1)];
        let images = [Some(50_000)];
        let calibration = calibrated(&images, &triggers);
        assert_eq!(calibration.pairs, vec![Pair { image: 0, trigger: 0, apart_s: 0 }]);
        assert!(calibration.unmatched_images.is_empty());
        assert_eq!(
            balance(images.len(), triggers.len()),
            Balance::MissingImages(1),
            "two triggers for one image means an image frame went missing, and the count is the answer, not a sentence"
        );
        assert_eq!(balance(3, 1), Balance::MissingTriggers(2));
        assert_eq!(balance(2, 2), Balance::Balanced);
        assert_eq!(Balance::Balanced.count(), 0);
    }

    #[test]
    fn balance_counts_only_the_triggers_that_could_have_tagged_an_image() {
        let images = [Some(50_000), Some(50_010)];
        let paired = [shot(1000, 47.0, 8.0), Trigger { fix: None, ..shot(1000, 47.0, 8.0) }, shot(1010, 47.1, 8.1), Trigger { fix: None, ..shot(1010, 47.1, 8.1) }];
        let mut session = walked(&images, paired.to_vec());
        session.calibrate().expect("the two placed triggers match the two images");
        let report = session.report();
        assert_eq!(
            report.balance,
            Balance::Balanced,
            "an ArduPilot log carries a bare trigger beside every camera feedback, and counting those placeless twins told the operator that half their image frames had gone missing when none had"
        );
        assert_eq!(report.unusable_triggers.len(), 2);
        assert_eq!(session.snapshot()["triggerCount"], json!(4), "the raw count stays available beside the usable one, so neither number has to lie");
        assert_eq!(session.snapshot()["usableTriggerCount"], json!(2));
    }

    #[test]
    fn a_missing_exif_time_is_not_a_time_of_zero() {
        let triggers = [shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1)];
        let unreadable = [None, Some(50_010)];
        let calibration = calibrated(&unreadable, &triggers);
        assert_eq!(reasons(&calibration), vec![(0, Reason::ExifTimestampUnreadable)]);
        assert_eq!(calibration.pairs, vec![Pair { image: 1, trigger: 1, apart_s: 0 }]);
        let at_epoch = [Some(0), Some(10)];
        assert_eq!(
            calibrate(&at_epoch, &triggers, 2).pairs.len(),
            2,
            "an image stamped at the epoch is a real time and must still match, which the C++ sentinel of zero could not distinguish"
        );
    }

    #[test]
    fn an_unmatched_image_says_which_of_the_three_causes_kept_it_unmatched() {
        let triggers = [shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1)];
        let far = calibrate(&[Some(50_000), Some(40_000), Some(50_010)], &triggers, 2);
        assert_eq!(
            far.unmatched_images,
            vec![Unmatched { image: 1, reason: Reason::NoTriggerWithinTolerance, nearest_apart_s: Some(10_000) }],
            "an image no trigger could reach says so and says how far the nearest trigger was, which is the number that tells the operator whether widening the tolerance could ever help"
        );
        let crowded = calibrate(&[Some(50_000), Some(50_010), Some(50_010)], &triggers, 2);
        assert_eq!(
            crowded.unmatched_images,
            vec![Unmatched { image: 2, reason: Reason::TriggerTakenByCloserImage, nearest_apart_s: Some(0) }],
            "an image a trigger reached but credited to another frame is not a tolerance problem, and telling the operator to widen the window would be advice that cannot work"
        );
        let placeless = calibrated(&[Some(50_000)], &[Trigger { fix: None, ..shot(1000, 47.0, 8.0) }]);
        assert_eq!(
            placeless.unmatched_images,
            vec![Unmatched { image: 0, reason: Reason::NoUsableTriggerInLog, nearest_apart_s: None }],
            "with no usable trigger in the whole log the tolerance is irrelevant, and calling it a tolerance failure sends the operator to turn a knob that can never match anything"
        );
        assert_eq!(reasons(&calibrated(&[Some(1), None], &[])), vec![(0, Reason::NoUsableTriggerInLog), (1, Reason::ExifTimestampUnreadable)]);
    }

    #[test]
    fn the_end_of_each_sequence_is_the_last_usable_entry_not_the_last_entry() {
        let triggers = [shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1)];
        let trailing_blank = [Some(50_000), Some(50_010), None];
        let calibration = calibrated(&trailing_blank, &triggers);
        assert_eq!(
            calibration.pairs,
            vec![Pair { image: 0, trigger: 0, apart_s: 0 }, Pair { image: 1, trigger: 1, apart_s: 0 }],
            "an unreadable last image must not become the base the whole normalisation is measured from, or every offset shifts and nothing matches"
        );
        let trailing_unusable = [shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1), Trigger { capture: Capture::Failure, ..shot(9999, 47.2, 8.2) }];
        let images = [Some(50_000), Some(50_010)];
        assert_eq!(calibrate(&images, &trailing_unusable, 2).pairs.len(), 2, "and the same holds for a trailing trigger that reported a failed capture");
    }

    #[test]
    fn every_way_a_trigger_can_be_unusable_has_its_own_token() {
        let no_time = Trigger { timestamp_s: None, ..shot(1000, 47.0, 8.0) };
        let no_fix = Trigger { fix: None, ..shot(1000, 47.0, 8.0) };
        let failed = Trigger { capture: Capture::Failure, ..shot(1000, 47.0, 8.0) };
        let unconfirmed = Trigger { capture: Capture::NoFeedback, ..shot(1000, 47.0, 8.0) };
        assert_eq!(no_time.unusable_reason(), Some(Reason::TriggerTimestampMissing));
        assert_eq!(no_fix.unusable_reason(), Some(Reason::TriggerCoordinateMissing));
        assert_eq!(failed.unusable_reason(), Some(Reason::TriggerCaptureFailed));
        assert_eq!(
            unconfirmed.unusable_reason(),
            Some(Reason::TriggerCaptureUnconfirmed),
            "the C++ GeoTagData::isValid gates on captureResult == Success, so a trigger nobody confirmed is a third answer and not a usable one"
        );
        assert_eq!(shot(1000, 47.0, 8.0).unusable_reason(), None);
        let calibration = calibrated(&[Some(50_000)], &[no_fix, failed, unconfirmed, shot(1000, 47.0, 8.0)]);
        assert_eq!(
            calibration.unusable_triggers,
            vec![(0, Reason::TriggerCoordinateMissing), (1, Reason::TriggerCaptureFailed), (2, Reason::TriggerCaptureUnconfirmed)]
        );
        assert_eq!(calibration.pairs, vec![Pair { image: 0, trigger: 3, apart_s: 0 }]);
        let tokens: Vec<&str> = [
            Reason::ExifTimestampUnreadable,
            Reason::NoTriggerWithinTolerance,
            Reason::TriggerTakenByCloserImage,
            Reason::NoUsableTriggerInLog,
            Reason::TriggerTimestampMissing,
            Reason::TriggerCoordinateMissing,
            Reason::TriggerCaptureFailed,
            Reason::TriggerCaptureUnconfirmed,
            Reason::ImageUnreadable,
            Reason::ExifWriteRefused,
            Reason::OutputWriteRefused,
            Reason::Cancelled,
        ]
        .iter()
        .map(|r| r.token())
        .collect();
        assert!(
            tokens.iter().all(|t| !t.contains(' ')) && tokens.iter().collect::<BTreeSet<_>>().len() == tokens.len(),
            "every reason is a distinct token a head can translate, never a sentence: {tokens:?}"
        );
    }

    #[test]
    fn nothing_to_work_with_still_reports_every_image_and_every_trigger() {
        let calibration = calibrated(&[Some(1), None], &[]);
        assert_eq!(calibration.pairs, Vec::new());
        let no_images = calibrated(&[], &[shot(1000, 47.0, 8.0)]);
        assert!(no_images.pairs.is_empty() && no_images.unmatched_images.is_empty() && no_images.unusable_triggers.is_empty());
        assert_eq!(calibrated(&[], &[]), Calibration::default());
    }

    #[test]
    fn the_tolerance_clamp_and_its_truncation_keep_the_cplusplus_numbers() {
        assert_eq!(clamp_tolerance(0.0), MIN_TOLERANCE_S);
        assert_eq!(clamp_tolerance(1000.0), MAX_TOLERANCE_S);
        assert_eq!(clamp_tolerance(2.0), 2.0);
        assert_eq!(clamp_tolerance(f64::NAN), DEFAULT_TOLERANCE_S, "a tolerance that is not a number falls back to the default rather than poisoning every comparison");
        assert_eq!((MIN_TOLERANCE_S, MAX_TOLERANCE_S, DEFAULT_TOLERANCE_S), (0.1, 60.0, 2.0));
        assert_eq!(
            whole_seconds(clamp_tolerance(0.1)),
            0,
            "the C++ casts the clamped tolerance to whole seconds, so the smallest tolerance it allows demands an exact offset; keep the number and show the consequence rather than quietly widening it"
        );
        assert_eq!(whole_seconds(59.9), 59);
        assert_eq!(whole_seconds(-3.0), -3);
        let exact_only = calibrate(&[Some(50_000), Some(50_009)], &[shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1)], whole_seconds(clamp_tolerance(0.1)));
        assert_eq!(exact_only.pairs, vec![Pair { image: 1, trigger: 1, apart_s: 0 }]);
    }

    #[test]
    fn a_negative_tolerance_cannot_widen_the_window_backwards() {
        let triggers = [shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1)];
        assert_eq!(
            calibrate(&[Some(50_000), Some(50_010)], &triggers, -5).pairs.len(),
            2,
            "a negative tolerance must degrade to an exact match, not invert the range bounds and silently match nothing"
        );
    }

    #[test]
    fn matching_survives_a_timestamp_at_the_far_end_of_the_range() {
        let triggers = [shot(1000, 47.0, 8.0)];
        let calibration = calibrate(&[Some(i64::MAX - 10), Some(i64::MIN + 10)], &triggers, 2);
        assert_eq!(
            calibration.pairs,
            vec![Pair { image: 1, trigger: 0, apart_s: 0 }],
            "every subtraction in the matching saturates, so an absurd EXIF timestamp is a missed match and never an overflow panic in debug or a wrapped garbage offset in release"
        );
        assert_eq!(
            calibration.unmatched_images,
            vec![Unmatched { image: 0, reason: Reason::NoTriggerWithinTolerance, nearest_apart_s: Some(i64::MAX) }],
            "and the distance reported for an unreachable image saturates too, rather than negating the smallest integer there is"
        );
    }

    #[test]
    fn a_bearing_wraps_and_a_tilt_folds_into_its_own_half_turn() {
        assert_eq!(geo::wrap_bearing(370.0), 10.0);
        assert_eq!(geo::wrap_bearing(-90.0), 270.0);
        assert_eq!(geo::wrap_tilt(190.0), -170.0);
        let wrapped = attitude(f64::NAN, -30.0, 450.0);
        assert_eq!(
            (wrapped.roll_deg, wrapped.pitch_deg, wrapped.yaw_deg),
            (None, Some(-30.0), Some(90.0)),
            "a camera yaw of 450 degrees is a bearing of 90 and an unreadable roll is absent, not zero"
        );
    }

    #[test]
    fn a_longitude_span_wraps_the_antimeridian() {
        let across = [fix(10.0, 179.0, None).unwrap(), fix(10.0, -179.0, None).unwrap(), fix(12.0, 179.5, None).unwrap()];
        let reach = extent(&across).unwrap();
        assert!((reach.longitude_span_deg - 2.0).abs() < 1.0e-9, "a flight either side of the antimeridian spans two degrees, not the 358 a plain max minus min reports: {reach:?}");
        assert_eq!((reach.west_deg, reach.east_deg), (179.0, -179.0));
        assert_eq!((reach.south_deg, reach.north_deg), (10.0, 12.0));
        let local = [fix(47.0, 8.0, None).unwrap(), fix(47.5, 8.5, None).unwrap()];
        let near = extent(&local).unwrap();
        assert!((near.longitude_span_deg - 0.5).abs() < 1.0e-9);
        assert_eq!((near.west_deg, near.east_deg), (8.0, 8.5));
        assert_eq!(extent(&[]), None, "no matched image is no extent at all, not a box at the null island");
        let single = extent(&[fix(47.0, 8.0, None).unwrap()]).unwrap();
        assert_eq!((single.longitude_span_deg, single.west_deg, single.east_deg), (0.0, 8.0, 8.0));
    }

    #[test]
    fn a_coordinate_is_absent_rather_than_zero_when_the_log_gave_nothing_usable() {
        assert_eq!(fix(f64::NAN, 8.0, None), None);
        assert_eq!(fix(91.0, 8.0, None), None);
        assert_eq!(
            fix(0.0, 0.0, None),
            None,
            "a camera that shot before GPS lock sends nought for latitude and longitude, and tagging those photos at null island hands the operator a results list where a ruined frame is indistinguishable from a good one"
        );
        assert_eq!(fix(0.0, 8.0, None).map(|f| f.latitude), Some(0.0), "the equator is still a real latitude, so only the exact pair of zeroes is the unlocked sentinel");
        assert_eq!(
            fix(47.0, 190.0, None),
            None,
            "the C++ gates on QGeoCoordinate::isValid, which refuses a longitude past the antimeridian rather than wrapping a corrupt field into a confident wrong position"
        );
        assert_eq!(fix(47.0, -181.0, None), None);
        assert_eq!(fix(47.0, 180.0, None).map(|f| f.longitude), Some(180.0), "the dateline itself is in range and stays a longitude");
        assert_eq!(fix(47.0, 8.0, Some(f64::INFINITY)).unwrap().altitude_m, None, "an infinite altitude is no altitude");
        assert_eq!(fix(47.0, 8.0, None).unwrap().altitude_m, None);
        assert_eq!(fix(47.0, 8.0, Some(0.0)).unwrap().altitude_m, Some(0.0), "and sea level is an altitude, so it must survive as one");
    }

    #[test]
    fn an_unlocked_trigger_falls_out_as_a_placeless_one_and_never_stretches_the_extent() {
        let unlocked = Trigger { fix: fix(0.0, 0.0, Some(0.0)), ..shot(1000, 0.0, 0.0) };
        assert_eq!(unlocked.unusable_reason(), Some(Reason::TriggerCoordinateMissing));
        let mut session = walked(&[Some(50_000), Some(50_010)], vec![unlocked, shot(1010, 47.1, 8.1)]);
        session.calibrate().expect("the placed trigger still matches");
        session.record_tag(1, Ok(()));
        let reach = session.report().extent.expect("one tagged image");
        assert!(
            (reach.west_deg - 8.1).abs() < 1.0e-9 && (reach.east_deg - 8.1).abs() < 1.0e-9 && reach.longitude_span_deg < 1.0e-9,
            "a single trigger at nought by nought used to stretch the reported bounds across half the planet, which is a survey area no operator flew: {reach:?}"
        );
    }

    #[test]
    fn a_trigger_timestamp_is_epoch_or_boot_relative_or_absent() {
        assert_eq!(split_time(1_700_000_000_000_000, 0), (Some(1_700_000_000), Some(1_700_000_000)));
        assert_eq!(split_time(42_000_000, 0), (Some(42), None), "a small time_usec is time since boot, so there is no UTC answer to give");
        assert_eq!(split_time(0, 1_700_000_000_000_000), (Some(1_700_000_000), Some(1_700_000_000)), "a camera that reported no time falls back to when the log recorded the frame");
        assert_eq!(split_time(0, 0), (None, None), "and with neither, the answer is no timestamp, not the epoch");
        assert_eq!(split_time(0, 5_000_000), (Some(5), None));
    }

    #[test]
    fn a_capture_result_comes_from_a_ulog_result_code_and_never_from_the_feedback_flags() {
        assert_eq!(Capture::from_code(1), Capture::Success);
        assert_eq!(Capture::from_code(0), Capture::Failure);
        assert_eq!(Capture::from_code(-1), Capture::NoFeedback);
        let feedback = |flags| {
            let frame = frame(camera_feedback(1_700_000_000_000_000, 470_000_000, flags));
            triggers_from_tlog_at(&tlog::record(1_700_000_000_000_000, &frame), 1_800_000_000_000_000).triggers
        };
        let tokens: Vec<(&str, Capture)> = [
            CameraFeedbackFlags::CAMERA_FEEDBACK_PHOTO,
            CameraFeedbackFlags::CAMERA_FEEDBACK_VIDEO,
            CameraFeedbackFlags::CAMERA_FEEDBACK_BADEXPOSURE,
            CameraFeedbackFlags::CAMERA_FEEDBACK_CLOSEDLOOP,
            CameraFeedbackFlags::CAMERA_FEEDBACK_OPENLOOP,
        ]
        .iter()
        .map(|flags| {
            let trigger = feedback(*flags).pop().expect("one camera feedback");
            (trigger.feedback.expect("a reported flag"), trigger.capture)
        })
        .collect();
        assert!(
            tokens.iter().all(|(_, capture)| *capture == Capture::Success),
            "a camera feedback message is ArduPilot reporting a capture, which DataFlashParser.cc marks Success unconditionally; reading the mode flag as a capture verdict made an open loop rig untaggable and threw away every badly exposed dusk frame: {tokens:?}"
        );
        assert_eq!(
            tokens.iter().map(|(token, _)| *token).collect::<Vec<_>>(),
            vec!["photo", "video", "badExposure", "closedLoop", "openLoop"],
            "the flag is still reported, as its own field a head can badge, so nothing the log said is lost"
        );
    }

    #[test]
    fn the_stage_spans_and_their_interpolation_keep_the_cplusplus_boundaries() {
        assert_eq!(
            (LOAD_IMAGES_END, PARSE_EXIF_END, PARSE_LOGS_END, CALIBRATE_END, TAG_IMAGES_END),
            (20.0, 40.0, 60.0, 80.0, 100.0)
        );
        assert_eq!(stage_progress(Stage::TaggingImages, 0, 0), Some(CALIBRATE_END), "no work to divide by reports the start of the stage, never a division by zero");
        assert_eq!(stage_progress(Stage::TaggingImages, 5, 10), Some(90.0));
        assert_eq!(stage_progress(Stage::TaggingImages, 10, 10), Some(TAG_IMAGES_END));
        assert_eq!(stage_progress(Stage::TaggingImages, 99, 10), Some(TAG_IMAGES_END), "a count past the total cannot push progress past the end of the run");
        assert_eq!(stage_progress(Stage::ParsingExif, 1, 2), Some(30.0));
        assert_eq!(stage_progress(Stage::LoadingImages, 1, 2), Some(10.0));
        assert_eq!(stage_progress(Stage::Idle, 1, 2), None, "an idle session has no progress to report, which is not the same as nought per cent");
    }

    #[test]
    fn the_session_reports_its_own_progress_through_every_stage_it_can_be_in() {
        let mut session = Session::new();
        assert_eq!(session.percent(), None, "a session that never ran has no percent, so a head can tell it apart from one sitting at nought");
        session.start(4).unwrap();
        assert_eq!(session.stage(), Stage::LoadingImages, "the head scanning a directory reports through the same interpolation the C++ gave that stage, rather than a bar that sits at nothing and jumps to twenty");
        session.record_loaded(2);
        assert_eq!(session.percent(), Some(10.0));
        session.finish_loading().unwrap();
        assert_eq!(session.percent(), Some(LOAD_IMAGES_END));
        session.record_exif(0, Some(50_000));
        assert_eq!(session.percent(), Some(25.0), "the session counts the images it has been handed, so two heads cannot invent two different denominators");
        [1usize, 2, 3].iter().for_each(|image| session.record_exif(*image, Some(50_000 + 10 * *image as i64)));
        session.finish_exif().unwrap();
        assert_eq!(session.percent(), Some(PARSE_EXIF_END));
        session.set_triggers(log((0..4).map(|i| shot(1000 + 10 * i, 47.0 + i as f64, 8.0)).collect())).unwrap();
        assert_eq!(session.percent(), Some(PARSE_LOGS_END));
        session.calibrate().unwrap();
        assert_eq!(session.percent(), Some(CALIBRATE_END));
        session.record_tag(0, Ok(()));
        session.record_tag(1, Ok(()));
        assert_eq!(session.percent(), Some(90.0), "half the matched images tagged is half of the tagging stage, which the session knows without being told");
        [2usize, 3].iter().for_each(|image| session.record_tag(*image, Ok(())));
        session.finish().unwrap();
        assert_eq!(session.percent(), Some(TAG_IMAGES_END));
    }

    #[test]
    fn a_session_walks_its_stages_and_reports_every_image_once() {
        let images = [Some(50_000), None, Some(50_020)];
        let mut session = Session::new();
        assert_eq!(session.stage(), Stage::Idle);
        assert_eq!(session.start(0), Err(Refusal::NoImages));
        assert_eq!(session.start(3), Ok(Stage::LoadingImages));
        assert_eq!(session.finish_loading(), Ok(Stage::ParsingExif));
        images.iter().enumerate().for_each(|(image, timestamp)| session.record_exif(image, *timestamp));
        assert_eq!(session.finish_exif(), Ok(Stage::ParsingLogs));
        assert_eq!(session.set_triggers(log(vec![shot(1000, 47.0, 8.0), shot(1020, 47.2, 8.2)])), Ok(Stage::Calibrating));
        let calibration = session.calibrate().expect("two of the three images have a trigger");
        assert_eq!(calibration.pairs, vec![Pair { image: 0, trigger: 0, apart_s: 0 }, Pair { image: 2, trigger: 1, apart_s: 0 }]);
        assert_eq!(session.stage(), Stage::TaggingImages);
        session.record_tag(0, Ok(()));
        session.record_tag(2, Err(Reason::OutputWriteRefused));
        let report = session.finish().expect("one image tagged, so the run is not a total failure");
        assert_eq!((report.tagged, report.failed, report.skipped, report.pending), (1, 1, 1, 0));
        assert_eq!(report.images[1].reason, Some(Reason::ExifTimestampUnreadable));
        assert_eq!(report.images[2].reason, Some(Reason::OutputWriteRefused));
        assert_eq!(report.images[0].fix.map(|f| f.latitude), Some(47.0));
        assert_eq!(
            report.images.len(),
            3,
            "every image the head handed over comes back with exactly one outcome, so nothing the operator selected can vanish from the answer"
        );
        assert_eq!(report.balance, Balance::MissingTriggers(1));
        assert_eq!(report.extent.map(|e| (e.south_deg, e.north_deg)), Some((47.0, 47.0)), "only tagged images have a place, so only they set the extent");
    }

    #[test]
    fn a_finished_run_comes_to_rest_idle_so_a_second_run_can_start() {
        let mut session = walked(&[Some(50_000)], vec![shot(1000, 47.0, 8.0)]);
        session.calibrate().unwrap();
        session.record_tag(0, Ok(()));
        session.finish().unwrap();
        assert_eq!(
            (session.stage(), session.is_busy(), session.is_completed()),
            (Stage::Idle, false, true),
            "the C++ _finishSuccess drops the stage to Idle and inProgress is stage != Idle, so a resting stage of anything else leaves the spinner turning for ever and the startTagging guard refuses every later run"
        );
        assert_eq!(session.start(1), Ok(Stage::LoadingImages), "and a second run is allowed, which is the whole point of coming to rest");
        assert!(!session.is_completed(), "a fresh start is not a completed run");
    }

    #[test]
    fn a_refused_stage_also_comes_to_rest_idle_and_names_its_refusal() {
        let refusals: Vec<(Refusal, Stage, Option<&str>)> = vec![
            {
                let mut blind = Session::new();
                blind.start(2).unwrap();
                blind.finish_loading().unwrap();
                blind.record_exif(0, None);
                blind.record_exif(1, None);
                let refusal = blind.finish_exif().expect_err("no readable timestamp");
                (refusal, blind.stage(), blind.snapshot()["refusal"].as_str().map(|_| "named"))
            },
            {
                let mut empty = Session::new();
                empty.start(1).unwrap();
                empty.finish_loading().unwrap();
                empty.record_exif(0, Some(50_000));
                empty.finish_exif().unwrap();
                let refusal = empty.set_triggers(log(Vec::new())).expect_err("no triggers");
                (refusal, empty.stage(), empty.snapshot()["refusal"].as_str().map(|_| "named"))
            },
            {
                let mut apart = walked(&[Some(50_000)], vec![Trigger { fix: None, ..shot(1000, 47.0, 8.0) }]);
                let refusal = apart.calibrate().expect_err("no match");
                (refusal, apart.stage(), apart.snapshot()["refusal"].as_str().map(|_| "named"))
            },
            {
                let mut failed = walked(&[Some(50_000)], vec![shot(1000, 47.0, 8.0)]);
                failed.calibrate().unwrap();
                failed.record_tag(0, Err(Reason::ExifWriteRefused));
                let refusal = failed.finish().expect_err("nothing tagged");
                (refusal, failed.stage(), failed.snapshot()["refusal"].as_str().map(|_| "named"))
            },
        ];
        assert_eq!(
            refusals,
            vec![
                (Refusal::NoExifTimestamps, Stage::Idle, Some("named")),
                (Refusal::NoTriggers, Stage::Idle, Some("named")),
                (Refusal::NoMatches, Stage::Idle, Some("named")),
                (Refusal::AllTagsFailed, Stage::Idle, Some("named")),
            ],
            "the C++ _finishWithError drops the stage to Idle on every refusal, so a refused run that latched the stage mid-flight would leave the operator with no escape short of restarting the application"
        );
    }

    #[test]
    fn a_log_that_could_not_be_decoded_refuses_differently_from_a_log_with_no_camera_events() {
        let mut garbled = Session::new();
        garbled.start(1).unwrap();
        garbled.finish_loading().unwrap();
        garbled.record_exif(0, Some(50_000));
        garbled.finish_exif().unwrap();
        assert_eq!(
            garbled.set_triggers(Log { triggers: Vec::new(), undecodable_frames: 91 }),
            Err(Refusal::UnreadableLog),
            "a firmware or dialect mismatch drops the very messages this feature lives on, and telling that operator there were no triggers sends them hunting a camera wiring fault in a log full of camera events"
        );
        assert_eq!(garbled.snapshot()["undecodableFrames"], json!(91), "the count the parser already returned is carried through instead of thrown away, the way the tlog view already publishes it");
        assert_eq!(garbled.snapshot()["refusal"], json!("unreadableLog"));
    }

    #[test]
    fn a_session_with_an_unusable_trigger_still_reports_it_beside_the_match() {
        let mut apart = walked(&[Some(50_000)], vec![shot(1000, 47.0, 8.0), Trigger { capture: Capture::Failure, ..shot(1010, 47.1, 8.1) }]);
        let calibration = apart.calibrate().expect("the good trigger still matches the single image");
        assert_eq!(calibration.unusable_triggers, vec![(1, Reason::TriggerCaptureFailed)]);
        assert_eq!(apart.report().unusable_triggers, vec![(1, Reason::TriggerCaptureFailed)]);
    }

    #[test]
    fn cancelling_stops_every_later_stage_and_every_result_still_in_flight() {
        let mut session = walked(&[Some(50_000), Some(50_010)], vec![shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1)]);
        session.calibrate().unwrap();
        session.cancel();
        assert!(session.is_cancelled() && session.stage() == Stage::Idle);
        session.record_tag(0, Ok(()));
        session.record_exif(1, Some(50_099));
        assert_eq!(
            (session.report().tagged, session.snapshot()["tagged"].clone()),
            (0, json!(0)),
            "the C++ discards results that land after a cancel, and a worker pool keeps completing tasks after the operator stops the run, so a cancelled session that still counted them would report a tagged image for a run nobody finished"
        );
        assert_eq!(session.finish_exif(), Err(Refusal::Cancelled));
        assert_eq!(session.set_triggers(log(vec![shot(1000, 47.0, 8.0)])), Err(Refusal::Cancelled));
        assert_eq!(session.calibrate(), Err(Refusal::Cancelled));
        assert_eq!(session.finish(), Err(Refusal::Cancelled));
        assert_eq!(session.start(2), Ok(Stage::LoadingImages));
        assert!(
            !session.is_cancelled() && session.refusal().is_none(),
            "the cancel flag is cleared by the next start, or a single cancelled run latches the subsystem shut for the rest of the session"
        );
        assert_eq!(session.report().images.iter().filter(|o| o.status == Status::Skipped).count(), 0, "and a new start clears the outcomes the cancelled run left behind");
    }

    #[test]
    fn a_stage_reached_out_of_order_is_refused_rather_than_half_run() {
        let mut session = Session::new();
        assert_eq!(session.calibrate(), Err(Refusal::OutOfOrder));
        assert_eq!(session.finish(), Err(Refusal::OutOfOrder));
        session.start(1).unwrap();
        assert_eq!(session.calibrate(), Err(Refusal::OutOfOrder));
        session.start(1).unwrap();
        session.record_exif(9, Some(1));
        session.record_tag(9, Ok(()));
        assert_eq!(session.report().images.len(), 1, "an index past the end of the image list is ignored, not grown into the report");
    }

    fn camera_feedback(time_usec: u64, lat: i32, flags: CameraFeedbackFlags) -> MavMessage {
        MavMessage::CAMERA_FEEDBACK(mavlink::dialects::ardupilotmega::CAMERA_FEEDBACK_DATA {
            time_usec,
            lat,
            lng: 80_000_000,
            alt_msl: 500.0,
            alt_rel: 120.0,
            roll: 1.0,
            pitch: -30.0,
            yaw: 450.0,
            foc_len: 24.0,
            img_idx: 7,
            target_system: 1,
            cam_idx: 0,
            flags,
            completed_captures: 7,
        })
    }

    fn frame(message: MavMessage) -> Vec<u8> {
        let header = mavlink::MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut frame = Vec::new();
        mavlink::write_v2_msg(&mut frame, header, &message).expect("a v2 frame");
        frame
    }

    #[test]
    fn triggers_come_out_of_a_telemetry_log_through_the_parser_the_crate_already_has() {
        let trigger_only = MavMessage::CAMERA_TRIGGER(mavlink::dialects::ardupilotmega::CAMERA_TRIGGER_DATA { time_usec: 1_700_000_010_000_000, seq: 8 });
        let bytes: Vec<u8> = [
            (1_700_000_000_000_000u64, camera_feedback(1_700_000_000_000_000, 470_000_000, CameraFeedbackFlags::CAMERA_FEEDBACK_CLOSEDLOOP)),
            (1_700_000_010_000_000u64, trigger_only),
        ]
        .into_iter()
        .flat_map(|(at, message)| tlog::record(at, &frame(message)))
        .collect();
        let parsed = triggers_from_tlog_at(&bytes, 1_800_000_000_000_000);
        assert_eq!(parsed.triggers.len(), 2, "both a camera feedback and a bare camera trigger are capture points");
        assert_eq!(parsed.undecodable_frames, 0);
        assert_eq!(parsed.triggers[0].timestamp_s, Some(1_700_000_000));
        assert_eq!(parsed.triggers[0].fix.map(|f| (f.latitude, f.longitude)), Some((47.0, 8.0)));
        assert_eq!(parsed.triggers[0].attitude.yaw_deg, Some(90.0));
        assert_eq!(parsed.triggers[0].ground_distance_m, Some(120.0));
        assert_eq!(parsed.triggers[0].sequence, 7);
        assert_eq!(parsed.triggers[0].unusable_reason(), None);
        assert_eq!(
            parsed.triggers[1].unusable_reason(),
            Some(Reason::TriggerCoordinateMissing),
            "a bare camera trigger carries a time and a sequence but no place, so it can be reported and cannot be used to tag"
        );
        assert_eq!(parsed.triggers[1].timestamp_s, Some(1_700_000_010));
        assert!(triggers_from_tlog(&[]).triggers.is_empty());
    }

    #[test]
    fn a_byte_swapped_record_timestamp_is_repaired_before_it_becomes_a_correlation_base() {
        let epoch_us = 1_700_000_000_000_000u64;
        let now_us = 1_800_000_000_000_000u64;
        let feedback = camera_feedback(0, 470_000_000, CameraFeedbackFlags::CAMERA_FEEDBACK_CLOSEDLOOP);
        let swapped = tlog::record(epoch_us.swap_bytes(), &frame(feedback));
        let parsed = triggers_from_tlog_at(&swapped, now_us);
        assert_eq!(
            (parsed.triggers[0].timestamp_s, parsed.triggers[0].timestamp_utc_s),
            (Some(1_700_000_000), Some(1_700_000_000)),
            "a camera with no time of its own falls back to the record timestamp, so the endianness repair the crate already has must run before that number becomes the base every offset is measured from"
        );
    }

    #[test]
    fn the_view_refuses_without_a_log_and_answers_with_tokens_and_numbers() {
        let refused = geotag_view(&Nothing, &[]);
        assert_eq!(refused["kind"], "null");
        assert!(refused["reason"].as_str().is_some_and(|r| r.contains("telemetry log")), "a view that cannot use its arguments says what it wanted");
        let missing = geotag_view(&Nothing, &["/no/such/log.tlog".to_string()]);
        assert_eq!((&missing["readable"], missing["class"].as_str()), (&json!(false), Some("GeoTag")));
        let sample = geotag_view(&Nothing, &[concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog").to_string(), "3".to_string(), "-".to_string()]);
        assert_eq!(sample["readable"], json!(true));
        assert_eq!(sample["toleranceSeconds"], json!(3.0));
        assert_eq!(sample["imageCount"], json!(1));
        assert_eq!(sample["images"][0]["reason"], json!("exifTimestampUnreadable"), "a dash for an unreadable EXIF time comes back as its own token, not as a match against the epoch");
        assert_eq!(sample["images"][0]["status"], json!("skipped"));
        assert!(sample["balance"].as_str().is_some());
        let body = sample.to_string();
        assert!(!body.contains(" m\"") && !body.contains(" degrees\"") && !body.contains(" seconds\""), "the view emits values and unit names as separate keys, never a formatted string an operator cannot read in their own language");
    }

    #[test]
    fn the_view_is_the_sessions_own_state_so_a_head_can_render_a_running_batch() {
        let sample = geotag_view(&Nothing, &[concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog").to_string(), "3".to_string(), "-".to_string()]);
        ["stage", "percent", "busy", "cancelled", "completed", "refusal", "tagged", "failed", "skipped", "pending", "clockSkewSeconds", "undecodableFrames"]
            .iter()
            .for_each(|field| {
                assert!(
                    sample.get(field).is_some(),
                    "a head with no stage token, no percent, no busy flag and no counts cannot tell a long run apart from a hung one, and {field} is missing"
                )
            });
        let mut session = walked(&[Some(50_000), Some(50_010)], vec![shot(1000, 47.0, 8.0), shot(1010, 47.1, 8.1)]);
        session.calibrate().unwrap();
        session.record_tag(0, Ok(()));
        let mid_run = session.snapshot();
        assert_eq!((&mid_run["stage"], &mid_run["busy"], &mid_run["percent"]), (&json!("taggingImages"), &json!(true), &json!(90.0)));
        assert_eq!((&mid_run["tagged"], &mid_run["pending"]), (&json!(1), &json!(1)), "the operator watching a long run sees tagged images land as they land");
        assert_eq!(mid_run["images"][0]["status"], json!("tagged"));
        assert_eq!(mid_run["clockSkewSeconds"], json!(49_000));
        assert_eq!(mid_run["triggers"][0]["feedback"], json!("closedLoop"));
        assert_eq!(mid_run["triggers"][0]["timeBase"], json!("epoch"));
        session.record_tag(1, Ok(()));
        session.finish().unwrap();
        let done = session.snapshot();
        assert_eq!((&done["stage"], &done["busy"], &done["completed"], &done["percent"]), (&json!("idle"), &json!(false), &json!(true), &json!(100.0)));
    }

    #[test]
    fn a_clamped_tolerance_reaches_the_matching_through_the_session() {
        let mut clamping = Session::new();
        clamping.set_tolerance_seconds(1000.0);
        assert_eq!(clamping.tolerance_seconds(), MAX_TOLERANCE_S);
        clamping.set_tolerance_seconds(f64::NAN);
        assert_eq!(clamping.tolerance_seconds(), DEFAULT_TOLERANCE_S);
        let mut session = walked(&[Some(50_000)], vec![shot(1000, 47.0, 8.0), shot(1004, 47.1, 8.1)]);
        session.set_tolerance_seconds(5.0);
        let calibration = session.calibrate().unwrap();
        assert_eq!(
            calibration.pairs,
            vec![Pair { image: 0, trigger: 0, apart_s: 4 }],
            "the session hands its clamped tolerance to the matching, and at five seconds the earlier trigger reaches the image a two second tolerance would have refused it"
        );
        assert_eq!(
            calibrate(&[Some(50_000)], &[shot(1000, 47.0, 8.0), shot(1004, 47.1, 8.1)], 2).pairs,
            vec![Pair { image: 0, trigger: 1, apart_s: 0 }],
            "at two seconds the earlier trigger cannot reach the image and the exact one takes it, so widening the tolerance changed which trigger the image was credited to, not merely how many matched"
        );
        assert_eq!(session.percent(), Some(CALIBRATE_END));
    }
}
