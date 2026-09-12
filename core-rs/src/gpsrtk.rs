use mavlink::dialects::ardupilotmega::GPS_RTCM_DATA_DATA;
use serde_json::{Value, json};

use crate::geo::wrap_longitude;
use crate::read::{flag, object, value_number};
use crate::router::Backend;
use crate::rtcm::Fragmenter;

pub const RECEIVE_TIMEOUT_MS: u32 = 1200;
pub const RECEIVE_RETRY_BUDGET: u8 = 3;
pub const SERIAL_OPEN_RETRIES: u32 = 60;
pub const SERIAL_OPEN_RETRY_DELAY_MS: u64 = 500;
pub const THREAD_DISCONNECT_TIMEOUT_MS: u32 = 2000;
pub const INITIAL_BAUD: u32 = 9600;
pub const ASHTECH_BAUD: u32 = 115200;
pub const AUTO_BAUD: u32 = 0;
pub const SBF_HEADING_OFFSET_RAD: f32 = 5.0;
pub const SURVEY_IN_ACCURACY_DRIVER_SCALE: f64 = 10000.0;
pub const MM_PER_M: f64 = 1000.0;
pub const MAX_SATELLITES: usize = 20;
pub const AZIMUTH_FULL_TURN: f64 = 255.0;
pub const STALE_AFTER_MS: u64 = RECEIVE_TIMEOUT_MS as u64 * RECEIVE_RETRY_BUDGET as u64;

pub const SURVEY_IN_VALID_BIT: u8 = 1;
pub const SURVEY_IN_ACTIVE_BIT: u8 = 1 << 1;

pub const FIX_TYPE_NONE: u8 = 1;
pub const FIX_TYPE_2D: u8 = 2;
pub const FIX_TYPE_3D: u8 = 3;
pub const FIX_TYPE_RTCM_CODE_DIFFERENTIAL: u8 = 4;
pub const FIX_TYPE_RTK_FLOAT: u8 = 5;
pub const FIX_TYPE_RTK_FIXED: u8 = 6;
pub const FIX_TYPE_EXTRAPOLATED: u8 = 8;

pub const SETTINGS_GROUP: &str = "settings.rtkSettings";
const SETTINGS_FACTS: [&str; 7] = [
    "surveyInAccuracyLimit",
    "surveyInMinObservationDuration",
    "useFixedBasePosition",
    "fixedBasePositionLatitude",
    "fixedBasePositionLongitude",
    "fixedBasePositionAltitude",
    "fixedBasePositionAccuracy",
];

pub fn deps() -> Vec<String> {
    SETTINGS_FACTS.iter().map(|name| fact_path(name)).collect()
}

fn fact_path(name: &str) -> String {
    format!("{SETTINGS_GROUP}.{name}.rawValue")
}

pub fn wrap_degrees(degrees: f64) -> f64 {
    degrees.rem_euclid(360.0)
}

pub fn radians_to_wrapped_degrees(radians: f32) -> f64 {
    wrap_degrees(radians.to_degrees() as f64)
}

pub fn fix_type_token(fix_type: u8) -> &'static str {
    match fix_type {
        FIX_TYPE_NONE => "none",
        FIX_TYPE_2D => "fix2D",
        FIX_TYPE_3D => "fix3D",
        FIX_TYPE_RTCM_CODE_DIFFERENTIAL => "rtcmCodeDifferential",
        FIX_TYPE_RTK_FLOAT => "rtkFloat",
        FIX_TYPE_RTK_FIXED => "rtkFixed",
        FIX_TYPE_EXTRAPOLATED => "extrapolated",
        _ => "unknown",
    }
}

pub fn jamming_token(state: u8) -> &'static str {
    match state {
        1 => "ok",
        2 => "warning",
        3 => "critical",
        _ => "unknown",
    }
}

pub fn spoofing_token(state: u8) -> &'static str {
    match state {
        1 => "none",
        2 => "indicated",
        3 => "multiple",
        _ => "unknown",
    }
}

pub fn rtcm_used_token(used: u8) -> &'static str {
    match used {
        1 => "notUsed",
        2 => "used",
        _ => "unknown",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Driver {
    UBlox,
    Trimble,
    Septentrio,
    Femtomes,
}

impl Driver {
    pub fn parse(gps_type: &str) -> Driver {
        let lowered = gps_type.to_lowercase();
        [("trimble", Driver::Trimble), ("septentrio", Driver::Septentrio), ("femtomes", Driver::Femtomes)]
            .into_iter()
            .find(|(needle, _)| lowered.contains(needle))
            .map(|(_, driver)| driver)
            .unwrap_or(Driver::UBlox)
    }

    pub fn id(self) -> &'static str {
        match self {
            Driver::UBlox => "ublox",
            Driver::Trimble => "trimble",
            Driver::Septentrio => "septentrio",
            Driver::Femtomes => "femtomes",
        }
    }

    pub fn baud(self) -> u32 {
        match self {
            Driver::Trimble => ASHTECH_BAUD,
            _ => AUTO_BAUD,
        }
    }

    pub fn heading_offset_rad(self) -> Option<f32> {
        matches!(self, Driver::Septentrio).then_some(SBF_HEADING_OFFSET_RAD)
    }

    pub fn azimuth_degrees(self, raw: u8) -> f64 {
        match self {
            Driver::UBlox => wrap_degrees(raw as f64 * 360.0 / AZIMUTH_FULL_TURN),
            _ => wrap_degrees(raw as f64),
        }
    }

    pub fn fills_satellite_table(self) -> bool {
        !matches!(self, Driver::Septentrio)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    PermissionDenied,
    DeviceMissing,
    SerialError,
    ConfigureFailed,
}

impl Fault {
    pub fn token(self) -> &'static str {
        match self {
            Fault::PermissionDenied => "permissionDenied",
            Fault::DeviceMissing => "deviceMissing",
            Fault::SerialError => "serialError",
            Fault::ConfigureFailed => "configureFailed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Settings {
    pub survey_in_accuracy_m: f64,
    pub survey_in_duration_s: u32,
    pub use_fixed_base: bool,
    pub fixed_latitude: f64,
    pub fixed_longitude: f64,
    pub fixed_altitude_m: f32,
    pub fixed_accuracy_m: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BasePosition {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f32,
    pub accuracy_mm: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurveySpec {
    pub accuracy_m: f64,
    pub accuracy_driver_units: f64,
    pub duration_s: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BasePlan {
    SurveyIn(SurveySpec),
    Fixed(BasePosition),
}

impl BasePlan {
    pub fn token(self) -> &'static str {
        match self {
            BasePlan::SurveyIn(_) => "surveyIn",
            BasePlan::Fixed(_) => "fixedBase",
        }
    }

    pub fn fixed(self) -> Option<BasePosition> {
        match self {
            BasePlan::Fixed(position) => Some(position),
            BasePlan::SurveyIn(_) => None,
        }
    }

    pub fn survey(self) -> Option<SurveySpec> {
        match self {
            BasePlan::SurveyIn(spec) => Some(spec),
            BasePlan::Fixed(_) => None,
        }
    }
}

pub fn survey_spec(settings: &Settings) -> SurveySpec {
    SurveySpec {
        accuracy_m: settings.survey_in_accuracy_m,
        accuracy_driver_units: settings.survey_in_accuracy_m * SURVEY_IN_ACCURACY_DRIVER_SCALE,
        duration_s: settings.survey_in_duration_s,
    }
}

pub fn base_position(settings: &Settings) -> Option<BasePosition> {
    let latitude = settings.fixed_latitude;
    let inside = settings.use_fixed_base && latitude.is_finite() && settings.fixed_longitude.is_finite() && (-90.0..=90.0).contains(&latitude);
    inside.then(|| BasePosition {
        latitude,
        longitude: wrap_longitude(settings.fixed_longitude),
        altitude_m: settings.fixed_altitude_m,
        accuracy_mm: (settings.fixed_accuracy_m as f64 * MM_PER_M) as f32,
    })
}

pub fn base_plan(settings: &Settings) -> BasePlan {
    base_position(settings).map(BasePlan::Fixed).unwrap_or(BasePlan::SurveyIn(survey_spec(settings)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Link {
    #[default]
    Disconnected,
    Opening {
        retries_left: u32,
    },
    Configuring,
    Streaming,
    Failed,
}

impl Link {
    pub fn token(self) -> &'static str {
        match self {
            Link::Disconnected => "disconnected",
            Link::Opening { .. } => "opening",
            Link::Configuring => "configuring",
            Link::Streaming => "streaming",
            Link::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurveyIn {
    pub duration_s: f32,
    pub accuracy_m: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f32,
    pub valid: bool,
    pub active: bool,
    pub at_ms: u64,
}

impl SurveyIn {
    pub fn position_known(&self) -> bool {
        self.latitude.is_finite() && self.longitude.is_finite()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Satellite {
    pub svid: u8,
    pub used: bool,
    pub elevation_raw: u8,
    pub azimuth_raw: u8,
    pub snr_db: u8,
    pub prn: u8,
}

impl Satellite {
    pub fn elevation_deg(&self) -> i8 {
        self.elevation_raw as i8
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Satellites {
    pub count: u8,
    pub list: Vec<Satellite>,
    pub at_ms: u64,
}

impl Satellites {
    pub fn used_count(&self) -> usize {
        self.list.iter().filter(|satellite| satellite.used).count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Relative {
    pub reference_station_id: u16,
    pub position_ned_m: [f32; 3],
    pub position_accuracy_m: [f32; 3],
    pub heading_rad: f32,
    pub heading_accuracy_rad: f32,
    pub position_length_m: f32,
    pub accuracy_length_m: f32,
    pub fix_ok: bool,
    pub differential: bool,
    pub position_valid: bool,
    pub carrier_floating: bool,
    pub carrier_fixed: bool,
    pub moving_base: bool,
    pub reference_position_miss: bool,
    pub reference_observations_miss: bool,
    pub heading_valid: bool,
    pub position_normalized: bool,
    pub at_ms: u64,
}

impl Relative {
    pub fn carrier_token(&self) -> &'static str {
        match (self.carrier_fixed, self.carrier_floating) {
            (true, _) => "fixed",
            (false, true) => "floating",
            (false, false) => "none",
        }
    }

    pub fn heading_deg(&self) -> Option<f64> {
        self.heading_valid.then(|| radians_to_wrapped_degrees(self.heading_rad))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fix {
    pub fix_type: u8,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_msl_m: f64,
    pub altitude_ellipsoid_m: f64,
    pub eph_m: f32,
    pub epv_m: f32,
    pub hdop: f32,
    pub vdop: f32,
    pub satellites_used: u8,
    pub heading_rad: f32,
    pub heading_offset_rad: f32,
    pub heading_accuracy_rad: f32,
    pub jamming_state: u8,
    pub spoofing_state: u8,
    pub rtcm_injection_rate_hz: f32,
    pub rtcm_msg_used: u8,
    pub rtcm_crc_failed: bool,
    pub at_ms: u64,
}

impl Fix {
    pub fn heading_deg(&self) -> Option<f64> {
        self.heading_rad.is_finite().then(|| radians_to_wrapped_degrees(self.heading_rad))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    NoData,
    Stale,
    Fresh,
}

impl Freshness {
    pub fn token(self) -> &'static str {
        match self {
            Freshness::NoData => "noData",
            Freshness::Stale => "stale",
            Freshness::Fresh => "fresh",
        }
    }
}

fn freshness(at_ms: Option<u64>, now_ms: u64) -> Freshness {
    match at_ms {
        None => Freshness::NoData,
        Some(at) if now_ms.saturating_sub(at) > STALE_AFTER_MS => Freshness::Stale,
        Some(_) => Freshness::Fresh,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    OpenSerial { device: String, baud: u32 },
    RetryOpenSerial { retries_left: u32, delay_ms: u64 },
    OpenFailed { reason: Fault },
    ConfigureDriver { driver: Driver, baud: u32, heading_offset_rad: Option<f32>, plan: BasePlan },
    RestartDriver,
    Connected(bool),
    WaitForThreadExit { timeout_ms: u32 },
    Rtcm(Vec<GPS_RTCM_DATA_DATA>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Corrections {
    pub bytes: u64,
    pub at_ms: u64,
}

#[derive(Debug, Default)]
pub struct Session {
    pub link: Link,
    pub fault: Option<Fault>,
    pub driver: Option<Driver>,
    pub device: String,
    pub settings: Settings,
    pub survey: Option<SurveyIn>,
    pub satellites: Option<Satellites>,
    pub relative: Option<Relative>,
    pub fix: Option<Fix>,
    pub corrections: Option<Corrections>,
    pub failed_receives: u8,
    pub failed_configures: u8,
    pub fragmenter: Fragmenter,
}

impl Session {
    pub fn live(&self) -> bool {
        matches!(self.link, Link::Configuring | Link::Streaming)
    }

    pub fn connect(&mut self, device: &str, gps_type: &str, settings: Settings) -> Vec<Out> {
        let stopping = self.disconnect();
        self.driver = Some(Driver::parse(gps_type));
        self.device = device.to_string();
        self.settings = settings;
        self.link = Link::Opening { retries_left: SERIAL_OPEN_RETRIES };
        stopping.into_iter().chain([Out::OpenSerial { device: device.to_string(), baud: INITIAL_BAUD }]).collect()
    }

    pub fn disconnect(&mut self) -> Vec<Out> {
        let was_live = self.link != Link::Disconnected;
        self.link = Link::Disconnected;
        self.fault = None;
        self.driver = None;
        self.device = String::new();
        self.survey = None;
        self.satellites = None;
        self.relative = None;
        self.fix = None;
        self.corrections = None;
        self.failed_receives = 0;
        self.failed_configures = 0;
        was_live.then(|| vec![Out::WaitForThreadExit { timeout_ms: THREAD_DISCONNECT_TIMEOUT_MS }, Out::Connected(false)]).unwrap_or_default()
    }

    pub fn serial_permission_denied(&mut self) -> Vec<Out> {
        match self.link {
            Link::Opening { retries_left } if retries_left > 0 => {
                self.link = Link::Opening { retries_left: retries_left - 1 };
                vec![Out::RetryOpenSerial { retries_left: retries_left - 1, delay_ms: SERIAL_OPEN_RETRY_DELAY_MS }]
            }
            Link::Opening { .. } => {
                self.link = Link::Failed;
                self.fault = Some(Fault::PermissionDenied);
                vec![Out::OpenFailed { reason: Fault::PermissionDenied }]
            }
            _ => Vec::new(),
        }
    }

    pub fn serial_failed(&mut self, reason: Fault) -> Vec<Out> {
        let live = !matches!(self.link, Link::Disconnected | Link::Failed);
        live.then(|| {
            self.link = Link::Failed;
            self.fault = Some(reason);
            vec![Out::OpenFailed { reason }, Out::Connected(false)]
        })
        .unwrap_or_default()
    }

    pub fn serial_timeout(&mut self) -> Vec<Out> {
        self.received(0)
    }

    pub fn serial_opened(&mut self) -> Vec<Out> {
        let Some(driver) = self.driver.filter(|_| matches!(self.link, Link::Opening { .. })) else { return Vec::new() };
        self.link = Link::Configuring;
        vec![Out::ConfigureDriver {
            driver,
            baud: driver.baud(),
            heading_offset_rad: driver.heading_offset_rad(),
            plan: base_plan(&self.settings),
        }]
    }

    pub fn driver_configured(&mut self) -> Vec<Out> {
        if self.link != Link::Configuring {
            return Vec::new();
        }
        self.link = Link::Streaming;
        self.failed_receives = 0;
        self.failed_configures = 0;
        self.fault = None;
        self.fix = None;
        vec![Out::Connected(true)]
    }

    pub fn driver_configure_failed(&mut self) -> Vec<Out> {
        match self.link {
            Link::Disconnected | Link::Failed => Vec::new(),
            _ => {
                self.link = Link::Configuring;
                self.failed_configures = self.failed_configures.saturating_add(1);
                self.fault = Some(Fault::ConfigureFailed);
                vec![Out::RestartDriver]
            }
        }
    }

    pub fn received(&mut self, mask: i32) -> Vec<Out> {
        if self.link != Link::Streaming {
            return Vec::new();
        }
        if mask > 0 {
            self.failed_receives = 0;
            return Vec::new();
        }
        self.failed_receives += 1;
        if self.failed_receives < RECEIVE_RETRY_BUDGET {
            return Vec::new();
        }
        self.failed_receives = 0;
        self.link = Link::Configuring;
        vec![Out::RestartDriver]
    }

    pub fn survey_in_status(&mut self, duration_s: f32, mean_accuracy_mm: f32, latitude: f64, longitude: f64, altitude_m: f32, flags: u8, now_ms: u64) {
        if !self.live() {
            return;
        }
        self.survey = Some(SurveyIn {
            duration_s,
            accuracy_m: mean_accuracy_mm as f64 / MM_PER_M,
            latitude,
            longitude: wrap_longitude(longitude),
            altitude_m,
            valid: flags & SURVEY_IN_VALID_BIT != 0,
            active: flags & SURVEY_IN_ACTIVE_BIT != 0,
            at_ms: now_ms,
        });
    }

    pub fn satellite_info(&mut self, count: u8, satellites: &[Satellite], now_ms: u64) {
        if !self.live() {
            return;
        }
        self.satellites = Some(Satellites {
            count,
            list: satellites.iter().take(MAX_SATELLITES.min(count as usize)).copied().collect(),
            at_ms: now_ms,
        });
    }

    pub fn gnss_relative(&mut self, relative: Relative) {
        if self.live() {
            self.relative = Some(relative);
        }
    }

    pub fn sensor_gps(&mut self, fix: Fix) {
        if self.live() {
            self.fix = Some(fix);
        }
    }

    pub fn rtcm(&mut self, data: &[u8], now_ms: u64) -> Vec<Out> {
        if !self.live() {
            return Vec::new();
        }
        self.corrections = Some(Corrections { bytes: self.corrections.map(|flow| flow.bytes).unwrap_or(0) + data.len() as u64, at_ms: now_ms });
        vec![Out::Rtcm(self.fragmenter.fragments(data))]
    }

    pub fn streaming_corrections(&self, now_ms: u64) -> bool {
        freshness(self.corrections.filter(|_| self.live()).map(|flow| flow.at_ms), now_ms) == Freshness::Fresh
    }

    pub fn survey_state_token(&self, now_ms: u64) -> &'static str {
        if !self.live() {
            return "idle";
        }
        let streaming = self.streaming_corrections(now_ms);
        match base_plan(&self.settings) {
            BasePlan::Fixed(_) if streaming => "rtkStreaming",
            BasePlan::Fixed(_) => "waitingForCorrections",
            BasePlan::SurveyIn(_) => match self.survey.filter(|survey| freshness(Some(survey.at_ms), now_ms) == Freshness::Fresh) {
                Some(survey) if survey.active => "surveyInActive",
                Some(survey) if survey.valid && streaming => "rtkStreaming",
                Some(survey) if survey.valid => "waitingForCorrections",
                _ => "surveyInWaiting",
            },
        }
    }

    pub fn facts(&self, now_ms: u64) -> Value {
        let live = self.live();
        let survey = self.survey.filter(|_| live);
        let satellites = self.satellites.as_ref().filter(|_| live);
        let relative = self.relative.filter(|_| live);
        let fix = self.fix.filter(|_| live);
        let corrections = self.corrections.filter(|_| live);
        let driver = self.driver.unwrap_or(Driver::UBlox);
        let table = satellites.filter(|_| driver.fills_satellite_table());
        let spec = survey_spec(&self.settings);
        json!({
            "kind": "object",
            "class": "GpsRtkFacts",
            "connected": live,
            "link": self.link.token(),
            "device": (!self.device.is_empty()).then(|| self.device.clone()),
            "reason": self.fault.map(Fault::token),
            "attemptsRemaining": match self.link {
                Link::Opening { retries_left } => Some(retries_left),
                _ => None,
            },
            "configureFailures": self.failed_configures,
            "driver": self.driver.map(Driver::id),
            "mode": base_plan(&self.settings).token(),
            "state": self.survey_state_token(now_ms),
            "surveyFreshness": freshness(survey.map(|s| s.at_ms), now_ms).token(),
            "currentDuration": survey.map(|s| s.duration_s),
            "durationUnit": "s",
            "targetDuration": spec.duration_s,
            "progress": survey.filter(|_| spec.duration_s > 0).map(|s| (s.duration_s as f64 / spec.duration_s as f64).clamp(0.0, 1.0)),
            "currentAccuracy": survey.map(|s| s.accuracy_m),
            "accuracyUnit": "m",
            "targetAccuracy": spec.accuracy_m,
            "withinAccuracyLimit": survey.map(|s| s.accuracy_m > 0.0 && s.accuracy_m <= spec.accuracy_m),
            "currentLatitude": survey.map(|s| s.latitude),
            "currentLongitude": survey.map(|s| s.longitude),
            "currentAltitude": survey.map(|s| s.altitude_m),
            "altitudeUnit": "m",
            "positionKnown": survey.map(|s| s.position_known()),
            "valid": survey.map(|s| s.valid),
            "active": survey.map(|s| s.active),
            "rtcmFreshness": freshness(corrections.map(|flow| flow.at_ms), now_ms).token(),
            "rtcmBytes": corrections.map(|flow| flow.bytes),
            "numSatellites": satellites.map(|s| s.count),
            "satellitesUsed": table.map(Satellites::used_count),
            "satelliteFreshness": freshness(satellites.map(|s| s.at_ms), now_ms).token(),
            "satellites": table.map(|s| {
                s.list
                    .iter()
                    .map(|satellite| {
                        json!({
                            "svid": satellite.svid,
                            "prn": satellite.prn,
                            "used": satellite.used,
                            "elevationDeg": satellite.elevation_deg(),
                            "azimuthDeg": driver.azimuth_degrees(satellite.azimuth_raw),
                            "snrDb": satellite.snr_db,
                        })
                    })
                    .collect::<Vec<_>>()
            }),
            "relative": relative.map(|relative| {
                json!({
                    "referenceStationId": relative.reference_station_id,
                    "carrierSolution": relative.carrier_token(),
                    "headingDeg": relative.heading_deg(),
                    "headingAccuracyDeg": relative.heading_accuracy_rad.to_degrees(),
                    "positionNedMeters": relative.position_ned_m,
                    "positionAccuracyMeters": relative.position_accuracy_m,
                    "positionLengthMeters": relative.position_length_m,
                    "accuracyLengthMeters": relative.accuracy_length_m,
                    "fixOk": relative.fix_ok,
                    "differential": relative.differential,
                    "positionValid": relative.position_valid,
                    "movingBase": relative.moving_base,
                    "referencePositionMiss": relative.reference_position_miss,
                    "referenceObservationsMiss": relative.reference_observations_miss,
                    "positionNormalized": relative.position_normalized,
                    "freshness": freshness(Some(relative.at_ms), now_ms).token(),
                })
            }),
            "fix": fix.map(|fix| {
                json!({
                    "fixType": fix_type_token(fix.fix_type),
                    "latitude": fix.latitude,
                    "longitude": fix.longitude,
                    "altitudeMslMeters": fix.altitude_msl_m,
                    "altitudeEllipsoidMeters": fix.altitude_ellipsoid_m,
                    "ephMeters": fix.eph_m,
                    "epvMeters": fix.epv_m,
                    "hdop": fix.hdop,
                    "vdop": fix.vdop,
                    "satellitesUsed": fix.satellites_used,
                    "headingDeg": fix.heading_deg(),
                    "headingOffsetDeg": fix.heading_offset_rad.to_degrees(),
                    "headingAccuracyDeg": fix.heading_accuracy_rad.to_degrees(),
                    "jamming": jamming_token(fix.jamming_state),
                    "spoofing": spoofing_token(fix.spoofing_state),
                    "rtcmInjectionRateHz": fix.rtcm_injection_rate_hz,
                    "rtcmMsgUsed": rtcm_used_token(fix.rtcm_msg_used),
                    "rtcmCrcFailed": fix.rtcm_crc_failed,
                    "freshness": freshness(Some(fix.at_ms), now_ms).token(),
                })
            }),
        })
    }
}

pub fn settings_from(backend: &dyn Backend) -> Settings {
    let number = |name: &str| value_number(&backend.get(&fact_path(name))).unwrap_or(0.0);
    Settings {
        survey_in_accuracy_m: number("surveyInAccuracyLimit"),
        survey_in_duration_s: number("surveyInMinObservationDuration") as u32,
        use_fixed_base: flag(&object(&backend.get(&fact_path("useFixedBasePosition"))), "value"),
        fixed_latitude: number("fixedBasePositionLatitude"),
        fixed_longitude: number("fixedBasePositionLongitude"),
        fixed_altitude_m: number("fixedBasePositionAltitude") as f32,
        fixed_accuracy_m: number("fixedBasePositionAccuracy") as f32,
    }
}

pub fn base_view(backend: &dyn Backend, args: &[String]) -> Value {
    let settings = settings_from(backend);
    let driver = Driver::parse(args.first().map(String::as_str).unwrap_or(""));
    let plan = base_plan(&settings);
    let spec = survey_spec(&settings);
    json!({
        "kind": "object",
        "class": "GpsRtkBase",
        "driver": driver.id(),
        "baud": driver.baud(),
        "headingOffsetRad": driver.heading_offset_rad(),
        "mode": plan.token(),
        "surveyInAccuracy": spec.accuracy_m,
        "surveyInAccuracyUnit": "m",
        "surveyInMinDuration": spec.duration_s,
        "surveyInMinDurationUnit": "s",
        "fixedBaseRequested": settings.use_fixed_base,
        "fixedBase": plan.fixed().map(|position| {
            json!({
                "latitude": position.latitude,
                "longitude": position.longitude,
                "altitude": position.altitude_m,
                "altitudeUnit": "m",
                "accuracy": position.accuracy_mm,
                "accuracyUnit": "mm",
            })
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            survey_in_accuracy_m: 2.0,
            survey_in_duration_s: 180,
            use_fixed_base: false,
            fixed_latitude: 0.0,
            fixed_longitude: 0.0,
            fixed_altitude_m: 0.0,
            fixed_accuracy_m: 0.0,
        }
    }

    fn fixed_settings() -> Settings {
        Settings { use_fixed_base: true, fixed_latitude: 47.4, fixed_longitude: 8.5, fixed_altitude_m: 410.0, fixed_accuracy_m: 0.05, ..settings() }
    }

    fn satellite(svid: u8, used: bool, azimuth_raw: u8) -> Satellite {
        Satellite { svid, used, elevation_raw: 40, azimuth_raw, snr_db: 42, prn: svid }
    }

    fn relative() -> Relative {
        Relative {
            reference_station_id: 7,
            position_ned_m: [1.0, 2.0, 3.0],
            position_accuracy_m: [0.01, 0.02, 0.03],
            heading_rad: -std::f32::consts::FRAC_PI_2,
            heading_accuracy_rad: 0.0,
            position_length_m: 3.7,
            accuracy_length_m: 0.04,
            fix_ok: true,
            differential: true,
            position_valid: true,
            carrier_floating: true,
            carrier_fixed: false,
            moving_base: false,
            reference_position_miss: false,
            reference_observations_miss: false,
            heading_valid: true,
            position_normalized: false,
            at_ms: 1_000,
        }
    }

    fn fix() -> Fix {
        Fix {
            fix_type: FIX_TYPE_RTK_FIXED,
            latitude: 47.4,
            longitude: 8.5,
            altitude_msl_m: 400.0,
            altitude_ellipsoid_m: 447.0,
            eph_m: 0.02,
            epv_m: 0.03,
            hdop: 0.7,
            vdop: 0.9,
            satellites_used: 19,
            heading_rad: 7.0,
            heading_offset_rad: SBF_HEADING_OFFSET_RAD,
            heading_accuracy_rad: 0.01,
            jamming_state: 2,
            spoofing_state: 1,
            rtcm_injection_rate_hz: 1.5,
            rtcm_msg_used: 2,
            rtcm_crc_failed: false,
            at_ms: 1_000,
        }
    }

    fn streaming(gps_type: &str, settings: Settings) -> Session {
        let mut session = Session::default();
        session.connect("/dev/ttyACM0", gps_type, settings);
        session.serial_opened();
        session.driver_configured();
        session
    }

    #[test]
    fn the_gps_type_string_picks_the_driver_its_baud_and_its_heading_offset() {
        assert_eq!(Driver::parse("Trimble BX992"), Driver::Trimble, "the C++ matches the vendor name case-insensitively anywhere in the string");
        assert_eq!(Driver::parse("SEPTENTRIO mosaic"), Driver::Septentrio);
        assert_eq!(Driver::parse("femtomes-x"), Driver::Femtomes);
        assert_eq!(Driver::parse("anything else"), Driver::UBlox, "an unrecognised type falls back to u-blox, never to nothing");
        assert_eq!(Driver::Trimble.baud(), 115200, "only the Ashtech driver is given a fixed baud; the rest autodetect from 0");
        assert_eq!((Driver::UBlox.baud(), Driver::Septentrio.baud(), Driver::Femtomes.baud()), (0, 0, 0));
        assert_eq!(Driver::Septentrio.heading_offset_rad(), Some(5.0), "the SBF driver is the only one QGC hands a heading offset, and the number is 5 exactly");
        assert_eq!(Driver::UBlox.heading_offset_rad(), None);
    }

    #[test]
    fn the_survey_spec_and_the_fixed_base_keep_the_cpp_unit_scales() {
        let spec = survey_spec(&Settings { survey_in_accuracy_m: 2.0, ..settings() });
        assert_eq!(
            (spec.accuracy_m, spec.accuracy_driver_units, spec.duration_s),
            (2.0, 20000.0, 180),
            "the driver wants 0.1mm, so metres scale by 10000 and never by 1000 - asserted as the literal the C++ multiplies by, not by calling survey_spec twice"
        );
        assert_eq!(STALE_AFTER_MS, 3600, "three receive timeouts of 1200ms is the staleness window, spelled out so the derivation cannot drift");
        assert_eq!(base_position(&settings()), None, "with the fixed base switched off there is no base position to send, not a zero one");
        let fixed = Settings { fixed_longitude: 200.0, ..fixed_settings() };
        let position = base_position(&fixed).expect("a switched-on fixed base yields a position");
        assert_eq!(position.longitude, -160.0, "longitude wraps into [-180,180] instead of being sent past the antimeridian");
        assert_eq!(position.accuracy_mm, 50.0, "the fixed base accuracy goes to the driver in millimetres, and 0.05m is 50mm exactly");
        assert_eq!(base_position(&Settings { fixed_latitude: 91.0, ..fixed }), None, "a latitude off the globe is refused, since it cannot wrap into one");
        assert_eq!(base_position(&Settings { fixed_latitude: f64::NAN, ..fixed }), None);
    }

    #[test]
    fn the_base_plan_is_one_value_so_a_fixed_base_cannot_be_overwritten_by_a_survey_spec() {
        assert_eq!(base_plan(&settings()), BasePlan::SurveyIn(survey_spec(&settings())), "with no fixed base the only plan is to survey in");
        let plan = base_plan(&fixed_settings());
        assert_eq!(plan, BasePlan::Fixed(base_position(&fixed_settings()).unwrap()));
        assert_eq!(plan.survey(), None, "the C++ union is last-call-wins, so a plan that is fixed carries no survey spec a host could apply after it");
        assert_eq!((plan.token(), base_plan(&settings()).token()), ("fixedBase", "surveyIn"));
        let mut session = Session::default();
        session.connect("/dev/ttyACM0", "u-blox", fixed_settings());
        assert_eq!(
            session.serial_opened(),
            vec![Out::ConfigureDriver { driver: Driver::UBlox, baud: 0, heading_offset_rad: None, plan: BasePlan::Fixed(base_position(&fixed_settings()).unwrap()) }],
            "one field means the ordering of setSurveyInSpecs and setBasePosition is no longer the caller's to get wrong"
        );
    }

    #[test]
    fn connecting_opens_the_port_retries_permission_denials_and_then_names_the_cause() {
        let mut session = Session::default();
        assert_eq!(session.connect("/dev/ttyACM0", "u-blox", settings()), vec![Out::OpenSerial { device: "/dev/ttyACM0".into(), baud: 9600 }]);
        assert_eq!(session.link, Link::Opening { retries_left: 60 }, "the C++ retries a permission error sixty times, and the budget is visible");
        assert_eq!(session.facts(0)["attemptsRemaining"], 60, "the operator can tell patient retrying from hung, which one opening token cannot say");
        assert_eq!(session.facts(0)["device"], "/dev/ttyACM0", "and which port is being waited on, since that is what a udev rule names");
        assert_eq!(session.serial_permission_denied(), vec![Out::RetryOpenSerial { retries_left: 59, delay_ms: 500 }]);
        let exhausted = (0..59).fold(Vec::new(), |_, _| session.serial_permission_denied());
        assert_eq!(exhausted, vec![Out::RetryOpenSerial { retries_left: 0, delay_ms: 500 }]);
        assert_eq!(session.serial_permission_denied(), vec![Out::OpenFailed { reason: Fault::PermissionDenied }], "once the budget is spent the retry stops, and the failure says which of the three cures applies");
        assert_eq!(session.link, Link::Failed);
        assert_eq!(session.facts(0)["reason"], "permissionDenied", "a udev rule fixes this one; a missing device does not, so the two cannot share a token");
        assert!(session.serial_opened().is_empty(), "a port that opened after the session gave up does not resurrect it");
    }

    #[test]
    fn the_three_link_failures_are_three_tokens_and_a_configure_loop_counts_itself() {
        let mut missing = Session::default();
        missing.connect("/dev/ttyACM0", "u-blox", settings());
        missing.serial_opened();
        assert_eq!(missing.serial_failed(Fault::DeviceMissing), vec![Out::OpenFailed { reason: Fault::DeviceMissing }, Out::Connected(false)]);
        assert_eq!(missing.facts(0)["reason"], "deviceMissing", "replug the cable, do not go hunting for a permission");

        let mut stubborn = streaming("u-blox", settings());
        stubborn.driver_configure_failed();
        stubborn.driver_configure_failed();
        let facts = stubborn.facts(0);
        assert_eq!((&facts["link"], &facts["reason"]), (&json!("configuring"), &json!("configureFailed")), "a receiver that never configures is not silently configuring forever");
        assert_eq!(facts["configureFailures"], 2, "the attempts are counted, so a stuck rebuild is distinguishable from a first try");
        assert_eq!(stubborn.driver_configured()[0], Out::Connected(true));
        assert_eq!(stubborn.facts(0)["reason"], Value::Null, "and a configure that finally works clears the cause");
    }

    #[test]
    fn a_read_timeout_is_tolerated_where_a_serial_error_is_fatal() {
        let mut session = streaming("u-blox", settings());
        assert!(session.serial_timeout().is_empty());
        assert!(session.serial_timeout().is_empty());
        assert_eq!(session.link, Link::Streaming, "the C++ explicitly excludes TimeoutError from the errors that end the session, and 1200ms is the timeout this module itself sets");
        assert_eq!(session.serial_timeout(), vec![Out::RestartDriver], "three of them rebuild the driver, which is the same budget an empty receive spends");
        assert_eq!(session.link, Link::Configuring, "and the session lives, so the operator does not have to re-press connect mid-mission");
        session.driver_configured();
        assert_eq!(session.serial_failed(Fault::SerialError), vec![Out::OpenFailed { reason: Fault::SerialError }, Out::Connected(false)], "a real serial error still ends it");
        assert_eq!(session.link, Link::Failed);
    }

    #[test]
    fn configuring_the_driver_carries_the_plan_and_clears_the_previous_fix() {
        let mut session = Session::default();
        session.connect("/dev/ttyACM0", "septentrio", fixed_settings());
        assert_eq!(
            session.serial_opened(),
            vec![Out::ConfigureDriver {
                driver: Driver::Septentrio,
                baud: 0,
                heading_offset_rad: Some(5.0),
                plan: BasePlan::Fixed(BasePosition { latitude: 47.4, longitude: 8.5, altitude_m: 410.0, accuracy_mm: 50.0 }),
            }],
            "the plan is asserted against the numbers the driver is owed, not against the functions that produce them"
        );
        assert_eq!(session.link, Link::Configuring);
        session.sensor_gps(fix());
        assert_eq!(session.driver_configured(), vec![Out::Connected(true)]);
        assert_eq!(session.fix, None, "the C++ memsets the gps struct on every driver start, so a fix from the last driver is not reported as this one's");
        assert!(session.driver_configured().is_empty(), "connected is announced once per driver start, not on every call");
    }

    #[test]
    fn three_empty_receives_restart_the_driver_and_the_budget_resets() {
        let mut session = streaming("u-blox", settings());
        assert!(session.received(0).is_empty());
        assert!(session.received(-1).is_empty());
        assert_eq!(session.received(0), vec![Out::RestartDriver], "the C++ gives the driver three failed receives before rebuilding it");
        assert_eq!(session.failed_receives, 0, "and the counter clears, so the next restart needs three more failures");
        session.driver_configured();
        assert!(session.received(-1).is_empty());
        assert!(session.received(3).is_empty(), "the host passed the mask in, so echoing it back as a publish order tells it nothing it did not already know");
        assert_eq!(session.failed_receives, 0, "any successful receive clears the budget");
        assert_eq!(RECEIVE_TIMEOUT_MS, 1200);
    }

    #[test]
    fn a_driver_rebuild_ages_the_survey_instead_of_blanking_it() {
        let mut session = streaming("u-blox", settings());
        session.survey_in_status(42.0, 1800.0, 47.4, 8.5, 410.0, SURVEY_IN_ACTIVE_BIT, 1_000);
        session.satellite_info(11, &[satellite(1, true, 0)], 1_000);
        assert_eq!(session.received(0), vec![]);
        assert_eq!(session.received(0), vec![]);
        assert_eq!(session.received(0), vec![Out::RestartDriver]);
        let facts = session.facts(1_100);
        assert_eq!(facts["currentDuration"], 42.0, "the C++ resets no fact when GPSProvider::run rebuilds the driver, so a 3.6s USB hiccup does not erase the operator's survey progress");
        assert_eq!(facts["surveyFreshness"], "fresh", "the age is what the freshness token is for; blanking the reading bypasses it");
        assert_eq!(facts["connected"], true, "_onGPSConnect fires once per connectGPS, so a rebuild does not report the base station as disconnected");
        assert_eq!(facts["state"], "surveyInActive");
        assert_eq!(facts["numSatellites"], 11, "and the satellite count ages by the same rule as the survey, not a different one");
    }

    #[test]
    fn the_survey_in_flags_split_into_valid_and_active_and_the_accuracy_is_metres() {
        let mut session = streaming("u-blox", settings());
        session.survey_in_status(12.0, 2500.0, 47.4, 190.0, 410.0, 0b10, 5_000);
        let survey = session.survey.expect("a survey-in report is stored");
        assert_eq!((survey.valid, survey.active), (false, true), "bit 0 is valid and bit 1 is active, and they are independent");
        assert!((survey.accuracy_m - 2.5).abs() < 1e-9, "the driver reports millimetres and the core reports metres");
        assert_eq!(survey.longitude, -170.0, "a survey-in longitude past the antimeridian wraps rather than being reported out of range");
        assert_eq!(session.survey_state_token(5_000), "surveyInActive");
        assert_eq!(
            session.survey_state_token(5_000 + STALE_AFTER_MS + 1),
            "surveyInWaiting",
            "the largest token on the screen cannot say a survey is in progress while the freshness field beside it says the report is stale"
        );
        session.survey_in_status(180.0, 900.0, 47.4, 8.5, 410.0, 0b01, 6_000);
        assert_eq!(session.survey_state_token(6_000), "waitingForCorrections", "a validated survey is not streaming until bytes move; that is the failure that flies a drone on a non-RTK fix under an RTK label");
        session.rtcm(&[1u8; 200], 6_000);
        assert_eq!(session.survey_state_token(6_000), "rtkStreaming", "valid, no longer active, and corrections leaving is the streaming state the head draws");
        session.survey_in_status(1.0, 0.0, 47.4, 8.5, 410.0, 0, 6_000);
        assert_eq!(session.survey_state_token(6_000), "surveyInWaiting", "neither valid nor active is its own answer, not streaming");
    }

    #[test]
    fn a_fixed_base_reports_its_mode_and_streams_without_a_survey_report() {
        let mut session = streaming("u-blox", fixed_settings());
        let waiting = session.facts(1_000);
        assert_eq!(waiting["mode"], "fixedBase", "the driver activates RTCM at configure time in fixed mode and never sends NAV-SVIN, so the mode has to reach the head some other way");
        assert_eq!(waiting["state"], "waitingForCorrections", "and not surveyInWaiting, which would be a survey the receiver was never asked to run");
        session.rtcm(&[1u8; 300], 1_000);
        let facts = session.facts(1_000);
        assert_eq!(facts["state"], "rtkStreaming", "a fixed base with corrections flowing is streaming, with no survey report to wait for");
        assert_eq!(facts["surveyFreshness"], "noData", "the survey block stays honestly absent rather than being invented");
        assert_eq!(facts["rtcmBytes"], 300);
        assert_eq!(session.facts(1_000 + STALE_AFTER_MS + 1)["state"], "waitingForCorrections", "and a base whose RTCM stopped stops claiming to stream");
    }

    #[test]
    fn the_survey_targets_travel_with_the_readings_they_are_measured_against() {
        let mut session = streaming("u-blox", settings());
        session.survey_in_status(45.0, 0.0, f64::NAN, f64::NAN, f32::NAN, 0b10, 1_000);
        let early = session.facts(1_000);
        assert_eq!((&early["targetDuration"], &early["targetAccuracy"]), (&json!(180), &json!(2.0)), "42 seconds and 3.1 metres answer nothing without the limits the survey is racing");
        assert_eq!(early["progress"], 0.25);
        assert_eq!(early["withinAccuracyLimit"], false, "ublox reports meanAcc 0 before it has computed anything, and zero must not read as perfect convergence");
        assert_eq!(early["positionKnown"], false, "the driver documents lat/lon as NAN until known, which is a different answer from never reported");
        assert_eq!(early["currentLatitude"], Value::Null);
        session.survey_in_status(200.0, 1500.0, 47.4, 8.5, 410.0, 0b01, 2_000);
        let converged = session.facts(2_000);
        assert_eq!(converged["progress"], 1.0, "past the minimum duration the fraction clamps instead of reading over 100%");
        assert_eq!(converged["withinAccuracyLimit"], true);
        assert_eq!(converged["positionKnown"], true);
    }

    #[test]
    fn no_data_stale_data_and_zero_data_stay_three_answers() {
        let mut session = streaming("u-blox", settings());
        let empty = session.facts(0);
        assert_eq!(empty["currentDuration"], Value::Null, "before any report the duration is absent, not zero");
        assert_eq!(empty["numSatellites"], Value::Null);
        assert_eq!(empty["surveyFreshness"], "noData");
        assert_eq!(empty["rtcmFreshness"], "noData", "and a base that has sent no corrections says so, rather than looking like one whose stream died");
        session.survey_in_status(0.0, 0.0, 0.0, 0.0, 0.0, 0, 1_000);
        session.satellite_info(0, &[], 1_000);
        let zeroed = session.facts(1_000);
        assert_eq!(zeroed["currentDuration"], 0.0, "a report of zero seconds is a number, and distinguishable from absence");
        assert_eq!(zeroed["numSatellites"], 0);
        assert_eq!(zeroed["surveyFreshness"], "fresh");
        assert_eq!(session.facts(1_000 + STALE_AFTER_MS + 1)["surveyFreshness"], "stale", "a report older than three receive timeouts is stale, and still carries its last numbers");
        assert_eq!(session.facts(1_000 + STALE_AFTER_MS + 1)["currentDuration"], 0.0);
        assert_eq!(session.facts(1_000 + STALE_AFTER_MS + 1)["state"], "surveyInWaiting", "and the biggest token on the screen agrees with the freshness beside it instead of contradicting it");
    }

    #[test]
    fn disconnecting_clears_every_reading_and_the_connected_flag() {
        let mut session = streaming("u-blox", settings());
        session.survey_in_status(20.0, 1000.0, 47.4, 8.5, 410.0, 0b11, 1_000);
        session.satellite_info(2, &[satellite(1, true, 0), satellite(2, false, 128)], 1_000);
        session.gnss_relative(relative());
        session.sensor_gps(fix());
        session.rtcm(&[1u8; 100], 1_000);
        assert_eq!(session.facts(1_000)["connected"], true);
        assert_eq!(session.disconnect(), vec![Out::WaitForThreadExit { timeout_ms: 2000 }, Out::Connected(false)]);
        let after = session.facts(1_000);
        assert_eq!(after["connected"], false);
        assert_eq!(
            (&after["numSatellites"], &after["valid"], &after["relative"], &after["fix"]),
            (&Value::Null, &Value::Null, &Value::Null, &Value::Null),
            "nothing survives a disconnect as a latched reading"
        );
        assert_eq!(after["rtcmFreshness"], "noData", "including the correction stream, whose byte count would otherwise span the gap to the next connect");
        assert_eq!(after["state"], "idle");
        assert!(session.disconnect().is_empty(), "disconnecting twice announces the drop once");
    }

    #[test]
    fn a_dead_link_reports_no_reading_and_accepts_no_late_one() {
        let mut session = streaming("u-blox", settings());
        session.satellite_info(14, &[satellite(1, true, 0)], 1_000);
        session.sensor_gps(fix());
        session.gnss_relative(relative());
        session.rtcm(&[1u8; 100], 1_000);
        assert_eq!(session.serial_failed(Fault::SerialError), vec![Out::OpenFailed { reason: Fault::SerialError }, Out::Connected(false)]);
        let after = session.facts(1_000);
        assert_eq!(after["connected"], false);
        assert_eq!(
            (&after["numSatellites"], &after["fix"], &after["relative"]),
            (&Value::Null, &Value::Null, &Value::Null),
            "a dead receiver rendering RTK Fixed on 14 satellites is the reading an operator trusts the base on"
        );
        assert_eq!(after["rtcmFreshness"], "noData");
        session.satellite_info(9, &[satellite(1, true, 0)], 1_200);
        session.sensor_gps(fix());
        session.gnss_relative(relative());
        assert!(session.rtcm(&[1u8; 100], 1_200).is_empty(), "the C++ queues these across threads while disconnect waits, so in-flight callbacks land after the stop and must not resurrect the session");
        let late = session.facts(1_200);
        assert_eq!((&late["numSatellites"], &late["fix"], &late["rtcmFreshness"]), (&Value::Null, &Value::Null, &json!("noData")));
        assert!(session.received(1).is_empty(), "a dead link accepts no further receives");
        assert!(session.serial_failed(Fault::SerialError).is_empty(), "and the drop is announced once");
    }

    #[test]
    fn satellite_azimuth_is_decoded_per_driver_and_elevation_can_be_below_the_horizon() {
        assert_eq!(Driver::UBlox.azimuth_degrees(0), 0.0);
        assert!((Driver::UBlox.azimuth_degrees(128) - 180.7).abs() < 0.1, "the ublox driver stores azimuth as 0..255 over a full turn");
        assert_eq!(Driver::UBlox.azimuth_degrees(255), 0.0, "a full turn wraps back to zero instead of reading 360");
        assert_eq!(Driver::Trimble.azimuth_degrees(200), 200.0, "ashtech parses the NMEA GSV azimuth in degrees and femtomes documents azi in degrees, so scaling them points the sky plot 40% off");
        assert_eq!(Driver::Femtomes.azimuth_degrees(200), 200.0);
        assert_eq!(wrap_degrees(-90.0), 270.0);
        assert_eq!(wrap_degrees(-360.0), 0.0, "a reverse turn is zero, wrapped with the rem_euclid every other module in the crate uses");

        let mut trimble = streaming("trimble", settings());
        trimble.satellite_info(1, &[Satellite { elevation_raw: (-5i8) as u8, azimuth_raw: 200, ..satellite(9, true, 200) }], 0);
        let entry = &trimble.facts(0)["satellites"][0];
        assert_eq!(entry["azimuthDeg"], 200.0);
        assert_eq!(entry["elevationDeg"], -5, "ubx stores elev as int8 cast to uint8, so a satellite below the horizon read as 251 degrees up");

        let mut septentrio = streaming("septentrio", settings());
        septentrio.satellite_info(12, &[], 0);
        let facts = septentrio.facts(0);
        assert_eq!(facts["numSatellites"], 12, "the sbf driver fills only count, and that count is already satellites-used");
        assert_eq!(facts["satellites"], Value::Null, "so there is no per-satellite table to draw, rather than twenty zeroed rows");
        assert_eq!(facts["satellitesUsed"], Value::Null, "and used-of-total is not invented as 0 of 12 under a good RTK fix");
    }

    #[test]
    fn the_satellite_table_is_capped_at_the_slots_the_struct_has() {
        let mut session = streaming("u-blox", settings());
        let many: Vec<Satellite> = (0..30u8).map(|i| satellite(i, i % 2 == 0, i)).collect();
        session.satellite_info(30, &many, 0);
        let stored = session.satellites.as_ref().unwrap();
        assert_eq!(stored.count, 30, "the driver's own count is reported as given");
        assert_eq!(stored.list.len(), MAX_SATELLITES, "but the per-satellite table is the twenty slots the struct has");
        assert_eq!(stored.used_count(), 10);
    }

    #[test]
    fn the_relative_reading_reports_a_wrapped_bearing_and_a_carrier_token() {
        let floating = relative();
        assert_eq!(floating.carrier_token(), "floating");
        assert_eq!(Relative { carrier_fixed: true, ..floating }.carrier_token(), "fixed");
        assert_eq!(Relative { carrier_floating: false, ..floating }.carrier_token(), "none");
        assert!((floating.heading_deg().unwrap() - 270.0).abs() < 1e-4, "a negative heading in radians comes back as a bearing in 0..360");
        assert_eq!(Relative { heading_valid: false, ..floating }.heading_deg(), None, "an invalid heading is absent, not north");
    }

    #[test]
    fn the_fact_group_emits_tokens_and_numbers_a_head_can_format() {
        let mut session = streaming("trimble", settings());
        session.survey_in_status(180.0, 1500.0, 47.4, 8.5, 410.0, 0b01, 1_000);
        session.satellite_info(1, &[satellite(9, true, 64)], 1_000);
        session.gnss_relative(relative());
        session.sensor_gps(fix());
        session.rtcm(&[1u8; 120], 1_000);
        let facts = session.facts(1_000);
        assert_eq!(facts["driver"], "trimble");
        assert_eq!(
            (&facts["accuracyUnit"], &facts["durationUnit"]),
            (&json!("m"), &json!("s")),
            "the unit is named and the value is a number, so the head formats it in its own language"
        );
        assert_eq!(facts["currentAccuracy"], 1.5);
        assert_eq!(facts["state"], "rtkStreaming");
        assert_eq!(facts["fix"]["fixType"], "rtkFixed");
        assert_eq!(facts["fix"]["jamming"], "warning");
        assert_eq!(facts["fix"]["spoofing"], "none");
        assert_eq!(facts["fix"]["rtcmMsgUsed"], "used");
        assert_eq!(facts["relative"]["carrierSolution"], "floating");
        assert_eq!(facts["satellites"][0]["used"], true);
        assert_eq!(facts["satellites"][0]["azimuthDeg"], 64.0);
        assert_eq!(fix_type_token(0), "unknown", "a fix type the firmware never defined is unknown, not none");
        assert_eq!(fix_type_token(FIX_TYPE_EXTRAPOLATED), "extrapolated");
        let text = facts.to_string();
        assert!(!text.contains(" m\"") && !text.contains(" s\""), "no value is emitted with its unit glued on, since the operators read Ukrainian: {text}");
    }

    #[test]
    fn a_mounting_offset_keeps_its_sign_where_a_bearing_wraps() {
        let mut session = streaming("septentrio", settings());
        session.sensor_gps(Fix { heading_offset_rad: -0.1, heading_rad: -std::f32::consts::FRAC_PI_2, ..fix() });
        let block = &session.facts(1_000)["fix"];
        assert!(
            (block["headingOffsetDeg"].as_f64().unwrap() + 5.729).abs() < 1e-3,
            "sbf documents the offset as radians [-pi,pi] subtracted from the measurement, so wrapping -0.1 rad into 354.3 degrees reads as an almost-north offset"
        );
        assert!((block["headingDeg"].as_f64().unwrap() - 270.0).abs() < 1e-3, "the measurement itself is a bearing and still wraps");
    }

    #[test]
    fn rtcm_goes_out_through_the_shared_fragmenter_and_the_bytes_are_counted() {
        let mut session = streaming("u-blox", settings());
        let out = session.rtcm(&[1u8; 400], 0);
        let Out::Rtcm(fragments) = &out[0] else { panic!("rtcm is fragmented for the wire") };
        assert_eq!(
            fragments.iter().map(|f| f.len as usize).collect::<Vec<_>>(),
            vec![180, 180, 40],
            "the crate's own fragmenter does the cutting, not a copy of it"
        );
        assert_eq!(out.len(), 1, "and nothing else rides along; the kB/s figure the C++ computed lives inside QT_DEBUG behind a single qCDebug line");
        session.rtcm(&[1u8; 100], 1_001);
        assert_eq!(session.facts(1_001)["rtcmBytes"], 500);
        assert_eq!(session.facts(1_001)["rtcmFreshness"], "fresh");
        assert_eq!(session.facts(1_001 + STALE_AFTER_MS + 1)["rtcmFreshness"], "stale", "a stream that stopped is stale rather than silently unchanged, which is the whole point of watching it");
    }

    struct Fake;

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.rtkSettings.surveyInAccuracyLimit.rawValue" => json!({ "kind": "value", "value": 1.5 }),
                "settings.rtkSettings.surveyInMinObservationDuration.rawValue" => json!({ "kind": "value", "value": 120 }),
                "settings.rtkSettings.useFixedBasePosition.rawValue" => json!({ "kind": "value", "value": true }),
                "settings.rtkSettings.fixedBasePositionLatitude.rawValue" => json!({ "kind": "value", "value": 47.4 }),
                "settings.rtkSettings.fixedBasePositionLongitude.rawValue" => json!({ "kind": "value", "value": -181.0 }),
                "settings.rtkSettings.fixedBasePositionAltitude.rawValue" => json!({ "kind": "value", "value": 410.0 }),
                "settings.rtkSettings.fixedBasePositionAccuracy.rawValue" => json!({ "kind": "value", "value": 0.1 }),
                _ => json!({ "kind": "value", "value": null }),
            }
            .to_string()
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

    #[test]
    fn the_base_view_reads_the_rtk_settings_and_names_every_unit_it_scales() {
        let view = base_view(&Fake, &["Trimble".to_string()]);
        assert_eq!(view["driver"], "trimble");
        assert_eq!(view["baud"], 115200);
        assert_eq!(view["mode"], "fixedBase");
        assert_eq!(view["surveyInAccuracy"], 1.5);
        assert_eq!(view["surveyInMinDuration"], 120);
        assert_eq!(view["fixedBase"]["longitude"], 179.0, "a longitude the operator typed past the antimeridian wraps back onto the globe");
        assert_eq!(view["fixedBase"]["accuracy"], 100.0);
        assert_eq!(view["fixedBase"]["accuracyUnit"], "mm");
        assert_eq!(
            (&view["receiveTimeoutMs"], &view["threadDisconnectTimeoutMs"], &view["surveyInAccuracyDriverUnits"], &view["initialBaud"]),
            (&Value::Null, &Value::Null, &Value::Null, &Value::Null),
            "a head can draw nothing with a thread-join timeout or a driver-scaled 0.1mm figure, and these were crowding out the numbers it can"
        );
        let defaulted = base_view(&Fake, &[]);
        assert_eq!(defaulted["driver"], "ublox", "no argument means the u-blox the C++ falls back to, and the view still answers");
        assert_eq!(defaulted["headingOffsetRad"], Value::Null);
        assert_eq!(base_view(&Fake, &["septentrio".to_string()])["headingOffsetRad"], 5.0);
    }

    #[test]
    fn every_setting_the_view_reads_is_a_path_it_watches() {
        assert_eq!(
            deps(),
            vec![
                "settings.rtkSettings.surveyInAccuracyLimit.rawValue",
                "settings.rtkSettings.surveyInMinObservationDuration.rawValue",
                "settings.rtkSettings.useFixedBasePosition.rawValue",
                "settings.rtkSettings.fixedBasePositionLatitude.rawValue",
                "settings.rtkSettings.fixedBasePositionLongitude.rawValue",
                "settings.rtkSettings.fixedBasePositionAltitude.rawValue",
                "settings.rtkSettings.fixedBasePositionAccuracy.rawValue",
            ],
            "the paths are asserted as strings, since a duplicate list of the same length proved nothing about whether the two agreed"
        );
        assert_eq!(settings_from(&Fake), Settings { survey_in_accuracy_m: 1.5, survey_in_duration_s: 120, use_fixed_base: true, fixed_latitude: 47.4, fixed_longitude: -181.0, fixed_altitude_m: 410.0, fixed_accuracy_m: 0.1 });
    }
}
