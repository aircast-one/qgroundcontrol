#![allow(deprecated)]
use mavlink::dialects::ardupilotmega::*;
use mavlink::MavHeader;
use num_traits::FromPrimitive;
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use crate::mockcal::Calibration;
use crate::mocklog::Logs;
use crate::mockcamera::{Cameras, StreamKind, Video, Where};
use crate::mockgimbal::Gimbal;
use crate::transport::{Owner, Registry};

const PX4_PARAMS: &str = include_str!("../../src/Comms/MockLink/PX4MockLink.params");
const GENERIC_PARAMS: &str = include_str!("../../src/Comms/MockLink/GenericMockLink.params");
const COPTER_PARAMS: &str = include_str!("../../src/FirmwarePlugin/APM/Copter.OfflineEditing.params");
const PLANE_PARAMS: &str = include_str!("../../src/FirmwarePlugin/APM/Plane.OfflineEditing.params");
const SUB_PARAMS: &str = include_str!("../../src/FirmwarePlugin/APM/Sub.OfflineEditing.params");
const ROVER_PARAMS: &str = include_str!("../../src/FirmwarePlugin/APM/Rover.OfflineEditing.params");

pub const AUTOPILOT: u8 = 1;
const REMOTE_ID: u8 = 236;
const FIRST_SYSTEM_ID: u8 = 128;
const HOME_LAT: f64 = 47.397;
const HOME_LON: f64 = 8.5455;
const HOME_ALT_M: f64 = 488.056;
const SYSTEM_SPACING_DEG: f64 = 0.0001;
const TICK: Duration = Duration::from_millis(100);
const POLL: Duration = Duration::from_millis(20);
const PARAMS_PER_POLL: usize = 20;
const TICKS_PER_SECOND: u64 = 10;
const CLIMB_MPS: f64 = 2.5;
const DESCENT_MPS: f64 = 1.5;
const CRUISE_MPS: f64 = 5.0;
const ARRIVED_M: f64 = 1.0;
const METRES_PER_DEG: f64 = 111_320.0;
const PX4_TAKEOFF_ALT_M: f64 = 2.5;
const APM_TAKEOFF_ALT_M: f64 = 10.0;
const BATTERY_FULL_TIME_S: f64 = 15.0 * 60.0;
const ADSB_VEHICLES: usize = 5;
const ADSB_STEP_M: f64 = 5.0;
const ADSB_ICAO_BASE: u32 = 12345;
const PID_TUNING_MESSAGES: [u32; 6] = [31, 83, 32, 85, 62, 74];
const PX4_AUTO: u32 = 4;
const PX4_MANUAL: u32 = 1;
const PX4_TAKEOFF: u32 = 2;
const PX4_LOITER: u32 = 3;
const PX4_RTL: u32 = 5;
const PX4_LAND: u32 = 6;
const FTP_NAK: u8 = 129;
const SET_MODE_ID: u32 = 11;
const SET_MODE_LEN: usize = 6;
const FTP_FILE_NOT_FOUND: u8 = 10;
const FTP_DATA_AT: usize = 12;
const APM_CAL_OFFSETS: [&str; 12] = ["COMPASS_OFS_X", "COMPASS_OFS_Y", "COMPASS_OFS_Z", "COMPASS_OFS2_X", "COMPASS_OFS2_Y", "COMPASS_OFS2_Z", "COMPASS_OFS3_X", "COMPASS_OFS3_Y", "COMPASS_OFS3_Z", "INS_ACCOFFS_X", "INS_ACCOFFS_Y", "INS_ACCOFFS_Z"];
const RC_MAPS: [&str; 4] = ["RCMAP_ROLL", "RCMAP_PITCH", "RCMAP_YAW", "RCMAP_THROTTLE"];
const STATUS_TEXTS: [(MavSeverity, &str); 9] = [
    (MavSeverity::MAV_SEVERITY_INFO, "#Testing audio output"),
    (MavSeverity::MAV_SEVERITY_EMERGENCY, "Status text emergency"),
    (MavSeverity::MAV_SEVERITY_ALERT, "Status text alert"),
    (MavSeverity::MAV_SEVERITY_CRITICAL, "Status text critical"),
    (MavSeverity::MAV_SEVERITY_ERROR, "Status text error"),
    (MavSeverity::MAV_SEVERITY_WARNING, "Status text warning"),
    (MavSeverity::MAV_SEVERITY_NOTICE, "Status text notice"),
    (MavSeverity::MAV_SEVERITY_INFO, "Status text info"),
    (MavSeverity::MAV_SEVERITY_DEBUG, "Status text debug"),
];
const PROXIMITY: [MavSensorOrientation; 6] = [
    MavSensorOrientation::MAV_SENSOR_ROTATION_NONE,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_45,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_90,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_180,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_270,
    MavSensorOrientation::MAV_SENSOR_ROTATION_YAW_315,
];

static DEBUG_BUILD: AtomicBool = AtomicBool::new(false);
static VIDEO_PATTERN: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub fn set_video_pattern(pattern: &str) {
    *VIDEO_PATTERN.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = pattern.to_string();
}

pub fn video_pattern() -> String {
    VIDEO_PATTERN.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}
static NEXT_SYSTEM: AtomicU8 = AtomicU8::new(FIRST_SYSTEM_ID);
static SILENT: AtomicBool = AtomicBool::new(false);

pub fn set_silent(silent: bool) {
    SILENT.store(silent, Ordering::Relaxed);
}

pub fn silent() -> bool {
    SILENT.load(Ordering::Relaxed)
}

pub fn set_available(available: bool) {
    DEBUG_BUILD.store(available, Ordering::Relaxed);
}

pub fn available() -> bool {
    DEBUG_BUILD.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vehicle {
    Px4,
    Copter,
    Plane,
    Sub,
    Rover,
    Generic,
}

pub const VEHICLES: [(Vehicle, &str, &str); 6] = [
    (Vehicle::Px4, "px4", "PX4 Vehicle"),
    (Vehicle::Copter, "apmCopter", "APM ArduCopter Vehicle"),
    (Vehicle::Plane, "apmPlane", "APM ArduPlane Vehicle"),
    (Vehicle::Sub, "apmSub", "APM ArduSub Vehicle"),
    (Vehicle::Rover, "apmRover", "APM ArduRover Vehicle"),
    (Vehicle::Generic, "generic", "Generic Vehicle"),
];

impl Vehicle {
    pub fn from_key(key: &str) -> Option<Vehicle> {
        VEHICLES.iter().find(|(_, k, _)| *k == key).map(|(v, _, _)| *v)
    }

    pub fn link_name(self) -> &'static str {
        match self {
            Vehicle::Px4 => "PX4 MultiRotor MockLink",
            Vehicle::Copter => "ArduCopter MockLink",
            Vehicle::Plane => "ArduPlane MockLink",
            Vehicle::Sub => "ArduSub MockLink",
            Vehicle::Rover => "ArduRover MockLink",
            Vehicle::Generic => "Generic MockLink",
        }
    }

    pub fn codes(self) -> (i64, i64) {
        let (autopilot, kind) = match self {
            Vehicle::Px4 => (MavAutopilot::MAV_AUTOPILOT_PX4, MavType::MAV_TYPE_QUADROTOR),
            Vehicle::Copter => (MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA, MavType::MAV_TYPE_QUADROTOR),
            Vehicle::Plane => (MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA, MavType::MAV_TYPE_FIXED_WING),
            Vehicle::Sub => (MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA, MavType::MAV_TYPE_SUBMARINE),
            Vehicle::Rover => (MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA, MavType::MAV_TYPE_GROUND_ROVER),
            Vehicle::Generic => (MavAutopilot::MAV_AUTOPILOT_GENERIC, MavType::MAV_TYPE_QUADROTOR),
        };
        (autopilot as i64, kind as i64)
    }

    fn of(firmware: i64, kind: i64) -> Vehicle {
        let apm = firmware == MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA as i64;
        let generic = firmware == MavAutopilot::MAV_AUTOPILOT_GENERIC as i64;
        match MavType::from_i64(kind) {
            _ if generic => Vehicle::Generic,
            _ if !apm => Vehicle::Px4,
            Some(MavType::MAV_TYPE_FIXED_WING) => Vehicle::Plane,
            Some(MavType::MAV_TYPE_SUBMARINE) => Vehicle::Sub,
            Some(MavType::MAV_TYPE_GROUND_ROVER) => Vehicle::Rover,
            _ => Vehicle::Copter,
        }
    }

    fn apm(self) -> bool {
        !matches!(self, Vehicle::Px4 | Vehicle::Generic)
    }

    fn params(self) -> &'static str {
        match self {
            Vehicle::Px4 => PX4_PARAMS,
            Vehicle::Generic => GENERIC_PARAMS,
            Vehicle::Copter => COPTER_PARAMS,
            Vehicle::Plane => PLANE_PARAMS,
            Vehicle::Sub => SUB_PARAMS,
            Vehicle::Rover => ROVER_PARAMS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub vehicle: Vehicle,
    pub send_status_text: bool,
    pub enable_camera: bool,
    pub enable_gimbal: bool,
    pub enable_proximity: bool,
    pub apm_start_fresh_params: bool,
    pub increment_vehicle_id: bool,
    pub video: StreamKind,
}

impl Options {
    pub fn from_kind(kind: &crate::linkconfig::Kind) -> Option<Options> {
        match kind {
            crate::linkconfig::Kind::Mock { firmware_type, vehicle_type, send_status_text, increment_vehicle_id, enable_camera, enable_gimbal, enable_proximity, apm_start_fresh_params, video_stream_type, .. } => Some(Options {
                vehicle: Vehicle::of(*firmware_type, *vehicle_type),
                send_status_text: *send_status_text,
                enable_camera: *enable_camera,
                enable_gimbal: *enable_gimbal,
                enable_proximity: *enable_proximity,
                apm_start_fresh_params: *apm_start_fresh_params,
                increment_vehicle_id: *increment_vehicle_id,
                video: StreamKind::from_index(*video_stream_type),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Param {
    name: String,
    kind: MavParamType,
    value: f64,
}

fn parse_params(text: &str) -> Vec<Param> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let [_, component, name, value, kind] = fields.as_slice() else { return None };
            (component.trim() == "1").then_some(())?;
            Some((name.to_string(), Param { name: name.to_string(), kind: MavParamType::from_u8(kind.trim().parse().ok()?)?, value: value.trim().parse().ok()? }))
        })
        .collect::<BTreeMap<String, Param>>()
        .into_values()
        .collect()
}

fn fresh_flash(params: Vec<Param>) -> Vec<Param> {
    let channels: Vec<String> = RC_MAPS.iter().filter_map(|map| params.iter().find(|p| p.name == *map)).map(|p| (p.value as i64).to_string()).collect();
    let reset = |name: &str| -> Option<f64> {
        let rc = channels.iter().find_map(|ch| match name.strip_prefix(&format!("RC{ch}_"))? {
            "MIN" => Some(1100.0),
            "MAX" => Some(1900.0),
            "TRIM" => Some(1500.0),
            _ => None,
        });
        match name {
            "FRAME_CLASS" => Some(0.0),
            n if APM_CAL_OFFSETS.contains(&n) => Some(0.0),
            _ => rc,
        }
    };
    params.into_iter().map(|p| Param { value: reset(&p.name).unwrap_or(p.value), ..p }).collect()
}

fn wire(param: &Param, bytewise: bool) -> f32 {
    let whole = param.value as i64;
    let bits = match param.kind {
        MavParamType::MAV_PARAM_TYPE_UINT8 | MavParamType::MAV_PARAM_TYPE_INT8 => whole as u32 & 0xFF,
        MavParamType::MAV_PARAM_TYPE_UINT16 | MavParamType::MAV_PARAM_TYPE_INT16 => whole as u32 & 0xFFFF,
        _ => whole as u32,
    };
    match (param.kind, bytewise) {
        (MavParamType::MAV_PARAM_TYPE_REAL32 | MavParamType::MAV_PARAM_TYPE_REAL64, _) | (_, false) => param.value as f32,
        _ => f32::from_bits(bits),
    }
}

fn unwire(kind: MavParamType, sent: f32, bytewise: bool) -> f64 {
    let bits = sent.to_bits();
    match (kind, bytewise) {
        (MavParamType::MAV_PARAM_TYPE_REAL32 | MavParamType::MAV_PARAM_TYPE_REAL64, _) | (_, false) => sent as f64,
        (MavParamType::MAV_PARAM_TYPE_UINT8, _) => (bits & 0xFF) as f64,
        (MavParamType::MAV_PARAM_TYPE_INT8, _) => (bits as u8 as i8) as f64,
        (MavParamType::MAV_PARAM_TYPE_UINT16, _) => (bits & 0xFFFF) as f64,
        (MavParamType::MAV_PARAM_TYPE_INT16, _) => (bits as u16 as i16) as f64,
        (MavParamType::MAV_PARAM_TYPE_INT32, _) => (bits as i32) as f64,
        _ => bits as f64,
    }
}

fn text_of<const N: usize>(chars: &mavlink::types::CharArray<N>) -> String {
    chars.to_str().map(|s| s.trim_end_matches('\0').to_string()).unwrap_or_default()
}

fn degrees(e7: i32) -> f64 {
    e7 as f64 / 1e7
}

fn e7(deg: f64) -> i32 {
    (deg * 1e7).round() as i32
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Target {
    lat: f64,
    lon: f64,
    alt: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Holding,
    Returning,
    Landing,
}

struct Upload {
    kind: MavMissionType,
    count: u16,
    items: Vec<MISSION_ITEM_INT_DATA>,
}

pub type Out = (u8, MavMessage);

pub struct Sim {
    system: u8,
    options: Options,
    params: Vec<Param>,
    queued: VecDeque<usize>,
    armed: bool,
    base_mode: MavModeFlag,
    custom_mode: u32,
    home: (f64, f64),
    lat: f64,
    lon: f64,
    alt: f64,
    heading: f64,
    target: Option<Target>,
    phase: Phase,
    missions: BTreeMap<u8, Vec<MISSION_ITEM_INT_DATA>>,
    upload: Option<Upload>,
    cameras: Option<Cameras>,
    gimbal: Option<Gimbal>,
    calibration: Calibration,
    logs: Logs,
    adsb: Vec<Adsb>,
    battery: [i8; 2],
    greeted: bool,
    ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Adsb {
    lat: f64,
    lon: f64,
    alt: f64,
    angle: f64,
}

impl Sim {
    pub fn new(options: Options, system: u8, served: Option<String>) -> Sim {
        let loaded = parse_params(options.vehicle.params());
        let params = if options.vehicle.apm() && options.apm_start_fresh_params { fresh_flash(loaded) } else { loaded };
        let offset = f64::from(system.saturating_sub(FIRST_SYSTEM_ID)) * SYSTEM_SPACING_DEG;
        let home = (HOME_LAT + offset, HOME_LON + offset);
        Sim {
            system,
            options,
            params,
            queued: VecDeque::new(),
            armed: false,
            base_mode: MavModeFlag::MAV_MODE_FLAG_MANUAL_INPUT_ENABLED | MavModeFlag::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED,
            custom_mode: if options.vehicle == Vehicle::Px4 { px4_mode(PX4_MANUAL, 0) } else { 0 },
            home,
            lat: home.0,
            lon: home.1,
            alt: 0.0,
            heading: 0.0,
            target: None,
            phase: Phase::Holding,
            missions: BTreeMap::new(),
            upload: None,
            cameras: options.enable_camera.then(|| Cameras::new(Video { requested: options.video, served })),
            gimbal: options.enable_gimbal.then(Gimbal::default),
            calibration: Calibration::default(),
            logs: Logs::default(),
            adsb: (0..ADSB_VEHICLES)
                .map(|i| {
                    let step = i as f64 * 0.001;
                    Adsb { lat: HOME_LAT + step, lon: HOME_LON + if i % 2 == 0 { step } else { -step }, alt: HOME_ALT_M + i as f64 * 5.0, angle: i as f64 * 72.0 }
                })
                .collect(),
            battery: [100, 100],
            greeted: false,
            ticks: 0,
        }
    }

    fn bytewise(&self) -> bool {
        !self.options.vehicle.apm()
    }

    fn airborne(&self) -> bool {
        self.alt > 0.05
    }

    fn mine(&self, target: u8) -> bool {
        target == 0 || target == self.system
    }

    fn param_value(&self, index: usize) -> Out {
        let param = &self.params[index];
        (AUTOPILOT, MavMessage::PARAM_VALUE(PARAM_VALUE_DATA {
            param_value: wire(param, self.bytewise()),
            param_count: self.params.len() as u16,
            param_index: index as u16,
            param_id: crate::mavout::param_id(&param.name),
            param_type: param.kind,
        }))
    }

    pub fn receive(&mut self, header: MavHeader, message: &MavMessage) -> Vec<Out> {
        let from = (header.system_id, header.component_id);
        match message {
            MavMessage::HEARTBEAT(_) if !self.greeted => {
                self.greeted = true;
                match self.options.send_status_text {
                    true => STATUS_TEXTS.iter().map(|(severity, text)| (AUTOPILOT, MavMessage::STATUSTEXT(STATUSTEXT_DATA { severity: *severity, text: crate::mavout::chars(text), ..Default::default() }))).collect(),
                    false => Vec::new(),
                }
            }
            MavMessage::PARAM_REQUEST_LIST(m) if self.mine(m.target_system) && matches!(m.target_component, 0 | AUTOPILOT) => {
                self.queued = (0..self.params.len()).collect();
                Vec::new()
            }
            MavMessage::PARAM_REQUEST_READ(m) if self.mine(m.target_system) => {
                let name = text_of(&m.param_id);
                let index = match m.param_index {
                    -1 => self.params.iter().position(|p| p.name == name),
                    at => usize::try_from(at).ok().filter(|at| *at < self.params.len()),
                };
                index.map(|at| self.param_value(at)).into_iter().collect()
            }
            MavMessage::PARAM_SET(m) if self.mine(m.target_system) => {
                let name = text_of(&m.param_id);
                let bytewise = self.bytewise();
                let index = self.params.iter().position(|p| p.name == name);
                index
                    .map(|at| {
                        let kind = self.params[at].kind;
                        self.params[at].value = unwire(kind, m.param_value, bytewise);
                        self.param_value(at)
                    })
                    .into_iter()
                    .collect()
            }
            MavMessage::COMMAND_LONG(m) if self.mine(m.target_system) => {
                let params = [m.param1, m.param2, m.param3, m.param4, m.param5, m.param6, m.param7];
                let position = (f64::from(m.param5), f64::from(m.param6), f64::from(m.param7));
                self.command(from, m.target_component, m.command, params, position, MavFrame::MAV_FRAME_GLOBAL)
            }
            MavMessage::COMMAND_INT(m) if self.mine(m.target_system) => {
                let params = [m.param1, m.param2, m.param3, m.param4, m.x as f32, m.y as f32, m.z];
                let position = (if m.x == 0 { f64::NAN } else { degrees(m.x) }, if m.y == 0 { f64::NAN } else { degrees(m.y) }, f64::from(m.z));
                self.command(from, m.target_component, m.command, params, position, m.frame)
            }
            MavMessage::COMMAND_ACK(m) => {
                self.calibration.acked(m.command);
                Vec::new()
            }
            MavMessage::LOG_REQUEST_LIST(m) if self.mine(m.target_system) && (m.start == 0 || m.end == u16::MAX) => vec![self.logs.listed()],
            MavMessage::LOG_REQUEST_DATA(m) if self.mine(m.target_system) => {
                self.logs.requested(m);
                Vec::new()
            }
            MavMessage::LOG_ERASE(m) if self.mine(m.target_system) => {
                self.logs.erase();
                Vec::new()
            }
            MavMessage::SET_MODE(m) if self.mine(m.target_system) => {
                self.set_mode(m.base_mode as u8, m.custom_mode);
                Vec::new()
            }
            MavMessage::SET_POSITION_TARGET_GLOBAL_INT(m) if self.mine(m.target_system) && !m.type_mask.contains(PositionTargetTypemask::POSITION_TARGET_TYPEMASK_X_IGNORE) => {
                let alt = relative(m.coordinate_frame, f64::from(m.alt));
                self.fly_to(Target { lat: degrees(m.lat_int), lon: degrees(m.lon_int), alt });
                Vec::new()
            }
            MavMessage::SET_POSITION_TARGET_LOCAL_NED(m) if self.mine(m.target_system) && !m.type_mask.contains(PositionTargetTypemask::POSITION_TARGET_TYPEMASK_Z_IGNORE) => {
                let alt = match m.coordinate_frame {
                    MavFrame::MAV_FRAME_LOCAL_OFFSET_NED | MavFrame::MAV_FRAME_BODY_OFFSET_NED => self.target.map_or(self.alt, |t| t.alt) - f64::from(m.z),
                    _ => -f64::from(m.z),
                };
                let (lat, lon) = self.target.map_or((self.lat, self.lon), |t| (t.lat, t.lon));
                self.fly_to(Target { lat, lon, alt });
                Vec::new()
            }
            MavMessage::MISSION_COUNT(m) if self.mine(m.target_system) => {
                let kind = m.mission_type;
                match m.count {
                    0 => {
                        self.missions.insert(kind as u8, Vec::new());
                        vec![mission_ack(from, kind, MavMissionResult::MAV_MISSION_ACCEPTED)]
                    }
                    count => {
                        self.upload = Some(Upload { kind, count, items: Vec::new() });
                        vec![request_item(from, kind, 0)]
                    }
                }
            }
            MavMessage::MISSION_ITEM_INT(m) if self.mine(m.target_system) => {
                let Some(upload) = self.upload.as_mut().filter(|u| u.kind == m.mission_type && usize::from(m.seq) == u.items.len()) else { return Vec::new() };
                upload.items.push(m.clone());
                match upload.items.len() < usize::from(upload.count) {
                    true => vec![request_item(from, upload.kind, upload.items.len() as u16)],
                    false => {
                        let done = self.upload.take().expect("an upload in progress");
                        self.missions.insert(done.kind as u8, done.items);
                        vec![mission_ack(from, done.kind, MavMissionResult::MAV_MISSION_ACCEPTED)]
                    }
                }
            }
            MavMessage::MISSION_REQUEST_LIST(m) if self.mine(m.target_system) => {
                let count = self.missions.get(&(m.mission_type as u8)).map_or(0, Vec::len) as u16;
                vec![(AUTOPILOT, MavMessage::MISSION_COUNT(MISSION_COUNT_DATA { count, target_system: from.0, target_component: from.1, mission_type: m.mission_type, opaque_id: 0 }))]
            }
            MavMessage::MISSION_REQUEST_INT(m) if self.mine(m.target_system) => self.mission_item(from, m.mission_type, m.seq),
            MavMessage::MISSION_REQUEST(m) if self.mine(m.target_system) => self.mission_item(from, m.mission_type, m.seq),
            MavMessage::MISSION_CLEAR_ALL(m) if self.mine(m.target_system) => {
                self.missions.remove(&(m.mission_type as u8));
                vec![mission_ack(from, m.mission_type, MavMissionResult::MAV_MISSION_ACCEPTED)]
            }
            MavMessage::MISSION_SET_CURRENT(m) if self.mine(m.target_system) => {
                let total = self.missions.get(&(MavMissionType::MAV_MISSION_TYPE_MISSION as u8)).map_or(0, Vec::len) as u16;
                vec![(AUTOPILOT, MavMessage::MISSION_CURRENT(MISSION_CURRENT_DATA { seq: m.seq, total, ..Default::default() }))]
            }
            MavMessage::FILE_TRANSFER_PROTOCOL(m) if self.mine(m.target_system) => {
                let request = m.payload;
                let sequence = u16::from_le_bytes([request[0], request[1]]).wrapping_add(1).to_le_bytes();
                let payload: [u8; 251] = std::array::from_fn(|at| match at {
                    0 => sequence[0],
                    1 => sequence[1],
                    2 => request[2],
                    3 => FTP_NAK,
                    4 => 1,
                    5 => request[3],
                    FTP_DATA_AT => FTP_FILE_NOT_FOUND,
                    _ => 0,
                });
                vec![(AUTOPILOT, MavMessage::FILE_TRANSFER_PROTOCOL(FILE_TRANSFER_PROTOCOL_DATA { target_network: 0, target_system: from.0, target_component: from.1, payload }))]
            }
            _ => Vec::new(),
        }
    }

    pub fn receive_undecoded(&mut self, extra: &crate::transport::Extra) {
        let header = if extra.v2 { 10 } else { 6 };
        let payload: Vec<u8> = extra.raw.iter().skip(header).take(SET_MODE_LEN).copied().chain(std::iter::repeat(0)).take(SET_MODE_LEN).collect();
        if extra.msgid == SET_MODE_ID && self.mine(payload[4]) {
            self.set_mode(payload[5], u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]));
        }
    }

    fn mission_item(&self, from: (u8, u8), kind: MavMissionType, seq: u16) -> Vec<Out> {
        self.missions
            .get(&(kind as u8))
            .and_then(|items| items.get(usize::from(seq)))
            .map(|item| (AUTOPILOT, MavMessage::MISSION_ITEM_INT(MISSION_ITEM_INT_DATA { target_system: from.0, target_component: from.1, ..item.clone() })))
            .into_iter()
            .collect()
    }

    fn command(&mut self, from: (u8, u8), component: u8, command: MavCmd, params: [f32; 7], position: (f64, f64, f64), frame: MavFrame) -> Vec<Out> {
        let now_ms = self.ticks * TICK.as_millis() as u64;
        if let Some(replies) = self.cameras.as_mut().and_then(|cameras| cameras.command(now_ms, from, component, command, params)) {
            return replies;
        }
        if let Some(replies) = self.gimbal.as_mut().and_then(|gimbal| gimbal.command(from, component, command, params)) {
            return replies;
        }
        let (result, extra) = self.autopilot_command(command, params, position, frame);
        let ack = (AUTOPILOT, MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command, result, progress: 0, result_param2: 0, target_system: from.0, target_component: from.1 }));
        extra.into_iter().chain(std::iter::once(ack)).collect()
    }

    fn autopilot_command(&mut self, command: MavCmd, params: [f32; 7], position: (f64, f64, f64), frame: MavFrame) -> (MavResult, Vec<Out>) {
        let accepted = |extra: Vec<Out>| (MavResult::MAV_RESULT_ACCEPTED, extra);
        match command {
            MavCmd::MAV_CMD_COMPONENT_ARM_DISARM => {
                self.armed = params[0] != 0.0;
                accepted(Vec::new())
            }
            MavCmd::MAV_CMD_DO_SET_MODE => {
                self.set_mode(params[0] as u8, params[1] as u32);
                accepted(Vec::new())
            }
            MavCmd::MAV_CMD_NAV_TAKEOFF => {
                self.takeoff(f64::from(params[6]));
                accepted(Vec::new())
            }
            MavCmd::MAV_CMD_NAV_LAND => {
                self.phase = Phase::Landing;
                accepted(Vec::new())
            }
            MavCmd::MAV_CMD_NAV_RETURN_TO_LAUNCH => {
                self.phase = Phase::Returning;
                accepted(Vec::new())
            }
            MavCmd::MAV_CMD_DO_REPOSITION => {
                let current = self.target.unwrap_or(Target { lat: self.lat, lon: self.lon, alt: self.alt });
                let (lat, lon, alt) = position;
                self.fly_to(Target {
                    lat: if lat.is_nan() { current.lat } else { lat },
                    lon: if lon.is_nan() { current.lon } else { lon },
                    alt: if alt.is_nan() { current.alt } else { relative(frame, alt) },
                });
                accepted(Vec::new())
            }
            MavCmd::MAV_CMD_DO_CHANGE_ALTITUDE => {
                let current = self.target.unwrap_or(Target { lat: self.lat, lon: self.lon, alt: self.alt });
                self.fly_to(Target { alt: relative(frame, f64::from(params[0])), ..current });
                accepted(Vec::new())
            }
            MavCmd::MAV_CMD_REQUEST_AUTOPILOT_CAPABILITIES => accepted(vec![self.autopilot_version()]),
            MavCmd::MAV_CMD_REQUEST_MESSAGE => match params[0] as u32 {
                148 => accepted(vec![self.autopilot_version()]),
                242 => accepted(vec![self.home_position()]),
                _ => (MavResult::MAV_RESULT_UNSUPPORTED, Vec::new()),
            },
            MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL if PID_TUNING_MESSAGES.contains(&(params[0] as u32)) => accepted(Vec::new()),
            MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION | MavCmd::MAV_CMD_DO_START_MAG_CAL | MavCmd::MAV_CMD_DO_CANCEL_MAG_CAL => self.calibration.command(self.options.vehicle.apm(), command, params),
            MavCmd::MAV_CMD_MISSION_START
            | MavCmd::MAV_CMD_DO_MOTOR_TEST
            | MavCmd::MAV_CMD_PREFLIGHT_STORAGE
            | MavCmd::MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN => accepted(Vec::new()),
            _ => (MavResult::MAV_RESULT_UNSUPPORTED, Vec::new()),
        }
    }

    fn set_mode(&mut self, base: u8, custom: u32) {
        self.base_mode = MavModeFlag::from_bits_truncate(base) - MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED;
        self.custom_mode = custom;
        let vehicle = self.options.vehicle;
        let (main, sub) = ((custom >> 16) & 0xFF, (custom >> 24) & 0xFF);
        match vehicle {
            Vehicle::Px4 if main == PX4_AUTO && sub == PX4_LAND => self.phase = Phase::Landing,
            Vehicle::Px4 if main == PX4_AUTO && sub == PX4_RTL => self.phase = Phase::Returning,
            Vehicle::Px4 if main == PX4_AUTO && sub == PX4_TAKEOFF && self.armed => self.takeoff(f64::NAN),
            Vehicle::Copter if custom == 9 => self.phase = Phase::Landing,
            Vehicle::Copter if matches!(custom, 6 | 21) => self.phase = Phase::Returning,
            Vehicle::Plane if custom == 20 => self.phase = Phase::Landing,
            Vehicle::Plane if matches!(custom, 11 | 21) => self.phase = Phase::Returning,
            Vehicle::Rover if matches!(custom, 11 | 12) => self.phase = Phase::Returning,
            _ => {}
        }
    }

    fn takeoff(&mut self, asked: f64) {
        let px4 = self.options.vehicle == Vehicle::Px4;
        let alt = match (asked.is_nan() || asked <= 0.0, px4) {
            (true, true) => PX4_TAKEOFF_ALT_M,
            (true, false) => APM_TAKEOFF_ALT_M,
            (false, true) => (asked - HOME_ALT_M).max(PX4_TAKEOFF_ALT_M),
            (false, false) => asked,
        };
        self.armed = true;
        if px4 {
            self.custom_mode = px4_mode(PX4_AUTO, PX4_TAKEOFF);
        }
        self.fly_to(Target { lat: self.lat, lon: self.lon, alt });
    }

    fn fly_to(&mut self, target: Target) {
        self.phase = Phase::Holding;
        self.target = Some(target);
    }

    fn step(&mut self, dt: f64) {
        let destination = match self.phase {
            Phase::Returning => Some(Target { lat: self.home.0, lon: self.home.1, alt: self.alt.max(self.target.map_or(0.0, |t| t.alt)) }),
            Phase::Landing => Some(Target { lat: self.lat, lon: self.lon, alt: 0.0 }),
            Phase::Holding => self.target.filter(|_| self.armed),
        };
        let Some(goal) = destination else { return };
        let north = (goal.lat - self.lat) * METRES_PER_DEG;
        let east = (goal.lon - self.lon) * METRES_PER_DEG * self.lat.to_radians().cos();
        let distance = north.hypot(east);
        let travel = (CRUISE_MPS * dt).min(distance);
        if distance > f64::EPSILON {
            self.heading = east.atan2(north).to_degrees().rem_euclid(360.0);
            self.lat += north / distance * travel / METRES_PER_DEG;
            self.lon += east / distance * travel / (METRES_PER_DEG * self.lat.to_radians().cos());
        }
        let rate = if goal.alt < self.alt { DESCENT_MPS } else { CLIMB_MPS };
        self.alt += (goal.alt - self.alt).clamp(-rate * dt, rate * dt);
        let arrived = distance - travel < ARRIVED_M && (goal.alt - self.alt).abs() < 0.05;
        match self.phase {
            Phase::Returning if arrived => self.phase = Phase::Landing,
            Phase::Landing if self.alt <= 0.0 => {
                self.alt = 0.0;
                self.armed = false;
                self.target = None;
                self.phase = Phase::Holding;
            }
            Phase::Holding if arrived && self.options.vehicle == Vehicle::Px4 && self.custom_mode == px4_mode(PX4_AUTO, PX4_TAKEOFF) => self.custom_mode = px4_mode(PX4_AUTO, PX4_LOITER),
            _ => {}
        }
    }

    pub fn pump(&mut self) -> Vec<Out> {
        let count = PARAMS_PER_POLL.min(self.queued.len());
        let batch: Vec<usize> = self.queued.drain(..count).collect();
        let logs = self.logs.pump();
        batch.into_iter().map(|at| self.param_value(at)).chain(logs).collect()
    }

    pub fn tick(&mut self, elapsed_ms: u64) -> Vec<Out> {
        self.step(TICK.as_secs_f64());
        let slow = self.ticks % TICKS_PER_SECOND == 0;
        self.ticks += 1;
        let here = Where { lat: e7(self.lat), lon: e7(self.lon), alt_mm: ((HOME_ALT_M + self.alt) * 1000.0) as i32 };
        let fast = vec![self.global_position(elapsed_ms), self.attitude(elapsed_ms), self.vfr_hud()];
        let cameras = self.cameras.as_mut().map(|cameras| cameras.tick(elapsed_ms, &here)).unwrap_or_default();
        let (calibration, stored) = self.calibration.tick();
        self.params = std::mem::take(&mut self.params).into_iter().map(|p| Param { value: stored.iter().find(|(name, _)| *name == p.name).map_or(p.value, |(_, value)| *value), ..p }).collect();
        let every_second = if slow { self.once_a_second(elapsed_ms) } else { Vec::new() };
        fast.into_iter().chain(cameras).chain(calibration).chain(every_second).collect()
    }

    fn once_a_second(&mut self, elapsed_ms: u64) -> Vec<Out> {
        let seconds = elapsed_ms / 1000;
        self.battery = [drained(self.battery[0], seconds), drained(self.battery[1], seconds / 2)];
        self.adsb = self.adsb.iter().enumerate().map(|(i, plane)| plane.advanced(i)).collect();
        let core = vec![vibration(), self.battery_status(1), self.battery_status(2)]
            .into_iter()
            .chain(named_values(elapsed_ms))
            .chain([self.sys_status()])
            .chain(self.adsb.iter().enumerate().map(|(i, plane)| plane.message(i)))
            .chain((self.options.vehicle != Vehicle::Sub).then(remote_id_arm_status))
            .chain(self.options.enable_proximity.then(|| distance_sensors(elapsed_ms)).unwrap_or_default())
            .chain([esc_info(elapsed_ms), esc_status(elapsed_ms), radio_status()]);
        let gimbal = self.gimbal.as_mut().map(|gimbal| gimbal.once_a_second(elapsed_ms)).unwrap_or_default();
        let cameras = self.cameras.as_ref().map(Cameras::heartbeats).unwrap_or_default();
        let rest = vec![rc_channels(), self.home_position(), self.heartbeat(), self.gps(elapsed_ms), self.extended_state()];
        core.chain(gimbal).chain(cameras).chain(rest).collect()
    }

    fn heartbeat(&self) -> Out {
        let (autopilot, kind) = self.options.vehicle.codes();
        let armed = if self.armed { MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED } else { MavModeFlag::empty() };
        (AUTOPILOT, MavMessage::HEARTBEAT(HEARTBEAT_DATA {
            custom_mode: self.custom_mode,
            mavtype: MavType::from_i64(kind).unwrap_or(MavType::MAV_TYPE_QUADROTOR),
            autopilot: MavAutopilot::from_i64(autopilot).unwrap_or(MavAutopilot::MAV_AUTOPILOT_GENERIC),
            base_mode: self.base_mode | armed,
            system_status: if self.airborne() { MavState::MAV_STATE_ACTIVE } else { MavState::MAV_STATE_STANDBY },
            mavlink_version: 3,
        }))
    }

    fn sys_status(&self) -> Out {
        (AUTOPILOT, MavMessage::SYS_STATUS(SYS_STATUS_DATA {
            onboard_control_sensors_present: MavSysStatusSensor::MAV_SYS_STATUS_SENSOR_GPS,
            load: 250,
            voltage_battery: 4200 * 4,
            current_battery: 8000,
            battery_remaining: self.battery[0],
            ..Default::default()
        }))
    }

    fn battery_status(&self, id: u8) -> Out {
        let percent = self.battery[usize::from(id - 1)];
        let first = id == 1;
        (AUTOPILOT, MavMessage::BATTERY_STATUS(BATTERY_STATUS_DATA {
            id,
            battery_function: MavBatteryFunction::MAV_BATTERY_FUNCTION_ALL,
            mavtype: MavBatteryType::MAV_BATTERY_TYPE_LIPO,
            temperature: if first { 2100 } else { i16::MAX },
            voltages: std::array::from_fn(|cell| if first && cell < 3 { 4200 } else { u16::MAX }),
            current_battery: 600,
            current_consumed: 100,
            energy_consumed: -1,
            battery_remaining: percent,
            time_remaining: (BATTERY_FULL_TIME_S * f64::from(percent) / 100.0) as i32,
            charge_state: charge_state(percent),
            ..Default::default()
        }))
    }

    fn gps(&self, elapsed_ms: u64) -> Out {
        (AUTOPILOT, MavMessage::GPS_RAW_INT(GPS_RAW_INT_DATA {
            time_usec: elapsed_ms * 1000,
            lat: e7(self.lat),
            lon: e7(self.lon),
            alt: ((HOME_ALT_M + self.alt) * 1000.0) as i32,
            eph: 100,
            epv: 100,
            vel: u16::MAX,
            cog: (self.heading * 100.0) as u16,
            fix_type: GpsFixType::GPS_FIX_TYPE_3D_FIX,
            satellites_visible: 12,
            ..Default::default()
        }))
    }

    fn global_position(&self, elapsed_ms: u64) -> Out {
        (AUTOPILOT, MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA {
            time_boot_ms: elapsed_ms as u32,
            lat: e7(self.lat),
            lon: e7(self.lon),
            alt: ((HOME_ALT_M + self.alt) * 1000.0) as i32,
            relative_alt: (self.alt * 1000.0) as i32,
            hdg: (self.heading * 100.0) as u16,
            ..Default::default()
        }))
    }

    fn attitude(&self, elapsed_ms: u64) -> Out {
        (AUTOPILOT, MavMessage::ATTITUDE(ATTITUDE_DATA { time_boot_ms: elapsed_ms as u32, yaw: self.heading.to_radians() as f32, ..Default::default() }))
    }

    fn vfr_hud(&self) -> Out {
        let moving = self.armed && self.target.is_some_and(|t| (t.lat - self.lat).abs() + (t.lon - self.lon).abs() > 1e-7);
        (AUTOPILOT, MavMessage::VFR_HUD(VFR_HUD_DATA {
            groundspeed: if moving { CRUISE_MPS as f32 } else { 0.0 },
            airspeed: if moving { CRUISE_MPS as f32 } else { 0.0 },
            alt: (HOME_ALT_M + self.alt) as f32,
            heading: self.heading as i16,
            throttle: if self.armed { 50 } else { 0 },
            ..Default::default()
        }))
    }

    fn extended_state(&self) -> Out {
        let landed = match (self.airborne(), self.phase) {
            (false, _) => MavLandedState::MAV_LANDED_STATE_ON_GROUND,
            (true, Phase::Landing) => MavLandedState::MAV_LANDED_STATE_LANDING,
            (true, _) => MavLandedState::MAV_LANDED_STATE_IN_AIR,
        };
        (AUTOPILOT, MavMessage::EXTENDED_SYS_STATE(EXTENDED_SYS_STATE_DATA { vtol_state: MavVtolState::MAV_VTOL_STATE_UNDEFINED, landed_state: landed }))
    }

    fn home_position(&self) -> Out {
        (AUTOPILOT, MavMessage::HOME_POSITION(HOME_POSITION_DATA { latitude: e7(self.home.0), longitude: e7(self.home.1), altitude: (HOME_ALT_M * 1000.0) as i32, q: [1.0, 0.0, 0.0, 0.0], ..Default::default() }))
    }

    fn autopilot_version(&self) -> Out {
        let apm = self.options.vehicle.apm();
        let version: u32 = match self.options.vehicle {
            Vehicle::Generic => 0,
            _ if apm => (4 << 24) | (7 << 16),
            _ => (1 << 24) | (17 << 16),
        } | FirmwareVersionType::FIRMWARE_VERSION_TYPE_OFFICIAL as u32;
        let capabilities = MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_MAVLINK2
            | MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_MISSION_FENCE
            | MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_MISSION_RALLY
            | MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_MISSION_INT
            | MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_COMMAND_INT
            | if apm { MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_TERRAIN } else { MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_PARAM_ENCODE_BYTEWISE };
        (AUTOPILOT, MavMessage::AUTOPILOT_VERSION(AUTOPILOT_VERSION_DATA { capabilities, flight_sw_version: version, ..Default::default() }))
    }
}

fn px4_mode(main: u32, sub: u32) -> u32 {
    (main << 16) | (sub << 24)
}

fn relative(frame: MavFrame, alt: f64) -> f64 {
    match frame {
        MavFrame::MAV_FRAME_GLOBAL | MavFrame::MAV_FRAME_GLOBAL_INT => alt - HOME_ALT_M,
        _ => alt,
    }
}

fn mission_ack(to: (u8, u8), kind: MavMissionType, result: MavMissionResult) -> Out {
    (AUTOPILOT, MavMessage::MISSION_ACK(MISSION_ACK_DATA { target_system: to.0, target_component: to.1, mavtype: result, mission_type: kind, opaque_id: 0 }))
}

fn request_item(to: (u8, u8), kind: MavMissionType, seq: u16) -> Out {
    (AUTOPILOT, MavMessage::MISSION_REQUEST_INT(MISSION_REQUEST_INT_DATA { seq, target_system: to.0, target_component: to.1, mission_type: kind }))
}

fn rc_channels() -> Out {
    (AUTOPILOT, MavMessage::RC_CHANNELS(RC_CHANNELS_DATA {
        chancount: 16,
        chan1_raw: 1500,
        chan2_raw: 1500,
        chan3_raw: 1500,
        chan4_raw: 1500,
        chan5_raw: 1500,
        chan6_raw: 1500,
        chan7_raw: 1500,
        chan8_raw: 1500,
        chan9_raw: 1500,
        chan10_raw: 1500,
        chan11_raw: 1500,
        chan12_raw: 1500,
        chan13_raw: 1500,
        chan14_raw: 1500,
        chan15_raw: 1500,
        chan16_raw: 1500,
        chan17_raw: u16::MAX,
        chan18_raw: u16::MAX,
        ..Default::default()
    }))
}

fn drained(percent: i8, seconds: u64) -> i8 {
    match percent > 1 {
        true => 100u64.saturating_sub(seconds) as i8,
        false => percent,
    }
}

fn charge_state(percent: i8) -> MavBatteryChargeState {
    match percent {
        p if p > 50 => MavBatteryChargeState::MAV_BATTERY_CHARGE_STATE_OK,
        p if p > 30 => MavBatteryChargeState::MAV_BATTERY_CHARGE_STATE_LOW,
        p if p > 20 => MavBatteryChargeState::MAV_BATTERY_CHARGE_STATE_CRITICAL,
        _ => MavBatteryChargeState::MAV_BATTERY_CHARGE_STATE_EMERGENCY,
    }
}

fn vibration() -> Out {
    (AUTOPILOT, MavMessage::VIBRATION(VIBRATION_DATA { vibration_x: 50.5, vibration_y: 10.5, vibration_z: 60.0, clipping_0: 1, clipping_1: 2, clipping_2: 3, ..Default::default() }))
}

fn named_values(elapsed_ms: u64) -> Vec<Out> {
    let seconds = elapsed_ms as f64 / 1000.0;
    [("sin_wave", seconds.sin()), ("cos_wave", seconds.cos())]
        .into_iter()
        .map(|(name, value)| (AUTOPILOT, MavMessage::NAMED_VALUE_FLOAT(NAMED_VALUE_FLOAT_DATA { time_boot_ms: elapsed_ms as u32, value: value as f32, name: crate::mavout::chars(name) })))
        .collect()
}

fn remote_id_arm_status() -> Out {
    (REMOTE_ID, MavMessage::OPEN_DRONE_ID_ARM_STATUS(OPEN_DRONE_ID_ARM_STATUS_DATA { status: MavOdidArmStatus::MAV_ODID_ARM_STATUS_GOOD_TO_ARM, error: crate::mavout::chars("") }))
}

fn esc_info(elapsed_ms: u64) -> Out {
    (AUTOPILOT, MavMessage::ESC_INFO(ESC_INFO_DATA { time_usec: elapsed_ms * 1000, count: 4, connection_type: EscConnectionType::ESC_CONNECTION_TYPE_DSHOT, info: 0x0F, temperature: [3000; 4], ..Default::default() }))
}

fn esc_status(elapsed_ms: u64) -> Out {
    (AUTOPILOT, MavMessage::ESC_STATUS(ESC_STATUS_DATA { time_usec: elapsed_ms * 1000, rpm: [5000; 4], voltage: [16.0; 4], current: [5.0; 4], index: 0 }))
}

fn radio_status() -> Out {
    (AUTOPILOT, MavMessage::RADIO_STATUS(RADIO_STATUS_DATA { rssi: 100, remrssi: 100, txbuf: 50, noise: 10, remnoise: 10, rxerrors: 0, fixed: 0 }))
}

impl Adsb {
    fn advanced(&self, index: usize) -> Adsb {
        let angle = self.angle + (index + 1) as f64;
        let bearing = angle.to_radians();
        let north = ADSB_STEP_M * bearing.cos() / METRES_PER_DEG;
        let east = ADSB_STEP_M * bearing.sin() / (METRES_PER_DEG * self.lat.to_radians().cos());
        Adsb { lat: self.lat + north, lon: self.lon + east, alt: self.alt + if index % 2 == 0 { 0.5 } else { -0.5 }, angle }
    }

    fn message(&self, index: usize) -> Out {
        (AUTOPILOT, MavMessage::ADSB_VEHICLE(ADSB_VEHICLE_DATA {
            ICAO_address: ADSB_ICAO_BASE + index as u32,
            lat: e7(self.lat),
            lon: e7(self.lon),
            altitude_type: AdsbAltitudeType::ADSB_ALTITUDE_TYPE_GEOMETRIC,
            altitude: (self.alt * 1000.0) as i32,
            heading: (self.angle.rem_euclid(360.0) * 100.0) as u16,
            callsign: crate::mavout::chars(&format!("N12345{index:02}")),
            emitter_type: AdsbEmitterType::ADSB_EMITTER_TYPE_ROTOCRAFT,
            tslc: 1,
            flags: AdsbFlags::ADSB_FLAGS_VALID_COORDS | AdsbFlags::ADSB_FLAGS_VALID_ALTITUDE | AdsbFlags::ADSB_FLAGS_VALID_HEADING | AdsbFlags::ADSB_FLAGS_VALID_CALLSIGN | AdsbFlags::ADSB_FLAGS_SIMULATED,
            ..Default::default()
        }))
    }
}

fn distance_sensors(elapsed_ms: u64) -> Vec<Out> {
    PROXIMITY
        .iter()
        .enumerate()
        .map(|(at, orientation)| {
            let sweep = (elapsed_ms as f64 / 5000.0 + at as f64 * std::f64::consts::FRAC_PI_4).sin();
            (AUTOPILOT, MavMessage::DISTANCE_SENSOR(DISTANCE_SENSOR_DATA {
                time_boot_ms: elapsed_ms as u32,
                min_distance: 20,
                max_distance: 4000,
                current_distance: (2000.0 + 1500.0 * sweep) as u16,
                mavtype: MavDistanceSensor::MAV_DISTANCE_SENSOR_LASER,
                id: at as u8,
                orientation: *orientation,
                covariance: 255,
                ..Default::default()
            }))
        })
        .collect()
}

fn encode(system: u8, sequences: &mut BTreeMap<u8, u8>, (component, message): &Out) -> Vec<u8> {
    let sequence = sequences.entry(*component).or_insert(0);
    let header = MavHeader { system_id: system, component_id: *component, sequence: *sequence };
    *sequence = sequence.wrapping_add(1);
    let mut bytes = Vec::new();
    mavlink::write_v2_msg(&mut bytes, header, message).map(|_| bytes).unwrap_or_default()
}

#[cfg(all(feature = "jni-host", not(test)))]
fn serve(kind: StreamKind) -> Option<crate::androidvideo::MockStream> {
    crate::androidvideo::mock_serve(kind.index() as i32, kind.port(), &video_pattern())
}

#[cfg(not(all(feature = "jni-host", not(test))))]
struct NoStream {
    uri: String,
}

#[cfg(not(all(feature = "jni-host", not(test))))]
fn serve(_: StreamKind) -> Option<NoStream> {
    None
}

pub struct MockLink {
    inbox: Sender<Vec<u8>>,
    alive: std::sync::Arc<AtomicBool>,
}

impl MockLink {
    pub fn open(options: Options, deliver: impl Fn(&[u8]) + Send + 'static) -> Result<MockLink, String> {
        let system = match options.increment_vehicle_id {
            true => NEXT_SYSTEM.fetch_add(1, Ordering::Relaxed),
            false => NEXT_SYSTEM.load(Ordering::Relaxed),
        };
        let (inbox, incoming) = mpsc::channel::<Vec<u8>>();
        let alive = std::sync::Arc::new(AtomicBool::new(true));
        let running = alive.clone();
        std::thread::Builder::new()
            .name("groundstation-mock-link".to_string())
            .spawn(move || {
                let stream = (options.enable_camera && options.video != StreamKind::None).then(|| serve(options.video)).flatten();
                let mut sim = Sim::new(options, system, stream.as_ref().map(|s| s.uri.clone()));
                let mut parser = Registry::default();
                let link = parser.open(Owner::Core, "mock", "parser");
                let mut sequences = BTreeMap::new();
                let started = Instant::now();
                let mut next_tick = started;
                while running.load(Ordering::Relaxed) {
                    let replies: Vec<Out> = match incoming.recv_timeout(POLL) {
                        Ok(bytes) => {
                            let decoded: Vec<Out> = parser.bytes_in(link, &bytes).iter().flat_map(|frame| sim.receive(frame.header, &frame.message)).collect();
                            parser.take_extras(link).iter().for_each(|extra| sim.receive_undecoded(extra));
                            decoded
                        }
                        Err(RecvTimeoutError::Timeout) => Vec::new(),
                        Err(RecvTimeoutError::Disconnected) => return,
                    };
                    let telemetry = match Instant::now() >= next_tick {
                        true => {
                            next_tick += TICK;
                            sim.tick(started.elapsed().as_millis() as u64)
                        }
                        false => Vec::new(),
                    };
                    let outgoing: Vec<u8> = replies.iter().chain(&telemetry).chain(&sim.pump()).flat_map(|out| encode(system, &mut sequences, out)).collect();
                    if !outgoing.is_empty() && running.load(Ordering::Relaxed) && !silent() {
                        deliver(&outgoing);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(MockLink { inbox, alive })
    }

    pub fn write(&self, bytes: &[u8]) -> bool {
        self.inbox.send(bytes.to_vec()).is_ok()
    }

    pub fn close(&self) {
        self.alive.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(vehicle: Vehicle) -> Options {
        Options { vehicle, send_status_text: false, enable_camera: false, enable_gimbal: false, enable_proximity: false, apm_start_fresh_params: false, increment_vehicle_id: true, video: StreamKind::None }
    }

    fn gcs() -> MavHeader {
        MavHeader { system_id: 255, component_id: 190, sequence: 0 }
    }

    fn command(sim: &mut Sim, command: MavCmd, params: [f32; 7]) -> Vec<Out> {
        let [param1, param2, param3, param4, param5, param6, param7] = params;
        sim.receive(gcs(), &MavMessage::COMMAND_LONG(COMMAND_LONG_DATA { param1, param2, param3, param4, param5, param6, param7, command, target_system: sim.system, target_component: 1, confirmation: 0 }))
    }

    fn fly(sim: &mut Sim, seconds: u64) {
        (0..seconds * TICKS_PER_SECOND).for_each(|n| {
            sim.tick(n * 100);
        });
    }

    #[test]
    fn every_seed_file_loads_and_a_list_request_streams_each_parameter_once() {
        VEHICLES.iter().for_each(|(vehicle, _, _)| {
            let mut sim = Sim::new(options(*vehicle), 128, None);
            assert!(!sim.params.is_empty(), "{vehicle:?}");
            sim.receive(gcs(), &MavMessage::PARAM_REQUEST_LIST(PARAM_REQUEST_LIST_DATA { target_system: 128, target_component: 1 }));
            let sent: usize = std::iter::from_fn(|| Some(sim.pump()).filter(|batch| !batch.is_empty())).map(|batch| batch.len()).sum();
            assert_eq!(sent, sim.params.len(), "{vehicle:?}");
        });
        assert!(parse_params(COPTER_PARAMS).len() > 1000);
    }

    #[test]
    fn integer_parameters_round_trip_bytewise_on_px4_and_by_value_on_ardupilot() {
        let int8 = Param { name: "X".into(), kind: MavParamType::MAV_PARAM_TYPE_INT8, value: -1.0 };
        assert_eq!(wire(&int8, true).to_bits(), 0xFF);
        assert_eq!(unwire(int8.kind, wire(&int8, true), true), -1.0);
        assert_eq!(wire(&int8, false), -1.0);
        let int32 = Param { name: "Y".into(), kind: MavParamType::MAV_PARAM_TYPE_INT32, value: 123456.0 };
        assert_eq!(unwire(int32.kind, wire(&int32, true), true), 123456.0);
        let mut sim = Sim::new(options(Vehicle::Px4), 128, None);
        let at = sim.params.iter().position(|p| p.kind == MavParamType::MAV_PARAM_TYPE_INT32).unwrap();
        let name = sim.params[at].name.clone();
        let set = Param { value: 7.0, ..sim.params[at].clone() };
        let echoed = sim.receive(gcs(), &MavMessage::PARAM_SET(PARAM_SET_DATA { param_value: wire(&set, true), target_system: 128, target_component: 1, param_id: crate::mavout::param_id(&name), param_type: set.kind }));
        assert!(matches!(&echoed[..], [(_, MavMessage::PARAM_VALUE(v))] if v.param_value.to_bits() == 7));
        assert_eq!(sim.params[at].value, 7.0);
    }

    #[test]
    fn a_fresh_flash_clears_the_frame_and_calibration_offsets() {
        let fresh = Sim::new(Options { apm_start_fresh_params: true, ..options(Vehicle::Copter) }, 128, None);
        let value = |name: &str| fresh.params.iter().find(|p| p.name == name).map(|p| p.value);
        assert_eq!((value("FRAME_CLASS"), value("COMPASS_OFS_X"), value("INS_ACCOFFS_Z")), (Some(0.0), Some(0.0), Some(0.0)));
        let stock = Sim::new(options(Vehicle::Copter), 128, None);
        assert_ne!(stock.params.iter().find(|p| p.name == "FRAME_CLASS").map(|p| p.value), Some(0.0));
    }

    #[test]
    fn a_takeoff_climbs_and_a_land_mode_brings_it_down_and_disarms() {
        let mut sim = Sim::new(options(Vehicle::Copter), 128, None);
        command(&mut sim, MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let ack = command(&mut sim, MavCmd::MAV_CMD_NAV_TAKEOFF, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 10.0]);
        assert!(matches!(&ack[..], [(1, MavMessage::COMMAND_ACK(a))] if a.result == MavResult::MAV_RESULT_ACCEPTED && a.target_system == 255));
        fly(&mut sim, 6);
        assert!((sim.alt - 10.0).abs() < 0.1 && sim.armed, "climbed to {}", sim.alt);
        command(&mut sim, MavCmd::MAV_CMD_DO_SET_MODE, [1.0, 9.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        fly(&mut sim, 10);
        assert_eq!((sim.alt, sim.armed), (0.0, false));
    }

    #[test]
    fn px4_takes_off_to_an_amsl_altitude_then_loiters_and_returns_home_on_rtl() {
        let mut sim = Sim::new(options(Vehicle::Px4), 128, None);
        command(&mut sim, MavCmd::MAV_CMD_NAV_TAKEOFF, [0.0, 0.0, 0.0, 0.0, f32::NAN, f32::NAN, (HOME_ALT_M + 5.0) as f32]);
        fly(&mut sim, 4);
        assert_eq!(sim.custom_mode, px4_mode(PX4_AUTO, PX4_LOITER));
        assert!((sim.alt - 5.0).abs() < 0.1);
        sim.receive(gcs(), &MavMessage::COMMAND_INT(COMMAND_INT_DATA { command: MavCmd::MAV_CMD_DO_REPOSITION, x: e7(sim.home.0 + 0.0005), y: e7(sim.home.1), z: 5.0, frame: MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT, param1: -1.0, target_system: 128, target_component: 1, ..Default::default() }));
        fly(&mut sim, 15);
        assert!((sim.lat - (sim.home.0 + 0.0005)).abs() < 1e-5, "went to {}", sim.lat);
        command(&mut sim, MavCmd::MAV_CMD_DO_SET_MODE, [1.0, px4_mode(PX4_AUTO, PX4_RTL) as f32, 0.0, 0.0, 0.0, 0.0, 0.0]);
        fly(&mut sim, 25);
        assert!((sim.lat - sim.home.0).abs() < 1e-5 && sim.alt == 0.0 && !sim.armed);
    }

    #[test]
    fn a_set_mode_whose_base_mode_the_dialect_cannot_name_still_lands_the_vehicle() {
        let mut sim = Sim::new(options(Vehicle::Px4), 128, None);
        command(&mut sim, MavCmd::MAV_CMD_NAV_TAKEOFF, [0.0, 0.0, 0.0, 0.0, f32::NAN, f32::NAN, f32::NAN]);
        fly(&mut sim, 3);
        let custom = px4_mode(PX4_AUTO, PX4_LAND).to_le_bytes();
        let payload = [custom[0], custom[1], custom[2], custom[3], 128, 0x81];
        let raw: Vec<u8> = [0xFD, 6, 0, 0, 0, 255, 190, 11, 0, 0].into_iter().chain(payload).chain([0, 0]).collect();
        sim.receive_undecoded(&crate::transport::Extra { link: 1, v2: true, header: gcs(), msgid: SET_MODE_ID, raw });
        fly(&mut sim, 5);
        assert_eq!((sim.alt, sim.armed, sim.custom_mode), (0.0, false, px4_mode(PX4_AUTO, PX4_LAND)));
    }

    #[test]
    fn a_mission_uploads_and_downloads_item_by_item() {
        let mut sim = Sim::new(options(Vehicle::Copter), 128, None);
        let first = sim.receive(gcs(), &MavMessage::MISSION_COUNT(MISSION_COUNT_DATA { count: 2, target_system: 128, target_component: 1, ..Default::default() }));
        assert!(matches!(&first[..], [(_, MavMessage::MISSION_REQUEST_INT(r))] if r.seq == 0));
        let item = |seq| MavMessage::MISSION_ITEM_INT(MISSION_ITEM_INT_DATA { seq, target_system: 128, target_component: 1, command: MavCmd::MAV_CMD_NAV_WAYPOINT, ..Default::default() });
        assert!(matches!(&sim.receive(gcs(), &item(0))[..], [(_, MavMessage::MISSION_REQUEST_INT(r))] if r.seq == 1));
        assert!(matches!(&sim.receive(gcs(), &item(1))[..], [(_, MavMessage::MISSION_ACK(a))] if a.mavtype == MavMissionResult::MAV_MISSION_ACCEPTED));
        let count = sim.receive(gcs(), &MavMessage::MISSION_REQUEST_LIST(MISSION_REQUEST_LIST_DATA { target_system: 128, target_component: 1, ..Default::default() }));
        assert!(matches!(&count[..], [(_, MavMessage::MISSION_COUNT(c))] if c.count == 2 && c.target_system == 255));
        let back = sim.receive(gcs(), &MavMessage::MISSION_REQUEST_INT(MISSION_REQUEST_INT_DATA { seq: 1, target_system: 128, target_component: 1, ..Default::default() }));
        assert!(matches!(&back[..], [(_, MavMessage::MISSION_ITEM_INT(i))] if i.seq == 1 && i.target_system == 255));
    }

    #[test]
    fn options_gate_the_camera_gimbal_proximity_and_status_text() {
        let plain = Sim::new(options(Vehicle::Px4), 128, None).once_a_second(0);
        let all = Sim::new(Options { enable_camera: true, enable_gimbal: true, enable_proximity: true, ..options(Vehicle::Px4) }, 128, None).once_a_second(0);
        assert_eq!(all.len() - plain.len(), PROXIMITY.len() + 2, "six distance sensors and a heartbeat from each camera");
        assert_eq!(plain.iter().filter(|(_, m)| matches!(m, MavMessage::ADSB_VEHICLE(_))).count(), ADSB_VEHICLES);
        let mut quiet = Sim::new(options(Vehicle::Px4), 128, None);
        let mut chatty = Sim::new(Options { send_status_text: true, ..options(Vehicle::Px4) }, 128, None);
        let hello = MavMessage::HEARTBEAT(HEARTBEAT_DATA::default());
        assert!(quiet.receive(gcs(), &hello).is_empty());
        assert_eq!(chatty.receive(gcs(), &hello).len(), STATUS_TEXTS.len());
        assert!(chatty.receive(gcs(), &hello).is_empty(), "the status texts go out once");
        let other = MavMessage::PARAM_REQUEST_LIST(PARAM_REQUEST_LIST_DATA { target_system: 7, target_component: 1 });
        quiet.receive(gcs(), &other);
        assert!(quiet.pump().is_empty(), "a request for another system is not ours to answer");
    }

    #[test]
    fn calibration_texts_precede_the_ack_and_ardupilot_accel_offsets_land_in_the_parameters() {
        let mut sim = Sim::new(Options { apm_start_fresh_params: true, ..options(Vehicle::Copter) }, 128, None);
        let started = command(&mut sim, MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(matches!(&started[..], [(_, MavMessage::STATUSTEXT(t)), (_, MavMessage::COMMAND_ACK(a))] if text_of(&t.text) == "[cal] calibration started: 2 gyro" && a.result == MavResult::MAV_RESULT_ACCEPTED));
        command(&mut sim, MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION, [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let next = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS, result: MavResult::MAV_RESULT_TEMPORARILY_REJECTED, ..Default::default() });
        let asked: Vec<u32> = (0..7)
            .flat_map(|n| {
                let sent = sim.tick(n * 100);
                sim.receive(gcs(), &next);
                sent.into_iter().filter_map(|(_, m)| match m {
                    MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS => Some(c.param1 as u32),
                    _ => None,
                })
            })
            .collect();
        assert_eq!(asked, [1, 2, 3, 4, 5, 6, AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_SUCCESS as u32]);
        assert_eq!(sim.params.iter().find(|p| p.name == "INS_ACCOFFS_Z").map(|p| p.value), Some(0.1));
    }

    #[test]
    fn the_link_answers_a_parameter_read_over_bytes() {
        let (sent, received) = mpsc::channel::<Vec<u8>>();
        let link = MockLink::open(options(Vehicle::Copter), move |bytes| {
            let _ = sent.send(bytes.to_vec());
        })
        .unwrap();
        let mut request = Vec::new();
        mavlink::write_v2_msg(&mut request, gcs(), &MavMessage::PARAM_REQUEST_READ(PARAM_REQUEST_READ_DATA { param_index: -1, target_system: 0, target_component: 1, param_id: crate::mavout::param_id("FRAME_CLASS") })).unwrap();
        assert!(link.write(&request));
        let mut parser = Registry::default();
        let id = parser.open(Owner::Core, "mock", "test");
        let found = std::iter::from_fn(|| received.recv_timeout(Duration::from_secs(2)).ok())
            .flat_map(|bytes| parser.bytes_in(id, &bytes))
            .find(|frame| matches!(&frame.message, MavMessage::PARAM_VALUE(v) if text_of(&v.param_id) == "FRAME_CLASS"));
        link.close();
        assert!(found.is_some());
    }
}
