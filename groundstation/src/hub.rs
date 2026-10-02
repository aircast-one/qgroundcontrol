use mavlink::{MavHeader, Message};
use mavlink::dialects::ardupilotmega::MavMessage;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use crate::batteryfacts::Batteries;
use crate::gpsfacts::GpsFacts;
use crate::compinfo::ComponentParameters;
use crate::compmeta::{self, MSG_COMPONENT_METADATA, TYPE_GENERAL, TYPE_PARAMETER, Uris};
use crate::connect::{self, Action, AutopilotVersion, Connect, Firmware, MSG_AUTOPILOT_VERSION, MSG_PROTOCOL_VERSION};
use crate::ftp::{self, Download};
use crate::guidedcmd::{self, CMD_DO_REPOSITION, Plan, VehicleState};
use crate::guidedexec::{Emit, Executor, Observed};
use crate::mavcmd::{Command, Commands, Failure, Out, RESULT_ACCEPTED};
use crate::mavout::{self, Outbound};
use crate::params::{self, INITIAL_REQUEST_TIMEOUT_MS, ParamValue, Params, WAITING_TIMEOUT_MS};
use crate::plantransfer::{self, PLAN_FENCE, PLAN_MISSION, PLAN_RALLY, Transfer};
use crate::remoteid::{self, GcsFix, RemoteId};
use crate::sensorcal::{self, Calibration};
use crate::ulogstream::Processor;
use crate::standardmodes::{self, AvailableMode, FlightMode, MSG_AVAILABLE_MODES, StandardModes};
use crate::transport::LinkId;
use crate::sensorfacts::{DistanceSensorFacts, EfiFacts, RpmFacts, SubInfoFacts, Escs, EstimatorStatusFacts, GeneratorFacts, HygrometerFacts, LocalPositionFacts, SetpointFacts, TemperatureFacts, WindFacts};
use crate::statustext::{Handler, StatusText};
use crate::sysstatus::SysStatusSensors;
use crate::vehiclefacts::VehicleFacts;

pub const TYPE_GCS: u8 = 6;
const TYPE_GENERIC: u8 = 0;
const AUTOPILOT_GENERIC: u8 = 0;
pub const TYPE_ONBOARD_CONTROLLER: u8 = 18;
pub const TYPE_GIMBAL: u8 = 26;
pub const TYPE_ADSB: u8 = 27;
pub const COMP_AUTOPILOT1: u8 = 1;
const CMD_CONTROL_HIGH_LATENCY: u16 = 2600;
const AIRFRAME_DISCONNECT_AFTER_MS: u64 = 1000;
pub const AUTOPILOT_INVALID: u8 = 8;
pub const ARMED_FLAG: u8 = 128;
const HIGH_LATENCY_DOWNLOAD: &str = "Download not supported on high latency links.";
const HIGH_LATENCY_UPLOAD: &str = "Upload not supported on high latency links.";

const MANUAL_CONTROL_SCALE: f32 = 1000.0;
const MAV_TYPE_AIRSHIP: u8 = 7;
enum LogFileJob {
    List(String),
    Download(String, std::path::PathBuf),
    Delete(String),
}

#[derive(Clone, Copy)]
enum LogFileKind {
    List,
    Download,
    Delete,
}

impl LogFileJob {
    fn kind(&self) -> LogFileKind {
        match self {
            LogFileJob::List(_) => LogFileKind::List,
            LogFileJob::Download(..) => LogFileKind::Download,
            LogFileJob::Delete(_) => LogFileKind::Delete,
        }
    }
}

pub const FILES_BUSY: &str = "Another file transfer with the vehicle is in progress.";
const AIRFRAME_PARAMS: [&str; 2] = ["SYS_AUTOSTART", "SYS_AUTOCONFIG"];
const AIRFRAME_REBOOT_DELAY_MS: u64 = 800;
const CMD_SET_MESSAGE_INTERVAL: u16 = 511;
const MSG_HOME_POSITION_ID: u32 = 242;
const MSG_EXTENDED_SYS_STATE_ID: u32 = 245;
const STREAM_INTERVAL_US: f64 = 1_000_000.0;
const STREAM_REINIT_MS: u64 = 10_000;
const APM_STREAMS: [(u8, &str, i64); 7] = [
    (1, "streamRateRawSensors", 2),
    (2, "streamRateExtendedStatus", 2),
    (3, "streamRateRCChannels", 2),
    (6, "streamRatePosition", 3),
    (10, "streamRateExtra1", 10),
    (11, "streamRateExtra2", 10),
    (12, "streamRateExtra3", 3),
];
const CMD_CONFIGURE_ACTUATOR: u16 = 311;
const ACTUATOR_ACTION_TIMEOUT_MS: u64 = 3000;
const MAV_STATE_ACTIVE: u8 = 4;
const SEVERITY_ERROR: u8 = 3;
const SEVERITY_NOTICE: u8 = 5;
const BATTERY_OK: u8 = 1;
const BATTERY_LOW: u8 = 2;
const BATTERY_CRITICAL: u8 = 3;
const BATTERY_EMERGENCY: u8 = 4;
const BATTERY_FAILED: u8 = 5;
const BATTERY_UNHEALTHY: u8 = 6;
const FENCE_BREACH_NONE: u8 = 0;
const FENCE_BREACH_MINALT: u8 = 1;
const FENCE_BREACH_MAXALT: u8 = 2;
const FENCE_BREACH_BOUNDARY: u8 = 3;
const FENCE_SPEECH_GAP_MS: u64 = 3000;
const COMMAND_ACK_ID: u32 = 77;
const COMMAND_LONG_ID: u32 = 76;
const ORBIT_TELEMETRY_TIMEOUT_MS: u64 = 3000;
const LINK_SILENT_MS: u64 = 3500;
const RC_OVERRIDE_CHANNEL_COUNT: u8 = 18;
const RC_OVERRIDE_PERIOD_MS: u64 = 200;
const RC_OVERRIDE_RELEASE_TICKS: u8 = 3;
const SENSOR_GPS: u32 = 0x20;
const SENSOR_MOTOR_OUTPUTS: u32 = 0x8000;
const SENSOR_PREARM_CHECK: u32 = 0x1000_0000;
const MAV_STATE_CRITICAL: u8 = 5;
const MAV_STATE_EMERGENCY: u8 = 6;
const LANDED_ON_GROUND: u8 = 1;
use crate::guidedcmd::VTOL_STATE_FW;
const LANDED_IN_AIR: u8 = 2;
const LANDED_TAKEOFF: u8 = 3;
const LANDED_LANDING: u8 = 4;
pub const CUSTOM_MODE_FLAG: u8 = 1;
const MAX_MESSAGES: usize = 200;
const FETCH_PARAMETER_PACK: u8 = 255;
const PREARM_REPEAT_MS: u64 = 10_000;
const PREARM_SHOWN_MS: u64 = 35_000;
const CHUNKED_TEXT_TIMEOUT_MS: u64 = 1000;
pub const RESULT_UNSUPPORTED: u8 = 3;
pub const MAX_ERRORS: usize = 10;
pub const PROTO_MAVLINK2: u32 = 200;
pub const ODID_SEND_MS: u64 = 1000;
pub const CMD_LOGGING_START: u16 = 2510;
pub const CMD_LOGGING_STOP: u16 = 2511;
pub const RESULT_DENIED: u8 = 2;

#[derive(Debug, Clone, Default)]

pub struct LogInputs {
    pub path: String,
    pub extension: String,
    pub auto_start: bool,
}

#[derive(Debug)]
struct LogSession {
    processor: Processor,
    file: std::fs::File,
    path: String,
}
pub const ODID_TIMEOUT_MS: u64 = 2500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub link: LinkId,
    pub replay: bool,
    pub v2: bool,
}

#[derive(Debug, Clone)]
struct RemoteInputs {
    settings: remoteid::Settings,
    fix: GcsFix,
    pushed_ms: u64,
}

#[derive(Debug)]
struct PlanSlot {
    transfer: Transfer,
    due: Option<u64>,
    progress: f64,
    error: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StatusBits {
    pub present: u32,
    pub enabled: u32,
    pub health: u32,
    pub ready_to_fly_available: bool,
    pub ready_to_fly: bool,
    pub all_healthy: bool,
}

impl StatusBits {
    fn after(self, present: u32, enabled: u32, health: u32) -> StatusBits {
        let prearm = enabled & SENSOR_PREARM_CHECK != 0;
        StatusBits {
            present,
            enabled,
            health,
            ready_to_fly_available: self.ready_to_fly_available || prearm,
            ready_to_fly: if prearm { health & SENSOR_PREARM_CHECK != 0 } else { self.ready_to_fly },
            all_healthy: enabled & health == enabled,
        }
    }

    pub fn unhealthy(self) -> u32 {
        self.enabled & !self.health
    }

    pub fn requires_gps_fix(self) -> bool {
        self.present & SENSOR_GPS != 0
    }
}

#[derive(Debug)]
pub struct Vehicle {
    pub id: u8,
    pub component: u8,
    pub link: LinkId,
    pub autopilot: u8,
    pub vehicle_type: u8,
    pub base_mode: u8,
    pub custom_mode: u32,
    pub system_status: u8,
    pub heartbeats: u64,
    pub messages: u64,
    pub last_heartbeat_us: u64,
    pub gps: GpsFacts,
    pub gps2: GpsFacts,
    streams_watched_ms: Option<(u64, u64)>,
    pub integrity_heard_ms: Option<u64>,
    pub batteries: Batteries,
    pub facts: VehicleFacts,
    pub wind: WindFacts,
    pub setpoint: SetpointFacts,
    pub hygrometer: HygrometerFacts,
    pub generator: GeneratorFacts,
    pub efi: EfiFacts,
    pub rpm: RpmFacts,
    pub sub_info: SubInfoFacts,
    pub terrain_blocks: (u16, u16),
    pub escs: Escs,
    pub rc_override: BTreeMap<u8, u16>,
    pub airframe_reboot: Option<Option<u64>>,
    stream: crate::streamconfig::StreamConfig,
    pub autotune: crate::autotune::Autotune,
    pub actuator_test: crate::actuatortest::ActuatorTest,
    actuator_action_pending: Option<u64>,
    pub motor_assignment: crate::motorassignment::MotorAssignment,
    autotune_due: Option<u64>,
    pub cameras: crate::cameraproto::Cameras,
    pub onboard_logs: crate::onboardlogs::OnboardLogs,
    pub shell: crate::shell::Shell,
    pending_notices: Vec<(&'static str, String)>,
    pub rc_values: Vec<u16>,
    pub servo_outputs: Vec<i32>,
    pub message_log: crate::messagelog::MessageLog,
    pub control: crate::operatorcontrol::ControlState,
    pub rccal: crate::rccal::RcCal,
    rccal_loaded: bool,
    pub camera_tracking_enabled: bool,
    pub camera_tracking_image: Option<crate::cameratrack::TrackingImage>,
    camera_sent: BTreeMap<(u8, u16), f64>,
    pub mission_current: i32,
    pub roi_enabled: bool,
    pub mode_ack: Option<(u8, u64)>,
    pub roi_coord: Option<(f64, f64, f64)>,
    pub comm_lost_enabled: bool,
    gimbal_rate_due: Option<u64>,
    pub link_states: Vec<(LinkId, u64, bool)>,
    pub primary_link: Option<LinkId>,
    pub link_kinds: LinkKinds,
    link_frames: Vec<(LinkId, Vec<u8>)>,
    pub auto_disconnect: bool,
    pub check_list_state: i64,
    pub mission_last_current: i32,
    mission_cached_last: i32,
    resume_upload: Option<i64>,
    pub resume_failed: Option<i64>,
    pub trigger_points: Vec<(f64, f64, f64)>,
    pub trigger_points_appended: bool,
    image_captured_seen: bool,
    rc_release_ticks: u8,
    rc_due: Option<u64>,
    pub temperature: TemperatureFacts,
    pub vibration: crate::vehiclefact::VibrationFacts,
    pub radio: crate::vehiclefact::RadioStatusFacts,
    pub aircast: crate::vehiclefact::AircastLinkFacts,
    pub obstacle: crate::vehiclefact::ObstacleFacts,
    pub rc_rssi: crate::vehiclefact::RcRssi,
    pub orbit_heard_ms: Option<u64>,
    pub orbit_circle: Option<(f32, i32, i32)>,
    pub prearm: Option<(String, u64)>,
    prearm_spoken: BTreeMap<String, u64>,
    announced: (Option<String>, bool, bool),
    speech_prefix: String,
    battery_announced: BTreeMap<u8, u8>,
    fence_quiet_ms: u64,
    pub distance: DistanceSensorFacts,
    pub local: LocalPositionFacts,
    pub local_setpoint: LocalPositionFacts,
    pub estimator: EstimatorStatusFacts,
    pub sensors: SysStatusSensors,
    pub status_text: Handler,
    pub recent: Vec<StatusText>,
    pub by_name: BTreeMap<String, u64>,
    pub capabilities: u64,
    version_notified: bool,
    pub capabilities_known: bool,
    pub home_altitude: Option<f64>,
    pub home: Option<(f64, f64, f64)>,
    pub reposition_supported: Option<bool>,
    pub errors: Vec<String>,
    pub connection_lost: bool,
    pub flying: bool,
    pub parameter_defaults: BTreeMap<String, ParamValue>,
    ardupilot_components: std::collections::BTreeSet<u8>,
    pub vtol_in_forward_flight: bool,
    armed_now: bool,
    pub status_bits: StatusBits,
    pub landing: bool,
    pub replay: bool,
    pub max_proto_version: Option<u32>,
    pub autopilot_version: Option<AutopilotVersion>,
    pub flight_modes: Vec<FlightMode>,
    pub connect_progress: f64,
    pub connected: bool,
    pub params_progress: f64,
    commands: Commands,
    guided: Executor,
    connect: Connect,
    modes: StandardModes,
    params: Params,
    initial_due: Option<u64>,
    waiting_due: Option<u64>,
    sensor_refresh_due: Option<u64>,
    terrain_request: Option<crate::terrainprotocol::Request>,
    terrain_due: Option<u64>,
    metadata_types: BTreeMap<u8, Uris>,
    parameter_metadata: Option<ComponentParameters>,
    pub parameter_download_skipped: bool,
    parameters_announce_due: bool,
    pub link_status: crate::linkcount::LinkStatus,
    pub actuators_metadata: Option<Value>,
    pub events: crate::libevents::Session,
    intended_custom_mode: u32,
    fetch: Option<Fetch>,
    ftp_due: Option<u64>,
    ftp_seq: u16,
    pub files: crate::filejobs::Files,
    download_to: Option<String>,
    camera_definition_from: Option<u8>,
    files_for_logs: bool,
    ftp_list_time_unsupported: bool,
    plans: [PlanSlot; 3],
    remote: RemoteId,
    odid_due: Option<u64>,
    odid_send_due: Option<u64>,
    log: Option<LogSession>,
    last_log: Option<String>,
    chunk_due: Option<u64>,
    log_denied: bool,
    log_error: Option<String>,
    was_armed: bool,
    flight_started_ms: Option<u64>,
    flight_seconds: f64,
    calibrate: Calibration,
}

#[derive(Debug)]
struct Fetch {
    kind: u8,
    uri: String,
    download: Download,
    started_ms: u64,
    progress: f64,
}

const SEVERITY_CRITICAL: u8 = 2;

const TYPE_SUBMARINE: u8 = 12;

fn sub_video_defaults() {
    let source = crate::settingsstore::raw_setting("settings.videoSettings.videoSource").and_then(|v| v.as_str().map(str::to_string));
    if source.as_deref() == Some(crate::settingsstore::VIDEO_DISABLED) {
        crate::settingsstore::written("Video/videoSource", crate::videostate::SOURCE_UDP_H264);
        crate::settingsstore::written("Video/lowLatencyMode", "true");
    }
}

pub fn transfer_failed_text(plan: &str, error: &str) -> String {
    transfer_failed(if plan == "fence" { plantransfer::PLAN_FENCE } else { plantransfer::PLAN_RALLY }, error)
}

fn transfer_failed(kind: u8, error: &str) -> String {
    let name = match kind {
        plantransfer::PLAN_FENCE => "GeoFence",
        plantransfer::PLAN_RALLY => "Rally Point",
        _ => "Mission",
    };
    format!("{name} transfer failed. Error: {error}")
}

pub fn is_prearm(text: &str, severity: u8) -> bool {
    text.starts_with("PreArm") || (text.get(..9).is_some_and(|head| head.eq_ignore_ascii_case("preflight")) && severity >= SEVERITY_CRITICAL)
}

impl Vehicle {
    fn new(id: u8, component: u8, autopilot: u8, vehicle_type: u8, link: LinkId, replay: bool) -> Vehicle {
        Vehicle {
            id,
            component,
            link,
            autopilot,
            vehicle_type,
            base_mode: 0,
            custom_mode: 0,
            system_status: 0,
            heartbeats: 0,
            messages: 0,
            last_heartbeat_us: 0,
            gps: GpsFacts::default(),
            gps2: GpsFacts::default(),
            streams_watched_ms: None,
            integrity_heard_ms: None,
            batteries: Batteries::default(),
            facts: VehicleFacts::for_vehicle(id, component),
            wind: WindFacts::default(),
            setpoint: SetpointFacts::default(),
            hygrometer: HygrometerFacts::default(),
            generator: GeneratorFacts::default(),
            efi: EfiFacts::default(),
            rpm: RpmFacts::default(),
            sub_info: SubInfoFacts::default(),
            terrain_blocks: (0, 0),
            escs: Escs::default(),
            rc_override: BTreeMap::new(),
            cameras: crate::cameraproto::Cameras::new(),
            onboard_logs: crate::onboardlogs::OnboardLogs::default(),
            shell: crate::shell::Shell::default(),
            pending_notices: Vec::new(),
            rc_values: Vec::new(),
            servo_outputs: Vec::new(),
            message_log: crate::messagelog::MessageLog::default(),
            control: crate::operatorcontrol::ControlState::default(),
            rccal: crate::rccal::RcCal::default(),
            rccal_loaded: false,
            camera_tracking_enabled: false,
            camera_tracking_image: None,
            camera_sent: BTreeMap::new(),
            mission_current: -1,
            roi_enabled: false,
            mode_ack: None,
            roi_coord: None,
            comm_lost_enabled: true,
            gimbal_rate_due: None,
            link_states: Vec::new(),
            primary_link: None,
            link_kinds: LinkKinds::default(),
            link_frames: Vec::new(),
            auto_disconnect: false,
            check_list_state: 0,
            mission_last_current: -1,
            mission_cached_last: -1,
            resume_upload: None,
            resume_failed: None,
            trigger_points: Vec::new(),
            trigger_points_appended: false,
            image_captured_seen: false,
            rc_release_ticks: 0,
            airframe_reboot: None,
            stream: crate::streamconfig::StreamConfig::default(),
            autotune: crate::autotune::Autotune::default(),
            actuator_test: crate::actuatortest::ActuatorTest::default(),
            actuator_action_pending: None,
            motor_assignment: crate::motorassignment::MotorAssignment::default(),
            autotune_due: None,
            rc_due: None,
            temperature: TemperatureFacts::default(),
            vibration: crate::vehiclefact::VibrationFacts::default(),
            radio: crate::vehiclefact::RadioStatusFacts::default(),
            aircast: crate::vehiclefact::AircastLinkFacts::default(),
            obstacle: crate::vehiclefact::ObstacleFacts::default(),
            rc_rssi: crate::vehiclefact::RcRssi::default(),
            orbit_heard_ms: None,
            orbit_circle: None,
            prearm: None,
            prearm_spoken: BTreeMap::new(),
            announced: (None, false, false),
            speech_prefix: String::new(),
            battery_announced: BTreeMap::new(),
            fence_quiet_ms: 0,
            distance: DistanceSensorFacts::default(),
            local: LocalPositionFacts::default(),
            local_setpoint: LocalPositionFacts::default(),
            estimator: EstimatorStatusFacts::default(),
            sensors: SysStatusSensors::default(),
            status_text: Handler::default(),
            recent: Vec::new(),
            by_name: BTreeMap::new(),
            capabilities: 0,
            version_notified: false,
            capabilities_known: false,
            home_altitude: None,
            home: None,
            reposition_supported: None,
            errors: Vec::new(),
            connection_lost: false,
            flying: false,
            parameter_defaults: BTreeMap::new(),
            ardupilot_components: std::collections::BTreeSet::new(),
            vtol_in_forward_flight: false,
            armed_now: false,
            status_bits: StatusBits { all_healthy: true, ..StatusBits::default() },
            landing: false,
            replay,
            max_proto_version: None,
            autopilot_version: None,
            flight_modes: Vec::new(),
            connect_progress: 0.0,
            connected: false,
            params_progress: 0.0,
            commands: Commands::for_firmware(autopilot == crate::modes::AUTOPILOT_PX4),
            guided: Executor::default(),
            connect: Connect::default(),
            modes: StandardModes::default(),
            params: Params::new(component, autopilot == crate::modes::AUTOPILOT_PX4),
            initial_due: None,
            waiting_due: None,
            sensor_refresh_due: None,
            terrain_request: None,
            terrain_due: None,
            metadata_types: BTreeMap::new(),
            parameter_metadata: None,
            parameter_download_skipped: false,
            parameters_announce_due: false,
            link_status: crate::linkcount::LinkStatus::default(),
            actuators_metadata: None,
            events: crate::libevents::Session::default(),
            intended_custom_mode: 0,
            fetch: None,
            ftp_due: None,
            ftp_seq: 0,
            files: crate::filejobs::Files::default(),
            download_to: None,
            camera_definition_from: None,
            files_for_logs: false,
            ftp_list_time_unsupported: false,
            plans: [PLAN_MISSION, PLAN_FENCE, PLAN_RALLY].map(|kind| PlanSlot { transfer: Transfer::new(autopilot == crate::modes::AUTOPILOT_ARDUPILOT, kind), due: None, progress: 0.0, error: None }),
            remote: RemoteId::default(),
            odid_due: None,
            odid_send_due: None,
            log: None,
            last_log: None,
            chunk_due: None,
            log_denied: false,
            log_error: None,
            was_armed: false,
            flight_started_ms: None,
            flight_seconds: 0.0,
            calibrate: Calibration::new(autopilot == crate::modes::AUTOPILOT_PX4),
        }
    }

    fn calibration_inputs(&self) -> sensorcal::Inputs {
        let number = |name: &str| self.params.value(self.component, name).map(ParamValue::as_f64);
        let compass_bit = |index: usize| {
            let suffix = if index == 0 { String::new() } else { (index + 1).to_string() };
            let present = number(&format!("COMPASS_DEV_ID{suffix}")).is_some_and(|id| id > 0.0);
            let used = number(&format!("COMPASS_USE{suffix}")).is_some_and(|use_it| use_it != 0.0);
            if present && used { 1u8 << index } else { 0 }
        };
        sensorcal::Inputs {
            mag_sides: number("CAL_MAG_SIDES").map(|sides| sides as u32),
            compass_mask: (0..3).map(compass_bit).sum(),
            compass_fitness: number(sensorcal::COMPASS_FITNESS_PARAM),
            compass_learn: number(sensorcal::COMPASS_LEARN_PARAM).is_some(),
            north: None,
        }
    }

    fn follow_calibration(&mut self, actions: Vec<sensorcal::Action>, now_ms: u64) -> Vec<Vec<u8>> {
        let target = (self.id, self.component);
        actions
            .into_iter()
            .flat_map(|action| match action {
                sensorcal::Action::Command { command: sensorcal::CMD_PREFLIGHT_CALIBRATION, params, show_error: false } => self.encode(&Outbound::CommandLong { target, command: sensorcal::CMD_PREFLIGHT_CALIBRATION, params }).into_iter().collect(),
                sensorcal::Action::Command { command, params, show_error } => {
                    let outs = self.commands.send(Command { component: self.component, command, command_int: false, frame: 0, params, show_error, tag: 0 }, now_ms);
                    self.handle(outs, now_ms)
                }
                sensorcal::Action::SetParam { name, value } => {
                    let Some(current) = self.params.value(self.component, name) else {
                        self.note(format!("Calibration wanted to write {name}, which this vehicle does not have"));
                        return Vec::new();
                    };
                    let Some(written) = ParamValue::from_f64(current.param_type(), value) else {
                        self.note(format!("Calibration could not write {value} to {name}"));
                        return Vec::new();
                    };
                    let actions = self.params.write(self.component, name, written);
                    self.follow_params(actions, now_ms)
                }
                sensorcal::Action::Ack => self.encode(&Outbound::AccelCalAck).into_iter().collect(),
                sensorcal::Action::Notice(text) => {
                    crate::noticeboard::post(crate::noticeboard::MESSAGE, "", text);
                    Vec::new()
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .chain(self.refresh_calibration_params(now_ms))
            .collect()
    }

    fn refresh_calibration_params(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        let Some(families) = self.calibrate.take_refresh() else { return Vec::new() };
        let component = self.component;
        let actions: Vec<params::Action> = self.parameters(component).into_iter().filter(|(name, _)| families.iter().any(|family| name.starts_with(family))).flat_map(|(name, _)| self.params.refresh_quietly(component, &name)).collect();
        self.follow_params(actions, now_ms)
    }

    pub fn calibrate_request(&mut self, request: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        let action = request.get("action").and_then(Value::as_str).ok_or_else(|| "A calibration request needs an action.".to_string())?;
        let actions = match action {
            "start" => {
                let id = request.get("type").and_then(Value::as_str).ok_or_else(|| "A calibration start needs a type.".to_string())?;
                let kind = sensorcal::Kind::parse(id).ok_or_else(|| format!("Unknown calibration type {id:?}"))?;
                let inputs = sensorcal::Inputs { north: sensorcal::north_request(request), ..self.calibration_inputs() };
                self.calibrate.start(kind, inputs, now_ms)?
            }
            "cancel" => self.calibrate.cancel()?,
            "next" => self.calibrate.next()?,
            other => return Err(format!("Unknown calibration action {other:?}")),
        };
        Ok(self.follow_calibration(actions, now_ms))
    }

    pub fn calibration_snapshot(&self) -> Value {
        self.calibrate.snapshot()
    }

    pub fn start_log(&mut self, inputs: &LogInputs, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        if self.autopilot != crate::modes::AUTOPILOT_PX4 {
            return Err("MAVLink log streaming is a PX4 feature.".to_string());
        }
        if self.log_denied {
            return Err("The vehicle denied log streaming earlier.".to_string());
        }
        if self.log.is_some() {
            return Err("A log is already being written.".to_string());
        }
        if inputs.path.is_empty() {
            return Err("A log path is required before starting a log.".to_string());
        }
        std::fs::create_dir_all(&inputs.path).map_err(|e| format!("Could not create the log directory {}: {e}", inputs.path))?;
        let path = format!("{}/{:03}-{}{}", inputs.path.trim_end_matches('/'), self.id, chrono::Local::now().format("%Y-%m-%d-%H-%M-%S-%3f"), inputs.extension);
        let file = std::fs::File::create(&path).map_err(|e| format!("Could not create the log file {path}: {e}"))?;
        self.log = Some(LogSession { processor: Processor::default(), file, path });
        self.log_error = None;
        let outs = self.commands.send(Command { component: self.component, command: CMD_LOGGING_START, command_int: false, frame: 0, params: [0.0; 7], show_error: false, tag: 0 }, now_ms);
        Ok(self.handle(outs, now_ms))
    }

    pub fn stop_log(&mut self, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        if self.autopilot != crate::modes::AUTOPILOT_PX4 {
            return Err("MAVLink log streaming is a PX4 feature.".to_string());
        }
        self.last_log = self.log.take().map(|session| session.path);
        let outs = self.commands.send(Command { component: self.component, command: CMD_LOGGING_STOP, command_int: false, frame: 0, params: [0.0; 7], show_error: false, tag: 0 }, now_ms);
        Ok(self.handle(outs, now_ms))
    }

    fn discard_log(&mut self, reason: String) {
        if let Some(session) = self.log.take() {
            std::fs::remove_file(&session.path).ok();
            self.log_error = Some(reason);
        }
    }

    fn log_data(&mut self, sequence: u16, first_message: u8, data: &[u8], now_ms: u64) -> Vec<Vec<u8>> {
        let Some(session) = self.log.as_mut() else { return Vec::new() };
        let ok = session.processor.process(sequence, first_message, data);
        let output = session.processor.take_output();
        if ok && session.file.write_all(&output).is_ok() {
            return Vec::new();
        }
        let path = session.path.clone();
        self.log_error = Some(format!("Error writing MAVLink log file: {path}"));
        self.stop_log(now_ms).unwrap_or_default()
    }

    pub fn log_snapshot(&self) -> Value {
        json!({
            "running": self.log.is_some(),
            "file": self.log.as_ref().map(|l| l.path.clone()).or_else(|| self.last_log.clone()),
            "bytes": self.log.as_ref().map(|l| l.processor.written()).unwrap_or(0),
            "drops": self.log.as_ref().map(|l| l.processor.drops()).unwrap_or(0),
            "denied": self.log_denied,
            "error": self.log_error,
        })
    }

    fn follow_remote(&mut self, outs: Vec<remoteid::Out>, now_ms: u64) {
        outs.into_iter().for_each(|out| match out {
            remoteid::Out::StartSendTimer => self.odid_send_due = Some(now_ms + ODID_SEND_MS),
            remoteid::Out::StopSendTimer => self.odid_send_due = None,
            remoteid::Out::StartOdidTimeout => self.odid_due = Some(now_ms + ODID_TIMEOUT_MS),
            _ => {}
        });
    }

    fn send_remote_id(&mut self, inputs: Option<&RemoteInputs>, now_ms: u64, now_s: u64) -> Vec<Vec<u8>> {
        let unknown = GcsFix { valid: false, latitude: f64::NAN, longitude: f64::NAN, altitude: f64::NAN, age_ms: u64::MAX };
        let (settings, fix) = inputs.map(|i| (i.settings.clone(), GcsFix { age_ms: i.fix.age_ms.saturating_add(now_ms.saturating_sub(i.pushed_ms)), ..i.fix })).unwrap_or((remoteid::Settings::default(), unknown));
        let (messages, outs) = self.remote.messages(&settings, fix, now_s);
        self.follow_remote(outs, now_ms);
        let target = (self.id, self.component);
        messages.into_iter().filter_map(|message| self.encode(&Outbound::Odid { target, message })).collect()
    }

    pub fn remote_snapshot(&self) -> Value {
        json!({
            "available": self.remote.available,
            "commsGood": self.remote.comms_good,
            "armStatusGood": self.remote.arm_status_good,
            "armStatusError": self.remote.arm_status_error,
            "basicIdGood": self.remote.basic_id_good,
            "gcsGpsGood": self.remote.gcs_gps_good,
            "emergency": self.remote.emergency,
            "sending": self.odid_send_due.is_some(),
        })
    }

    fn plan_step(kind: u8) -> connect::Step {
        match kind {
            PLAN_FENCE => connect::Step::GeoFence,
            PLAN_RALLY => connect::Step::RallyPoints,
            _ => connect::Step::Mission,
        }
    }

    fn follow_plan(&mut self, kind: u8, outs: Vec<plantransfer::Out>, now_ms: u64) -> Vec<Vec<u8>> {
        let target = (self.id, self.component);
        let plan = kind as usize;
        outs.into_iter()
            .flat_map(|out| match out {
                plantransfer::Out::RequestList => self.encode(&Outbound::MissionRequestList { target, plan: kind }).into_iter().collect(),
                plantransfer::Out::RequestItem(seq) => self.encode(&Outbound::MissionRequestInt { target, plan: kind, seq }).into_iter().collect(),
                plantransfer::Out::SendCount(count) => self.encode(&Outbound::MissionCount { target, plan: kind, count }).into_iter().collect(),
                plantransfer::Out::SendItem(item) => self.encode(&Outbound::MissionItemInt { target, plan: kind, item }).into_iter().collect(),
                plantransfer::Out::SendAck => self.encode(&Outbound::MissionAck { target, plan: kind, result: plantransfer::RESULT_ACCEPTED }).into_iter().collect(),
                plantransfer::Out::ClearAll => self.encode(&Outbound::MissionClearAll { target, plan: kind }).into_iter().collect(),
                plantransfer::Out::StartTimer(ms) => {
                    self.plans[plan].due = Some(now_ms + ms);
                    Vec::new()
                }
                plantransfer::Out::StopTimer => {
                    self.plans[plan].due = None;
                    Vec::new()
                }
                plantransfer::Out::Progress(progress) => {
                    self.plans[plan].progress = progress;
                    Vec::new()
                }
                plantransfer::Out::HomePosition(latitude, longitude, altitude) => {
                    self.home_altitude = Some(altitude);
                    self.home = Some((latitude, longitude, altitude));
                    Vec::new()
                }
                plantransfer::Out::Done { success, error } => {
                    if kind == plantransfer::PLAN_MISSION && self.plans[plan].transfer.wrote {
                        self.resume_failed = self.resume_upload.take().filter(|_| !success);
                    }
                    if kind == plantransfer::PLAN_MISSION {
                        self.clear_trigger_points();
                        crate::track::clear(i64::from(self.id));
                        if self.plans[plan].transfer.wrote {
                            (self.mission_current, self.mission_last_current) = (-1, -1);
                        }
                    }
                    self.plans[plan].due = None;
                    self.plans[plan].error = (!success).then_some(error.clone());
                    if !success {
                        if !error.is_empty() {
                            crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &transfer_failed(kind, &error));
                        }
                        self.note(error);
                    }
                    self.step_done(Self::plan_step(kind), now_ms)
                }
            })
            .collect()
    }

    fn load_plan(&mut self, kind: u8, now_ms: u64) -> Vec<Vec<u8>> {
        let outs = self.plans[kind as usize].transfer.load();
        self.follow_plan(kind, outs, now_ms)
    }

    fn items_from(items: &[Value]) -> Option<Vec<plantransfer::Item>> {
        items
            .iter()
            .map(|item| {
                let params: Vec<f64> = item.get("params").and_then(Value::as_array)?.iter().map(|v| v.as_f64().filter(|f| f.is_finite())).collect::<Option<Vec<_>>>()?;
                let frame = u8::try_from(item.get("frame").and_then(Value::as_u64)?).ok().filter(|f| mavout::frame_known(*f))?;
                let command = u16::try_from(item.get("command").and_then(Value::as_u64)?).ok().filter(|c| mavout::command_known(*c))?;
                Some(plantransfer::Item { seq: 0, frame, command, current: false, auto_continue: item.get("autoContinue").and_then(Value::as_bool).unwrap_or(true), params: params.get(..7).and_then(|p| p.try_into().ok())? })
            })
            .collect()
    }

    fn fence_from(request: &Value) -> Option<plantransfer::Fence> {
        let pair = |v: &Value| Some((v.get(0)?.as_f64().filter(|f| f.is_finite())?, v.get(1)?.as_f64().filter(|f| f.is_finite())?));
        let polygons = request
            .get("polygons")
            .and_then(Value::as_array)
            .map(|list| list.iter().map(|p| Some(plantransfer::Polygon { inclusion: p.get("inclusion").and_then(Value::as_bool).unwrap_or(true), vertices: p.get("vertices")?.as_array()?.iter().map(pair).collect::<Option<Vec<_>>>().filter(|v| v.len() >= 3)? })).collect::<Option<Vec<_>>>())
            .unwrap_or(Some(Vec::new()))?;
        let circles = request
            .get("circles")
            .and_then(Value::as_array)
            .map(|list| list.iter().map(|c| Some(plantransfer::Circle { inclusion: c.get("inclusion").and_then(Value::as_bool).unwrap_or(true), center: pair(c.get("center")?)?, radius: c.get("radius")?.as_f64().filter(|f| f.is_finite())? })).collect::<Option<Vec<_>>>())
            .unwrap_or(Some(Vec::new()))?;
        let breach_return = match request.get("breachReturn") {
            None | Some(Value::Null) => None,
            Some(v) => Some((pair(v)?.0, pair(v)?.1, v.get(2)?.as_f64()?)),
        };
        Some(plantransfer::Fence { polygons, circles, breach_return })
    }

    pub fn write_mission(&mut self, items: Vec<plantransfer::Item>, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        let plan = &mut self.plans[PLAN_MISSION as usize];
        if plan.transfer.in_progress() {
            return Err("A plan transfer is still in progress.".to_string());
        }
        let outs = plan.transfer.write(items);
        Ok(self.follow_plan(PLAN_MISSION, outs, now_ms))
    }

    pub fn is_ardusub(&self) -> bool {
        self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT && crate::modes::vehicle_class(self.vehicle_type) == crate::modes::VehicleClass::Sub
    }

    pub fn sends_home(&self) -> bool {
        self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT
    }

    pub fn mission_request(&mut self, request: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        let kind = match request.get("plan").and_then(Value::as_str).unwrap_or("mission") {
            "mission" => PLAN_MISSION,
            "fence" => PLAN_FENCE,
            "rally" => PLAN_RALLY,
            other => return Err(format!("Unknown plan {other:?}")),
        };
        if self.plans[kind as usize].transfer.in_progress() {
            return Err("A plan transfer is still in progress.".to_string());
        }
        match (self.commands.high_latency, request.get("action").and_then(Value::as_str).unwrap_or("")) {
            (true, "load") => return Err(HIGH_LATENCY_DOWNLOAD.to_string()),
            (true, "write") => return Err(HIGH_LATENCY_UPLOAD.to_string()),
            _ => {}
        }
        match request.get("action").and_then(Value::as_str).unwrap_or("") {
            "load" => Ok(self.load_plan(kind, now_ms)),
            "removeAll" => {
                if kind == PLAN_MISSION {
                    (self.mission_current, self.mission_last_current) = (-1, -1);
                }
                let outs = self.plans[kind as usize].transfer.remove_all();
                Ok(self.follow_plan(kind, outs, now_ms))
            }
            "write" => {
                let items = match kind {
                    PLAN_FENCE => plantransfer::fence_items(&Self::fence_from(request).ok_or_else(|| "A fence write needs polygons with [lat, lon] vertices, circles with a center and radius, and an optional breachReturn.".to_string())?),
                    PLAN_RALLY => plantransfer::rally_items(&request.get("points").and_then(Value::as_array).ok_or_else(|| "A rally write needs a points array of [lat, lon, alt].".to_string())?.iter().map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?, p.get(2)?.as_f64()?))).collect::<Option<Vec<_>>>().ok_or_else(|| "Every rally point needs lat, lon and alt.".to_string())?),
                    _ => {
                        let listed = request.get("items").and_then(Value::as_array).filter(|items| !items.is_empty()).ok_or_else(|| "A write needs a non-empty items array.".to_string())?;
                        Self::items_from(listed).ok_or_else(|| "Every item needs a known frame and command and seven finite params.".to_string())?
                    }
                };
                let outs = self.plans[kind as usize].transfer.write(items);
                Ok(self.follow_plan(kind, outs, now_ms))
            }
            other => Err(format!("Unknown mission action {other:?}")),
        }
    }

    fn plan_state(&self, kind: u8) -> Value {
        let plan = &self.plans[kind as usize];
        json!({
            "inProgress": plan.transfer.in_progress(),
            "transaction": plan.transfer.transaction().map(|t| format!("{t:?}")),
            "progress": plan.progress,
            "error": plan.error,
            "count": plan.transfer.items.len(),
        })
    }

    pub fn mission_snapshot(&self) -> Value {
        let mut mission = self.plan_state(PLAN_MISSION);
        mission["items"] = self.plans[PLAN_MISSION as usize].transfer.items.iter().map(|i| json!({ "seq": i.seq, "frame": i.frame, "command": i.command, "current": i.current, "autoContinue": i.auto_continue, "params": i.params })).collect();
        let mut fence = self.plan_state(PLAN_FENCE);
        match plantransfer::fence_from_items(&self.plans[PLAN_FENCE as usize].transfer.items) {
            Ok(parsed) => {
                fence["polygons"] = parsed.polygons.iter().map(|p| json!({ "inclusion": p.inclusion, "vertices": p.vertices.iter().map(|(lat, lon)| json!([lat, lon])).collect::<Vec<_>>() })).collect();
                fence["circles"] = parsed.circles.iter().map(|c| json!({ "inclusion": c.inclusion, "center": [c.center.0, c.center.1], "radius": c.radius })).collect();
                fence["breachReturn"] = parsed.breach_return.map(|(lat, lon, alt)| json!([lat, lon, alt])).unwrap_or(Value::Null);
            }
            Err(reason) => fence["parseError"] = json!(reason),
        }
        let mut rally = self.plan_state(PLAN_RALLY);
        rally["points"] = plantransfer::rally_from_items(&self.plans[PLAN_RALLY as usize].transfer.items).iter().map(|(lat, lon, alt)| json!([lat, lon, alt])).collect();
        json!({ "mission": mission, "fence": fence, "rally": rally })
    }

    fn stream_parameters(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        if self.params.px4 {
            let cache = crate::paramcache::load(self.id, self.params.default_component);
            let volatile = cache
                .iter()
                .filter(|(name, value)| crate::factmeta::ValueType::from_param_type(value.param_type()).is_some_and(|kind| self.parameter_definition(name, kind).volatile_value))
                .map(|(name, _)| name.clone())
                .collect();
            self.params.use_cache(cache, volatile);
        }
        let actions = self.params.start();
        self.follow_params(actions, now_ms)
    }

    fn pack_received(&mut self, bytes: &[u8], now_ms: u64) -> Vec<Vec<u8>> {
        match params::parse_pack(bytes) {
            Ok(entries) => {
                self.parameter_defaults = entries.iter().filter_map(|e| Some((e.name.clone(), e.default?))).collect();
                let actions = self.params.load_pack(COMP_AUTOPILOT1, &entries);
                self.follow_params(actions, now_ms)
            }
            Err(reason) => {
                self.note(format!("The parameter file could not be read, so the parameters are streamed instead: {reason}"));
                self.stream_parameters(now_ms)
            }
        }
    }

    fn after_parameter_metadata(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        match self.metadata_types.get(&compmeta::TYPE_EVENTS).map(|u| u.uri.clone()) {
            Some(uri) => self.start_fetch(compmeta::TYPE_EVENTS, &uri, now_ms),
            None => self.after_event_metadata(now_ms),
        }
    }

    fn after_event_metadata(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        match self.metadata_types.get(&compmeta::TYPE_ACTUATORS).map(|u| u.uri.clone()) {
            Some(uri) => self.start_fetch(compmeta::TYPE_ACTUATORS, &uri, now_ms),
            None => self.step_done(connect::Step::ComponentInformation, now_ms),
        }
    }

    fn fetch_failed(&mut self, kind: u8, reason: String, now_ms: u64) -> Vec<Vec<u8>> {
        match kind {
            FETCH_PARAMETER_PACK => self.stream_parameters(now_ms),
            TYPE_PARAMETER => {
                self.note(reason);
                self.after_parameter_metadata(now_ms)
            }
            compmeta::TYPE_EVENTS => {
                self.note(reason);
                self.after_event_metadata(now_ms)
            }
            _ => {
                self.note(reason);
                self.step_done(connect::Step::ComponentInformation, now_ms)
            }
        }
    }

    fn start_fetch(&mut self, kind: u8, uri: &str, now_ms: u64) -> Vec<Vec<u8>> {
        match Download::start_from(self.component, uri, true, self.ftp_seq) {
            Ok((download, outs)) => {
                self.fetch = Some(Fetch { kind, uri: uri.to_string(), download, started_ms: now_ms, progress: 0.0 });
                self.follow_ftp(outs, now_ms)
            }
            Err(reason) => {
                self.note(format!("Component metadata at {uri} is not fetched: {reason}"));
                self.step_done(connect::Step::ComponentInformation, now_ms)
            }
        }
    }

    fn file_job(&mut self, action: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        let op = action.get("op").and_then(Value::as_str).unwrap_or("");
        if op == "cancel" {
            let steps = self.files.job.as_mut().map(crate::filejobs::Job::cancel).unwrap_or_default();
            return Ok(self.follow_files(steps, now_ms));
        }
        if self.files.busy() || self.fetch.is_some() {
            return Err(FILES_BUSY.to_string());
        }
        let path = action.get("path").and_then(Value::as_str).filter(|p| !p.is_empty()).ok_or("A file operation needs the path on the vehicle.")?;
        let local = action.get("file").and_then(Value::as_str).unwrap_or("").to_string();
        let camera = action.get("cameraDefinition").and_then(Value::as_u64).and_then(|c| u8::try_from(c).ok());
        let (component, seq) = (camera.unwrap_or(self.component), self.ftp_seq);
        let started = match op {
            "list" => crate::filejobs::Job::list(component, path, seq),
            "download" if !local.is_empty() || camera.is_some() => crate::filejobs::Job::download(component, path, seq),
            "upload" => std::fs::read(&local).map_err(|e| format!("Upload failed for: {path} - {e}")).and_then(|data| crate::filejobs::Job::upload(component, path, data, seq)),
            "delete" => crate::filejobs::Job::delete(component, path, seq),
            _ => Err(format!("{op} is not a file operation")),
        }?;
        let (job, steps) = started;
        self.download_to = (op == "download" && camera.is_none()).then_some(local);
        self.camera_definition_from = camera;
        self.files.job = Some(job);
        self.files.progress = 0.0;
        self.files.generation += 1;
        Ok(self.follow_files(steps, now_ms))
    }

    fn follow_files(&mut self, steps: Vec<crate::filejobs::Step>, now_ms: u64) -> Vec<Vec<u8>> {
        use crate::filejobs::{Outcome, Step};
        steps
            .into_iter()
            .flat_map(|step| match step {
                Step::Send(request) => {
                    let Some(job) = self.files.job.as_ref() else { return Vec::new() };
                    self.ftp_seq = job.expected_seq();
                    let target = (self.id, job.component());
                    self.encode(&Outbound::Ftp { target, payload: request.encode() }).into_iter().collect()
                }
                Step::StartTimer => {
                    self.files.due_ms = Some(now_ms + compmeta::FTP_ACK_TIMEOUT_MS);
                    Vec::new()
                }
                Step::StopTimer => {
                    self.files.due_ms = None;
                    Vec::new()
                }
                Step::Progress(progress) => {
                    self.files.progress = progress;
                    if self.files_for_logs {
                        self.onboard_logs.on_ftp_progress(progress, now_ms);
                    }
                    Vec::new()
                }
                Step::Done(result) => {
                    self.files.due_ms = None;
                    let Some(job) = self.files.job.take() else { return Vec::new() };
                    self.ftp_seq = job.expected_seq();
                    self.ftp_list_time_unsupported = self.ftp_list_time_unsupported || job.list_time_unsupported();
                    let for_logs = std::mem::take(&mut self.files_for_logs);
                    let result = match (result, self.download_to.take()) {
                        (Ok(Outcome::Downloaded(bytes)), Some(to)) => std::fs::write(&to, &bytes).map(|_| Outcome::Downloaded(Vec::new())).map_err(|e| format!("Download failed for: {} - {e}", job.path)),
                        (other, _) => other,
                    };
                    if for_logs {
                        let kind = match &result {
                            Ok(crate::filejobs::Outcome::Listed(_)) => LogFileKind::List,
                            Ok(crate::filejobs::Outcome::Deleted) => LogFileKind::Delete,
                            Ok(_) => LogFileKind::Download,
                            Err(_) if job.is_list() => LogFileKind::List,
                            Err(_) if job.is_delete() => LogFileKind::Delete,
                            Err(_) => LogFileKind::Download,
                        };
                        return self.log_file_job_done(&kind, result, now_ms);
                    }
                    match self.camera_definition_from.take() {
                        Some(compid) => crate::camsettings::ftp_finished(self.id, compid, result),
                        None => {
                            self.files.last = Some((job.path, result));
                            self.files.generation += 1;
                        }
                    }
                    Vec::new()
                }
            })
            .collect()
    }

    fn follow_ftp(&mut self, outs: Vec<ftp::Out>, now_ms: u64) -> Vec<Vec<u8>> {
        outs.into_iter()
            .flat_map(|out| match out {
                ftp::Out::Send(request) => {
                    let Some(fetch) = self.fetch.as_ref() else { return Vec::new() };
                    self.ftp_seq = fetch.download.expected_seq();
                    self.encode(&Outbound::Ftp { target: (self.id, fetch.download.component), payload: request.encode() }).into_iter().collect()
                }
                ftp::Out::StartTimer => {
                    self.ftp_due = Some(now_ms + compmeta::FTP_ACK_TIMEOUT_MS);
                    Vec::new()
                }
                ftp::Out::StopTimer => {
                    self.ftp_due = None;
                    Vec::new()
                }
                ftp::Out::Progress(progress) => {
                    if let Some(fetch) = self.fetch.as_mut() {
                        fetch.progress = progress;
                    }
                    Vec::new()
                }
                ftp::Out::Complete { ok, error, bytes } => {
                    self.ftp_due = None;
                    let Some(fetch) = self.fetch.take() else { return Vec::new() };
                    self.ftp_seq = fetch.download.expected_seq();
                    match (ok, fetch.kind) {
                        (false, kind) => self.fetch_failed(kind, format!("Component metadata download failed: {error}"), now_ms),
                        (true, FETCH_PARAMETER_PACK) => self.pack_received(&bytes, now_ms),
                        (true, kind) => self.metadata_received(kind, &fetch.uri, &bytes, now_ms),
                    }
                }
            })
            .collect()
    }

    fn metadata_received(&mut self, kind: u8, uri: &str, bytes: &[u8], now_ms: u64) -> Vec<Vec<u8>> {
        let text = compmeta::inflate(uri, bytes).map(|b| String::from_utf8_lossy(&b).to_string());
        match (kind, text) {
            (_, Err(reason)) => {
                self.note(format!("Component metadata could not be read: {reason}"));
                self.step_done(connect::Step::ComponentInformation, now_ms)
            }
            (TYPE_GENERAL, Ok(text)) => match compmeta::parse_general(&text) {
                Ok(types) => {
                    self.metadata_types = types;
                    match self.metadata_types.get(&TYPE_PARAMETER).map(|u| u.uri.clone()) {
                        Some(uri) => self.start_fetch(TYPE_PARAMETER, &uri, now_ms),
                        None => self.step_done(connect::Step::ComponentInformation, now_ms),
                    }
                }
                Err(reason) => {
                    self.note(reason);
                    self.step_done(connect::Step::ComponentInformation, now_ms)
                }
            },
            (compmeta::TYPE_EVENTS, Ok(text)) => {
                match crate::libevents::parse(&text) {
                    Ok(definitions) => {
                        let delivered = self.events.load(definitions, self.component);
                        delivered.into_iter().for_each(|d| self.event_delivered(d));
                    }
                    Err(reason) => self.note(reason),
                }
                self.after_event_metadata(now_ms)
            }
            (compmeta::TYPE_ACTUATORS, Ok(text)) => {
                match serde_json::from_str::<Value>(&text) {
                    Ok(parsed) => self.actuators_metadata = Some(parsed),
                    Err(reason) => self.note(format!("Actuator metadata could not be parsed: {reason}")),
                }
                self.step_done(connect::Step::ComponentInformation, now_ms)
            }
            (_, Ok(text)) => {
                match crate::compinfo::parse(&text) {
                    Ok(parsed) => self.parameter_metadata = Some(parsed),
                    Err(reason) => self.note(format!("Parameter metadata could not be parsed: {reason}")),
                }
                self.after_parameter_metadata(now_ms)
            }
        }
    }

    pub fn parameter_definition(&self, name: &str, value_type: crate::factmeta::ValueType) -> crate::factmeta::MetaData {
        match &self.parameter_metadata {
            Some(described) => described.metadata_for(name, value_type, self.component),
            None => crate::px4meta::bundled().get(name).cloned().unwrap_or_else(|| crate::factmeta::MetaData { name: name.to_string(), ..crate::px4meta::bare(value_type) }),
        }
    }

    pub fn parameter_meta(&self, name: &str, value: Option<ParamValue>) -> Option<Value> {
        let described = self.parameter_metadata.as_ref()?;
        let value_type = value.and_then(|v| crate::factmeta::ValueType::from_param_type(v.param_type())).unwrap_or(crate::factmeta::ValueType::Float);
        let meta = described.metadata_for(name, value_type, self.component);
        if !described.named.contains_key(name) && meta.short_description.is_empty() {
            return None;
        }
        Some(json!({
            "shortDescription": meta.short_description,
            "longDescription": meta.long_description,
            "units": meta.units,
            "min": meta.min,
            "max": meta.max,
            "decimalPlaces": meta.decimal_places,
            "increment": meta.increment,
            "readOnly": meta.read_only,
            "rebootRequired": meta.vehicle_reboot_required || meta.qgc_reboot_required,
            "vehicleRebootRequired": meta.vehicle_reboot_required,
            "applicationRestartRequired": meta.qgc_reboot_required,
            "restartNotices": crate::control::restart_notices(meta.vehicle_reboot_required, meta.qgc_reboot_required),
            "group": meta.group,
            "category": meta.category,
            "default": meta.default,
            "bitmask": meta.bitmask,
            "enums": meta.enums.iter().map(|e| json!({ "label": e.label, "value": e.value })).collect::<Vec<_>>(),
        }))
    }

    fn connect_link(&self) -> connect::Link {
        connect::Link { present: true, high_latency: self.commands.high_latency, log_replay: self.replay }
    }

    pub fn plans_supported(&self) -> (bool, bool) {
        let v = self.connect_vehicle();
        (v.fence_supported, v.rally_supported)
    }

    fn connect_vehicle(&self) -> connect::Vehicle {
        let px4 = self.autopilot == crate::modes::AUTOPILOT_PX4;
        let apm = self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT;
        let proto = self.max_proto_version.unwrap_or(0);
        connect::Vehicle { px4, apm, fence_supported: self.capabilities & connect::CAP_MISSION_FENCE != 0, rally_supported: self.capabilities & connect::CAP_MISSION_RALLY != 0, max_proto_version: proto }
    }

    pub fn pending_parameter_writes(&self) -> bool {
        self.params.pending_writes()
    }

    pub fn parameters_ready(&self) -> bool {
        self.params.ready()
    }

    pub fn parameters_unanswered(&self) -> bool {
        self.params.unanswered()
    }

    pub fn parameters_missing(&self) -> bool {
        self.params.missing()
    }

    pub fn firmware(&self) -> Option<Firmware> {
        self.autopilot_version.as_ref().map(|v| connect::firmware_from(v, self.autopilot == crate::modes::AUTOPILOT_PX4))
    }

    pub fn parameter(&self, component: u8, name: &str) -> Option<ParamValue> {
        self.params.value(component, &self.parameter_name(name))
    }

    fn vtol(&self) -> bool {
        (19..=25).contains(&self.vehicle_type)
    }

    fn raw_parameter(&self, name: &str) -> Option<f64> {
        self.params.value(self.component, name).map(|p| p.as_f64())
    }

    pub fn firmware_limit(&self, path: &str) -> Option<f64> {
        crate::vehiclefacade::firmware_limit(path, self.autopilot, self.vtol(), &|name| self.raw_parameter(name))
    }

    pub fn parameter_name(&self, name: &str) -> String {
        let family = (self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT).then(|| crate::apmmeta::vehicle_file_name(self.vehicle_type)).flatten();
        let version = self.firmware().and_then(|f| f.version).map(|(major, minor, _, _)| (major, minor));
        crate::paramremap::versioned(family, version, name)
    }

    pub fn parameters(&self, component: u8) -> Vec<(String, ParamValue)> {
        self.params.entries(component).map(|(name, value)| (name.clone(), *value)).collect()
    }

    pub fn parameter_request(&mut self, request: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        if !self.params.ready() {
            return Err("Parameters are still loading.".to_string());
        }
        let component = request.get("component").and_then(Value::as_u64).filter(|c| (1..=255).contains(c)).map(|c| c as u8).unwrap_or(self.component);
        let name = request
            .get("name")
            .and_then(Value::as_str)
            .filter(|n| !n.is_empty() && n.len() <= 16 && n.bytes().all(|b| b.is_ascii_graphic()))
            .ok_or_else(|| "A parameter name of at most sixteen printable characters is required.".to_string())?;
        let name = &self.parameter_name(name);
        if request.get("refresh").and_then(Value::as_bool).unwrap_or(false) {
            let actions = self.params.refresh(component, name);
            return Ok(self.follow_params(actions, now_ms));
        }
        let number = request.get("value").and_then(Value::as_f64).ok_or_else(|| format!("A numeric value is required to set {name}."))?;
        let known = self.params.value(component, name).ok_or_else(|| format!("{name} is not a parameter of component {component}."))?;
        let value = ParamValue::from_f64(known.param_type(), number).ok_or_else(|| format!("{number} is out of range for {name} (type {}).", known.param_type()))?;
        let actions = self.params.write(component, name, value);
        Ok(self.follow_params(actions, now_ms))
    }

    fn begin_connect(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        let actions = self.connect.start(&self.connect_link(), &self.connect_vehicle());
        let mut bytes = self.follow_connect(actions, now_ms);
        bytes.extend(self.initialize_stream_rates(now_ms));
        bytes
    }

    fn initialize_stream_rates(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        if !self.sends_home() {
            return Vec::new();
        }
        self.streams_watched_ms = Some((now_ms, now_ms));
        let target = (self.id, self.component);
        let requests: Vec<Outbound> = match crate::settingsstore::raw_setting("settings.mavlinkSettings.apmStartMavlinkStreams").and_then(|v| v.as_bool()).unwrap_or(true) {
            true => APM_STREAMS
                .iter()
                .filter_map(|(stream, name, default)| {
                    let rate = crate::settingsstore::raw_setting(&format!("settings.apmMavlinkStreamRateSettings.{name}")).and_then(|v| v.as_i64()).unwrap_or(*default);
                    (rate >= 0).then_some(Outbound::RequestDataStream { target, stream: *stream, rate: rate as u16 })
                })
                .collect(),
            false => Vec::new(),
        };
        let mut bytes: Vec<Vec<u8>> = requests.iter().filter_map(|send| self.encode(send)).collect();
        [MSG_HOME_POSITION_ID, MSG_EXTENDED_SYS_STATE_ID].iter().for_each(|message| {
            let params = [f64::from(*message), STREAM_INTERVAL_US, 0.0, 0.0, 0.0, 0.0, 0.0];
            let outs = self.commands.send(Command { component: COMP_AUTOPILOT1, command: CMD_SET_MESSAGE_INTERVAL, command_int: false, frame: 0, params, show_error: false, tag: 0 }, now_ms);
            bytes.extend(self.handle(outs, now_ms));
        });
        bytes
    }

    fn tick_stream_rates(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        match self.streams_watched_ms {
            Some((battery, home)) if now_ms.saturating_sub(battery) > STREAM_REINIT_MS || now_ms.saturating_sub(home) > STREAM_REINIT_MS => self.initialize_stream_rates(now_ms),
            _ => Vec::new(),
        }
    }

    fn step_done(&mut self, step: connect::Step, now_ms: u64) -> Vec<Vec<u8>> {
        if self.connect.current() != Some(step) {
            return Vec::new();
        }
        if step == connect::Step::Parameters && !self.parameter_download_skipped {
            self.parameters_announce_due = true;
        }
        let actions = self.connect.on_step_done(&self.connect_link(), &self.connect_vehicle());
        self.follow_connect(actions, now_ms)
    }

    fn follow_connect(&mut self, actions: Vec<Action>, now_ms: u64) -> Vec<Vec<u8>> {
        actions
            .into_iter()
            .flat_map(|action| match action {
                Action::RequestMessage { message_id } => {
                    let outs = self.commands.request_message(message_id as u64, self.component, message_id, [0.0; 5], now_ms);
                    self.handle(outs, now_ms)
                }
                Action::RequestStandardModes if self.replay => self.step_done(connect::Step::StandardModes, now_ms),
                Action::RequestStandardModes => {
                    let outs = self.modes.request();
                    self.follow_modes(outs, now_ms)
                }
                Action::RefreshParameters if self.replay => self.step_done(connect::Step::Parameters, now_ms),
                Action::RefreshParameters if self.skips_download_flying() => {
                    self.parameter_download_skipped = true;
                    self.step_done(connect::Step::Parameters, now_ms)
                }
                Action::RefreshParameters if self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT => match Download::start_from(COMP_AUTOPILOT1, params::PACK_URI, false, self.ftp_seq) {
                    Ok((download, outs)) => {
                        self.fetch = Some(Fetch { kind: FETCH_PARAMETER_PACK, uri: params::PACK_URI.to_string(), download, started_ms: now_ms, progress: 0.0 });
                        self.follow_ftp(outs, now_ms)
                    }
                    Err(_) => self.stream_parameters(now_ms),
                },
                Action::RefreshParameters => self.stream_parameters(now_ms),
                Action::RequestComponentInformation => {
                    let outs = self.commands.request_message(MSG_COMPONENT_METADATA as u64, self.component, MSG_COMPONENT_METADATA, [0.0; 5], now_ms);
                    self.handle(outs, now_ms)
                }
                Action::LoadMission if self.skips_download_flying() || (self.parameter_download_skipped && self.armed()) => self.step_done(connect::Step::Mission, now_ms),
                Action::LoadMission => self.load_plan(PLAN_MISSION, now_ms),
                Action::LoadGeoFence => self.load_plan(PLAN_FENCE, now_ms),
                Action::LoadRallyPoints => self.load_plan(PLAN_RALLY, now_ms),
                Action::FirstMissionLoadComplete => self.step_done(connect::Step::Mission, now_ms),
                Action::FirstGeoFenceLoadComplete => self.step_done(connect::Step::GeoFence, now_ms),
                Action::FirstRallyPointLoadComplete => self.step_done(connect::Step::RallyPoints, now_ms),
                Action::SetCapabilities(capabilities) => {
                    self.capabilities = capabilities;
                    self.capabilities_known = true;
                    Vec::new()
                }
                Action::SetMaxProtoVersion(version) => {
                    self.max_proto_version = Some(version);
                    Vec::new()
                }
                Action::Progress(progress) => {
                    self.connect_progress = progress;
                    Vec::new()
                }
                Action::InitialConnectComplete => {
                    log::info!("Initial connect complete for vehicle {}", self.id);
                    self.connected = true;
                    self.connect_progress = 1.0;
                    Vec::new()
                }
            })
            .collect()
    }

    fn follow_modes(&mut self, outs: Vec<standardmodes::Out>, now_ms: u64) -> Vec<Vec<u8>> {
        outs.into_iter()
            .flat_map(|out| match out {
                standardmodes::Out::RequestMode(index) => {
                    let outs = self.commands.request_message(MSG_AVAILABLE_MODES as u64, self.component, MSG_AVAILABLE_MODES, [index as f64, 0.0, 0.0, 0.0, 0.0], now_ms);
                    self.handle(outs, now_ms)
                }
                standardmodes::Out::Completed(modes) => {
                    self.flight_modes = modes;
                    self.step_done(connect::Step::StandardModes, now_ms)
                }
                standardmodes::Out::Failed => self.step_done(connect::Step::StandardModes, now_ms),
            })
            .collect()
    }

    fn follow_params(&mut self, actions: Vec<params::Action>, now_ms: u64) -> Vec<Vec<u8>> {
        let id = self.id;
        actions
            .into_iter()
            .flat_map(|action| match action {
                params::Action::RequestList { component } => self.encode(&Outbound::ParamRequestList { target: (id, component) }).into_iter().collect(),
                params::Action::ReadByIndex { component, index } => self.encode(&Outbound::ParamRequestRead { target: (id, component), name: None, index: index as i16 }).into_iter().collect(),
                params::Action::ReadByName { component, name } => self.encode(&Outbound::ParamRequestRead { target: (id, component), name: Some(name), index: -1 }).into_iter().collect(),
                params::Action::Set { component, name, value } => self.encode(&Outbound::ParamSet { target: (id, component), name, bits: if self.ardupilot_components.contains(&component) { value.encode_cast() } else { value.encode() }, param_type: value.param_type() }).into_iter().collect(),
                params::Action::StartInitialTimer => {
                    self.initial_due = Some(now_ms + INITIAL_REQUEST_TIMEOUT_MS);
                    Vec::new()
                }
                params::Action::StopInitialTimer => {
                    self.initial_due = None;
                    Vec::new()
                }
                params::Action::StartWaitingTimer => {
                    self.waiting_due = Some(now_ms + WAITING_TIMEOUT_MS);
                    Vec::new()
                }
                params::Action::StopWaitingTimer => {
                    self.waiting_due = None;
                    Vec::new()
                }
                params::Action::Progress(progress) => {
                    self.params_progress = progress;
                    self.connect_progress = self.connect.progress(progress);
                    Vec::new()
                }
                params::Action::Ready { missing } => {
                    self.params_progress = 1.0;
                    if missing {
                        self.note("Some parameters were not received from the vehicle.".to_string());
                        let app = Some(crate::noticeboard::application_name()).filter(|n| !n.is_empty()).unwrap_or_else(|| "QGroundControl".to_string());
                        crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &format!("{app} was unable to retrieve the full set of parameters from vehicle {}. This will cause {app} to be unable to display its full user interface. If you are using modified firmware, you may need to resolve any vehicle startup errors to resolve the issue. If you are using standard firmware, you may need to upgrade to a newer version to resolve the issue.", self.id));
                    }
                    let clock = self.send_clock();
                    clock.into_iter().chain(self.step_done(connect::Step::Parameters, now_ms)).collect()
                }
                params::Action::SaveCache { component } => {
                    let cache = self.params.entries(component).map(|(name, value)| (name.clone(), *value)).collect();
                    crate::paramcache::save(self.id, component, &cache);
                    Vec::new()
                }
                params::Action::NoResponse => {
                    self.note("The vehicle did not respond to the parameter request.".to_string());
                    let clock = self.send_clock();
                    clock.into_iter().chain(self.step_done(connect::Step::Parameters, now_ms)).collect()
                }
                params::Action::ReadFailed { component, name } => {
                    let text = format!("Parameter read failed: param: {name} {}", self.params.component_label(component)).trim_end().to_string();
                    crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &text);
                    self.note(text);
                    Vec::new()
                }
                params::Action::WriteFailed { component, name } => {
                    let text = format!("Parameter write failed: param: {name} {}", self.params.component_label(component)).trim_end().to_string();
                    crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &text);
                    self.note(text);
                    Vec::new()
                }
                params::Action::Added { .. } => Vec::new(),
            })
            .collect()
    }

    pub fn flight_mode(&self) -> String {
        let announced = (self.base_mode & crate::modes::FLAG_CUSTOM != 0).then(|| self.flight_modes.iter().find(|m| m.custom_mode == self.custom_mode)).flatten();
        announced.map_or_else(|| crate::modes::name(self.autopilot, self.vehicle_type, self.base_mode, self.custom_mode), |m| m.name.clone())
    }

    pub fn auto_stream(&self) -> Option<(u8, u8, String)> {
        self.cameras.selected().and_then(|camera| camera.current_stream()).filter(|stream| !stream.thermal()).map(|stream| (stream.kind, stream.encoding, stream.uri.clone()))
    }

    pub fn on_high_latency_link(&self) -> bool {
        self.commands.high_latency
    }

    pub fn parameter_components(&self) -> Vec<u8> {
        self.params.components()
    }

    pub fn announced_name(&self, canonical: &str) -> String {
        crate::modes::custom_mode_for(self.autopilot, self.vehicle_type, canonical)
            .and_then(|custom| self.flight_modes.iter().find(|m| m.custom_mode == custom))
            .map_or_else(|| canonical.to_string(), |m| m.name.clone())
    }

    fn custom_mode_named(&self, name: &str) -> Option<u32> {
        self.flight_modes.iter().find(|m| m.name == name).map(|m| m.custom_mode).or_else(|| crate::modes::custom_mode_for(self.autopilot, self.vehicle_type, name))
    }

    fn observed(&self) -> Observed {
        Observed { flight_mode: self.flight_mode(), armed: self.armed() }
    }

    fn known(&self, value: f64) -> Option<f64> {
        (self.facts.coordinate.is_some() && value.is_finite()).then_some(value)
    }

    pub fn planning_state(&self) -> VehicleState {
        VehicleState {
            autopilot: self.autopilot,
            vehicle_type: self.vehicle_type,
            base_mode: self.base_mode,
            flight_mode: self.flight_mode(),
            armed: self.armed(),
            altitude_amsl: self.known(self.facts.altitude_amsl),
            altitude_relative: self.known(self.facts.altitude_relative),
            home_altitude: self.home_altitude,
            capabilities: self.capabilities,
            reposition_supported: self.reposition_supported,
            minimum_takeoff_altitude: crate::vehiclefacade::minimum_takeoff_altitude(self.autopilot, self.vtol(), &|name| self.raw_parameter(name)),
            current_heading: Some(self.facts.heading),
            announced_modes: self.flight_modes.iter().map(|m| (m.name.clone(), m.custom_mode)).collect(),
        }
    }

    fn plan(&self, action: &Value) -> Plan {
        let state = self.planning_state();
        let number = |key: &str| action.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN);
        let flag = |key: &str| action.get(key).and_then(Value::as_bool).unwrap_or(false);
        match action.get("action").and_then(Value::as_str).unwrap_or("") {
            "takeoff" if guidedcmd::guided_takeoff_with_altitude(&state) => guidedcmd::takeoff(&state, number("altitude")),
            "takeoff" => guidedcmd::start_takeoff(&state, self.flying),
            "goto" => match guidedcmd::too_far_refusal(self.facts.coordinate.map(|(lat, lon, _)| (lat, lon)), (number("latitude"), number("longitude")), guidedcmd::max_goto_meters()) {
                Some(reason) => Plan::Refused(reason),
                None => guidedcmd::goto(&state, number("latitude"), number("longitude"), action.get("loiterRadius").and_then(Value::as_f64).unwrap_or(0.0)),
            },
            "changeAltitude" => guidedcmd::change_altitude(&state, number("delta"), flag("pause")),
            "pause" => guidedcmd::pause(&state),
            "rtl" => guidedcmd::rtl(&state, flag("smart")),
            "land" => guidedcmd::land(&state),
            "speed" => guidedcmd::change_speed(flag("ground"), number("metresPerSecond")),
            "arm" => Plan::Steps(vec![guidedcmd::arm(flag("arm"), flag("force"))]),
            "startMission" => guidedcmd::start_mission(&state, flag("flying")),
            "emergencyStop" => guidedcmd::emergency_stop(),
            "abortLanding" => guidedcmd::abort_landing(number("climbOut")),
            "gripper" => guidedcmd::gripper(number("gripAction")),
            "cancelRoi" => guidedcmd::cancel_roi(&state),
            "resetParameters" => guidedcmd::reset_parameters(),
            "setCurrentMission" => guidedcmd::set_current_mission(&state, number("sequence")),
            "orbit" => guidedcmd::orbit(&state, number("latitude"), number("longitude"), number("radius"), number("altitudeAmsl")),
            "setHome" => guidedcmd::set_home(number("latitude"), number("longitude"), action.get("terrain").and_then(Value::as_f64)),
            "roi" => guidedcmd::roi(&state, number("latitude"), number("longitude"), number("altitude"), action.get("frame").and_then(Value::as_u64).and_then(|f| u8::try_from(f).ok()).unwrap_or(guidedcmd::FRAME_GLOBAL)),
            "heading" => guidedcmd::change_heading(
                &state,
                self.facts.coordinate.map(|(lat, lon, _)| (lat, lon)),
                (number("latitude"), number("longitude")),
                self.parameter(self.component, "ATC_RATE_Y_MAX").map(|p| p.as_f64()),
            ),
            "estimatorOrigin" => guidedcmd::estimator_origin(number("latitude"), number("longitude"), action.get("altitude").and_then(Value::as_f64).unwrap_or(0.0)),
            "triggerCamera" => guidedcmd::trigger_camera(),
            "landingGear" => guidedcmd::landing_gear(flag("retract")),
            "motorInterlock" => guidedcmd::motor_interlock(&state, flag("enable")),
            "motorTest" => guidedcmd::motor_test(number("motor"), number("percent"), number("seconds")),
            "vtolTransition" => guidedcmd::vtol_transition(action.get("forward").and_then(Value::as_bool).unwrap_or(false)),
            "setMode" => guidedcmd::set_mode(&state, action.get("mode").and_then(Value::as_str).unwrap_or("")).map(Plan::Steps).unwrap_or_else(|| Plan::Refused("Unknown flight mode".to_string())),
            other => Plan::Refused(format!("Unknown guided action {other:?}")),
        }
    }

    fn send_rc_override(&mut self) -> Vec<Vec<u8>> {
        if self.commands.high_latency {
            return Vec::new();
        }
        let channels: [u16; 18] = std::array::from_fn(|i| self.rc_override.get(&(i as u8 + 1)).copied().unwrap_or(u16::MAX));
        let target = (self.id, self.component);
        self.encode(&Outbound::RcOverride { target, channels }).into_iter().collect()
    }

    fn change_autostart(&mut self, action: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        let autostart = action.get("autostartId").and_then(Value::as_f64).ok_or("Changing the airframe needs its autostart id.")?;
        let known = AIRFRAME_PARAMS.iter().map(|name| self.params.value(self.component, name).map(|v| (*name, v))).collect::<Option<Vec<_>>>().ok_or("This vehicle has no SYS_AUTOSTART and SYS_AUTOCONFIG to change.")?;
        let actions: Vec<_> = known
            .iter()
            .filter_map(|(name, held)| ParamValue::from_f64(held.param_type(), if *name == "SYS_AUTOSTART" { autostart } else { 1.0 }).map(|value| (*name, value)))
            .flat_map(|(name, value)| self.params.write(self.component, name, value))
            .collect();
        self.airframe_reboot = Some(None);
        Ok(self.follow_params(actions, now_ms))
    }

    fn param_integer(&self, name: &str) -> Option<i64> {
        self.parameter(self.component, name).map(ParamValue::as_f64).filter(|v| v.fract() == 0.0).map(|v| v as i64)
    }

    fn write_function_params(&mut self, writes: crate::motorassignment::Writes, now_ms: u64) -> Vec<Vec<u8>> {
        let typed: Vec<(String, ParamValue)> = writes
            .into_iter()
            .filter_map(|(name, value)| {
                let held = self.params.value(self.component, &name)?;
                Some((name, ParamValue::from_f64(held.param_type(), value as f64)?))
            })
            .collect();
        let component = self.component;
        let actions: Vec<_> = typed.into_iter().flat_map(|(name, value)| self.params.write(component, &name, value)).collect();
        self.follow_params(actions, now_ms)
    }

    fn motor_spin(&mut self, function: Option<i64>) -> Vec<Vec<u8>> {
        self.actuator_request(function.map(|function| crate::actuatortest::Request { function, value: crate::motorassignment::SPIN_VALUE, timeout: crate::motorassignment::SPIN_TIMEOUT }))
    }

    fn motor_assignment_action(&mut self, action: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        let metadata = self.actuators_metadata.as_ref().and_then(|m| crate::actuators::parse(m).ok()).ok_or("This vehicle sent no actuator metadata.")?;
        let mut assignment = std::mem::take(&mut self.motor_assignment);
        let integer = |name: &str| self.param_integer(name);
        let (answer, writes, spin) = match action.get("op").and_then(Value::as_str) {
            Some("init") => {
                let selected = action.get("output").and_then(Value::as_u64).unwrap_or(0) as usize;
                let exists = |name: &str| self.parameter(self.component, name).is_some();
                let outputs = crate::actuators::kept_outputs(&metadata, &exists);
                let groups: Vec<Vec<String>> = outputs.iter().map(|o| crate::actuators::function_params(o).into_iter().filter(|n| exists(n)).collect()).collect();
                let labels: Vec<String> = outputs.iter().map(|o| o.label.clone()).collect();
                let mixer = crate::actuators::mixer_state(&metadata, &integer);
                let count = crate::actuators::motor_geometry(&metadata, &mixer, &|name| self.parameter(self.component, name).map(ParamValue::as_f64)).len() as i64;
                match metadata.actuator_types.iter().find(|t| t.name == "motor").map(|t| t.function_min) {
                    None => (Err("Actuator type 'motor' not found".to_string()), Vec::new(), None),
                    Some(first) => match assignment.init(groups, &labels, selected, first, count, &integer) {
                        true => (Ok(()), Vec::new(), None),
                        false => (Err(assignment.message.clone()), Vec::new(), None),
                    },
                }
            }
            Some("start") => (Ok(()), assignment.start(now_ms, &integer), None),
            Some("select") => match action.get("motor").and_then(Value::as_i64) {
                Some(motor) => (Ok(()), assignment.select(motor, now_ms, &integer), None),
                None => (Err("Select which motor spun.".to_string()), Vec::new(), None),
            },
            Some("spinAgain") => (Ok(()), Vec::new(), assignment.spin_again(now_ms)),
            Some("abort") => {
                assignment.abort();
                (Ok(()), Vec::new(), None)
            }
            _ => (Err("Motor assignment is init, start, select, spinAgain or abort.".to_string()), Vec::new(), None),
        };
        self.motor_assignment = assignment;
        answer?;
        let written = self.write_function_params(writes, now_ms);
        Ok(written.into_iter().chain(self.motor_spin(spin)).collect())
    }

    fn actuator_request(&mut self, request: Option<crate::actuatortest::Request>) -> Vec<Vec<u8>> {
        let Some(request) = request else { return Vec::new() };
        let target = (self.id, COMP_AUTOPILOT1);
        let params = [f64::from(request.value), f64::from(request.timeout), 0.0, 0.0, (crate::actuatortest::FUNCTION_OFFSET + request.function) as f64, 0.0, 0.0];
        self.encode(&Outbound::CommandLong { target, command: crate::actuatortest::CMD_ACTUATOR_TEST, params }).into_iter().collect()
    }

    fn autotune_poll(&mut self) -> Vec<Vec<u8>> {
        let target = (self.id, COMP_AUTOPILOT1);
        self.encode(&Outbound::CommandLong { target, command: crate::autotune::CMD_DO_AUTOTUNE_ENABLE, params: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] }).into_iter().collect()
    }

    fn tick_autotune(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        match self.autotune_due {
            Some(due) if self.autotune.in_progress && now_ms >= due => {
                self.autotune_due = Some(now_ms + crate::autotune::POLL_MS);
                self.autotune_poll()
            }
            Some(_) if !self.autotune.in_progress => {
                self.autotune_due = None;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn send_interval(&mut self, interval: Option<crate::streamconfig::Interval>, now_ms: u64) -> Vec<Vec<u8>> {
        let Some((message, rate)) = interval else { return Vec::new() };
        let params = [f64::from(message), f64::from(rate), 0.0, 0.0, 0.0, 0.0, 0.0];
        let outs = self.commands.send(Command { component: self.component, command: CMD_SET_MESSAGE_INTERVAL, command_int: false, frame: 0, params, show_error: true, tag: 0 }, now_ms);
        self.handle(outs, now_ms)
    }

    fn tick_airframe_reboot(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        let acked = !AIRFRAME_PARAMS.iter().any(|name| self.params.writing(self.component, name));
        match self.airframe_reboot {
            Some(None) if acked => {
                self.airframe_reboot = Some(Some(now_ms + AIRFRAME_REBOOT_DELAY_MS));
                Vec::new()
            }
            Some(Some(due)) if now_ms >= due => {
                self.airframe_reboot = None;
                log::info!("Rebooting vehicle {} after the airframe change", self.id);
                crate::corelinks::close_links_at(now_ms + AIRFRAME_DISCONNECT_AFTER_MS, None, "the vehicle is rebooting after an airframe change");
                self.send_tagged(guidedcmd::CMD_PREFLIGHT_REBOOT_SHUTDOWN, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], true, 0, now_ms)
            }
            _ => Vec::new(),
        }
    }

    fn virtual_joystick(&mut self, action: &Value) -> Vec<Vec<u8>> {
        if self.commands.high_latency {
            return Vec::new();
        }
        let axis = |key: &str| (action.get(key).and_then(Value::as_f64).unwrap_or(0.0) as f32 * MANUAL_CONTROL_SCALE) as i16;
        let control = Outbound::ManualControl { target: self.id, x: axis("pitch"), y: axis("roll"), z: axis("thrust"), r: axis("yaw") };
        self.encode(&control).into_iter().collect()
    }

    pub fn set_rc_override(&mut self, channel: u8, pwm: i64, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        if !(1..=RC_OVERRIDE_CHANNEL_COUNT).contains(&channel) {
            return Err(format!("RC channels run from 1 to {RC_OVERRIDE_CHANNEL_COUNT}."));
        }
        self.rc_release_ticks = 0;
        self.rc_override.insert(channel, pwm.clamp(800, 2200) as u16);
        self.rc_due.get_or_insert(now_ms + RC_OVERRIDE_PERIOD_MS);
        Ok(self.send_rc_override())
    }

    pub fn clear_rc_overrides(&mut self) -> Vec<Vec<u8>> {
        if self.rc_override.is_empty() {
            return Vec::new();
        }
        self.rc_override.values_mut().for_each(|pwm| *pwm = 0);
        self.rc_release_ticks = RC_OVERRIDE_RELEASE_TICKS;
        self.send_rc_override()
    }

    fn tick_rc_override(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        if !self.rc_due.is_some_and(|due| now_ms >= due) {
            return Vec::new();
        }
        self.rc_due = Some(now_ms + RC_OVERRIDE_PERIOD_MS);
        if self.rc_release_ticks > 0 {
            self.rc_release_ticks -= 1;
            if self.rc_release_ticks == 0 {
                self.rc_due = None;
                let last = self.send_rc_override();
                self.rc_override.clear();
                return last;
            }
        }
        self.send_rc_override()
    }

    pub fn start_guided(&mut self, action: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        match action.get("action").and_then(Value::as_str) {
            Some("rcOverride") => {
                let whole = |key: &str| action.get(key).and_then(Value::as_i64);
                let (Some(channel), Some(pwm)) = (whole("channel").and_then(|c| u8::try_from(c).ok()), whole("pwm")) else {
                    return Err("An RC override takes a channel number and a PWM value.".to_string());
                };
                return self.set_rc_override(channel, pwm, now_ms);
            }
            Some("rcRelease") => return Ok(self.clear_rc_overrides()),
            Some("virtualJoystick") => return Ok(self.virtual_joystick(action)),
            Some("reboot") => return Ok(self.send_tagged(guidedcmd::CMD_PREFLIGHT_REBOOT_SHUTDOWN, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], false, REBOOT_TAG, now_ms)),
            Some("factoryReset") => return Ok(self.send_tagged(guidedcmd::CMD_PREFLIGHT_STORAGE, [STORAGE_RESET_FACTORY, STORAGE_MISSION_UNTOUCHED, 0.0, 0.0, 0.0, 0.0, 0.0], true, FACTORY_RESET_TAG, now_ms)),
            Some("changeAutostart") => return self.change_autostart(action, now_ms),
            Some("motorAssignment") => return self.motor_assignment_action(action, now_ms),
            Some("actuatorAction") => {
                let number = |key: &str| action.get(key).and_then(Value::as_f64);
                let (Some(kind), Some(function)) = (number("type"), number("function")) else { return Err("An actuator action takes its type and output function.".to_string()) };
                if self.actuator_action_pending.is_some_and(|until| now_ms < until) {
                    return Ok(Vec::new());
                }
                self.actuator_action_pending = Some(now_ms + ACTUATOR_ACTION_TIMEOUT_MS);
                let target = (self.id, COMP_AUTOPILOT1);
                let params = [kind, 0.0, 0.0, 0.0, (crate::actuatortest::FUNCTION_OFFSET as f64) + function, 0.0, 0.0];
                return Ok(self.encode(&Outbound::CommandLong { target, command: CMD_CONFIGURE_ACTUATOR, params }).into_iter().collect());
            }
            Some("actuatorTest") => {
                let number = |key: &str| action.get(key).and_then(Value::as_f64);
                let request = match action.get("op").and_then(Value::as_str) {
                    Some("active") => self.actuator_test.set_active(action.get("on").and_then(Value::as_bool).unwrap_or(false), now_ms),
                    Some("set") => {
                        let (Some(function), Some(value)) = (number("function"), number("value")) else { return Err("An actuator test sets a function to a value.".to_string()) };
                        self.actuator_test.set(function as i64, value as f32, now_ms)
                    }
                    Some("stop") => self.actuator_test.stop(number("function").map(|f| f as i64), now_ms),
                    _ => return Err("An actuator test is active, set or stop.".to_string()),
                };
                return Ok(self.actuator_request(request));
            }
            Some("autotune") => {
                self.autotune.request();
                self.autotune_due = Some(now_ms + crate::autotune::POLL_MS);
                return Ok(self.autotune_poll());
            }
            Some("pidTuningMode") => {
                let mode = action.get("mode").and_then(Value::as_i64).and_then(crate::streamconfig::Mode::from_index).ok_or("The PID tuning telemetry mode is 0 to 3.")?;
                let next = self.stream.set_mode(mode);
                return Ok(self.send_interval(next, now_ms));
            }
            Some("gimbal") => return self.gimbal_action(action, now_ms),
            Some("ftp") => return self.file_job(action, now_ms),
            Some("camera") => return self.camera_action(action, now_ms),
            Some("paramSetRaw") => {
                let component = action.get("component").and_then(Value::as_u64).and_then(|c| u8::try_from(c).ok()).unwrap_or(self.component);
                let name = action.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).ok_or("A parameter is set by name.")?.to_string();
                let param_type = action.get("type").and_then(Value::as_u64).and_then(|t| u8::try_from(t).ok()).ok_or("A parameter missing on the vehicle is sent with its MAVLink type.")?;
                let value = action.get("value").and_then(Value::as_f64).and_then(|v| ParamValue::from_f64(param_type, v)).ok_or("That value does not fit the parameter's type.")?;
                let bits = if self.ardupilot_components.contains(&component) { value.encode_cast() } else { value.encode() };
                return Ok(self.encode(&Outbound::ParamSet { target: (self.id, component), name, bits, param_type: value.param_type() }).into_iter().collect());
            }
            Some("mavlinkCommand") => {
                let command = action.get("command").and_then(Value::as_u64).and_then(|c| u16::try_from(c).ok()).ok_or("A MAVLink action needs its command id.")?;
                let component = action.get("component").and_then(Value::as_u64).and_then(|c| u8::try_from(c).ok()).unwrap_or(self.component);
                let given: Vec<f64> = action.get("params").and_then(Value::as_array).map(|p| p.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect()).unwrap_or_default();
                let params: [f64; 7] = std::array::from_fn(|i| given.get(i).copied().unwrap_or(0.0));
                let outs = self.commands.send(Command { component, command, command_int: false, frame: guidedcmd::FRAME_GLOBAL, params, show_error: true, tag: 0 }, now_ms);
                return Ok(self.handle(outs, now_ms));
            }
            Some("refreshParameters") => {
                if action.get("names").is_none() {
                    self.parameter_download_skipped = false;
                }
                let actions = match action.get("names").and_then(Value::as_array) {
                    Some(names) => names.iter().filter_map(Value::as_str).flat_map(|name| self.params.refresh(self.component, name)).collect(),
                    None => self.params.refresh_all(params::ALL_COMPONENTS),
                };
                return Ok(self.follow_params(actions, now_ms));
            }
            Some("calibrate") => return self.calibrate_request(&action["request"], now_ms),
            Some("rcCal") => {
                self.load_rccal();
                let vehicle = self.rccal_vehicle();
                let mut cal = std::mem::take(&mut self.rccal);
                let outcomes = {
                    let lookup = |name: &str| self.params.value(self.component, name).map(|p| p.as_f64());
                    match action.get("op").and_then(Value::as_str) {
                        Some("next") => cal.next(&vehicle, &lookup),
                        Some("cancel") => vec![cal.stop(&vehicle, &lookup)],
                        _ => Vec::new(),
                    }
                };
                self.rccal = cal;
                if outcomes.contains(&crate::rccal::Outcome::ThrottleReversed) {
                    self.note("Attempt to calibrate with a reversed throttle. Reverse the throttle on the transmitter and calibrate again.".to_string());
                }
                let target = (self.id, self.component);
                let wrote = outcomes.iter().any(|outcome| matches!(outcome, crate::rccal::Outcome::Write(_)));
                let sent: Vec<Vec<u8>> = outcomes
                    .into_iter()
                    .flat_map(|outcome| match outcome {
                        crate::rccal::Outcome::StartCalibration => self.encode(&Outbound::CommandLong { target, command: sensorcal::CMD_PREFLIGHT_CALIBRATION, params: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0] }).into_iter().collect(),
                        crate::rccal::Outcome::StopCalibration => self.encode(&Outbound::CommandLong { target, command: sensorcal::CMD_PREFLIGHT_CALIBRATION, params: [0.0; 7] }).into_iter().collect(),
                        crate::rccal::Outcome::Write(writes) => writes
                            .into_iter()
                            .flat_map(|(name, value)| {
                                let written = self.params.value(self.component, &name).and_then(|current| ParamValue::from_f64(current.param_type(), value));
                                written.map(|written| {
                                    let actions = self.params.write(self.component, &name, written);
                                    self.follow_params(actions, now_ms)
                                }).unwrap_or_default()
                            })
                            .collect(),
                        _ => Vec::new(),
                    })
                    .collect();
                if wrote {
                    let lookup = |name: &str| self.params.value(self.component, name).map(|p| p.as_f64());
                    let mut cal = std::mem::take(&mut self.rccal);
                    cal.read_stored(&vehicle, &lookup);
                    self.rccal = cal;
                }
                return Ok(sent);
            }
            Some("resumeMission") => {
                let index = action.get("index").and_then(Value::as_i64).ok_or("A resume names the mission index to resume from.")?;
                let on_vehicle = if self.sends_home() { index } else { index - 1 };
                let commands = crate::cmdinfo::tree(crate::plandoc::firmware(i64::from(self.autopilot)), crate::plandoc::vehicle_class(i64::from(self.vehicle_type)));
                let items = crate::resumemission::resume_items(self.mission_items(), usize::try_from(on_vehicle.max(0)).unwrap_or(0), self.sends_home(), |command| crate::resumemission::shape(&commands, command))
                    .inspect_err(|refusal| { crate::noticeboard::post(crate::noticeboard::MESSAGE, "", refusal); })?;
                self.resume_failed = None;
                let sent = self.write_mission(items, now_ms)?;
                self.resume_upload = Some(index);
                return Ok(sent);
            }
            Some("revertTakeoverTimer") => {
                self.control.start_revert(now_ms);
                return Ok(Vec::new());
            }
            Some("requestControl") => {
                let allow = action.get("allowTakeover").and_then(Value::as_bool).unwrap_or(false);
                let (Some(timeout), Some(safe)) = (action.get("timeout").and_then(Value::as_i64), action.get("safeTimeout").and_then(Value::as_i64)) else {
                    return Err("A control request names how long to wait.".to_string());
                };
                let params = [0.0, 1.0, if allow { 1.0 } else { 0.0 }, safe as f64, 0.0, 0.0, 0.0];
                let outs = self.commands.send(Command { component: self.component, command: crate::operatorcontrol::REQUEST_OPERATOR_CONTROL, command_int: false, frame: 0, params, show_error: false, tag: 0 }, now_ms);
                self.control.requested(timeout, now_ms);
                self.control.answered();
                return Ok(self.handle(outs, now_ms));
            }
            Some("messageInterval") => {
                let whole = |key: &str| action.get(key).and_then(Value::as_i64);
                let (Some(component), Some(message), Some(rate)) = (whole("component").and_then(|c| u8::try_from(c).ok()), whole("message"), whole("rate")) else {
                    return Err("A message rate names a component, a message and a rate.".to_string());
                };
                let interval = if rate > 0 { 1_000_000.0 / rate as f64 } else { rate as f64 };
                let set = Outbound::CommandLong { target: (self.id, component), command: 511, params: [message as f64, interval, 0.0, 0.0, 0.0, 0.0, 0.0] };
                let ask = Outbound::CommandLong { target: (self.id, component), command: 512, params: [244.0, message as f64, 0.0, 0.0, 0.0, 0.0, 0.0] };
                return Ok([set, ask].iter().filter_map(|send| self.encode(send)).collect());
            }
            Some("shellCommand") => {
                let Some(command) = action.get("command").and_then(Value::as_str) else {
                    return Err("A console command is text.".to_string());
                };
                self.shell.command_sent();
                let target = (self.id, self.component);
                return Ok(crate::shell::Shell::chunks(command).into_iter().filter_map(|data| self.encode(&Outbound::ShellData { target, data })).collect());
            }
            Some(log @ ("logRefresh" | "logDownload" | "logCancel" | "logEraseAll" | "logEraseSelected")) => return Ok(self.onboard_log_action(log, action.get("folder").and_then(Value::as_str), now_ms)),
            _ => {}
        }
        if self.guided.running() {
            return Err("A guided action is still running.".to_string());
        }
        match self.plan(action) {
            Plan::Refused(reason) => Err(reason),
            Plan::Steps(steps) => {
                self.errors.clear();
                if action.get("action").and_then(Value::as_str) == Some("roi") {
                    let number = |key: &str| action.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN);
                    self.roi_coord = Some((number("latitude"), number("longitude"), number("altitude")));
                }
                let emits = self.guided.start(steps, &self.observed(), now_ms);
                Ok(self.carry(emits, now_ms))
            }
        }
    }

    fn note(&mut self, error: String) {
        self.errors.push(error);
        let excess = self.errors.len().saturating_sub(MAX_ERRORS);
        self.errors.drain(..excess);
    }

    fn encode(&mut self, send: &Outbound) -> Option<Vec<u8>> {
        let bytes = mavout::encode_next(send);
        if bytes.is_none() {
            self.note(format!("Unable to encode {send:?}"));
        }
        bytes
    }

    fn send_terrain(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        let Some(request) = self.terrain_request else {
            self.terrain_due = None;
            return Vec::new();
        };
        let step = crate::terrainprotocol::step(&request, crate::terrainservice::cached_height);
        self.terrain_due = (step != crate::terrainprotocol::Step::Done).then_some(now_ms + crate::terrainprotocol::SEND_INTERVAL_MS);
        match step {
            crate::terrainprotocol::Step::Done => {
                self.terrain_request = None;
                Vec::new()
            }
            crate::terrainprotocol::Step::Wait => Vec::new(),
            crate::terrainprotocol::Step::Skip { bit } => {
                self.terrain_request = Some(crate::terrainprotocol::without(request, bit));
                Vec::new()
            }
            crate::terrainprotocol::Step::Send { bit, data } => {
                self.terrain_request = Some(crate::terrainprotocol::without(request, bit));
                self.encode(&Outbound::TerrainData { lat: request.lat, lon: request.lon, grid_spacing: request.grid_spacing, gridbit: bit, data }).into_iter().collect()
            }
        }
    }

    fn send_clock(&mut self) -> Vec<Vec<u8>> {
        let time = Outbound::SystemTime { time_unix_usec: now_us() };
        [&time, &time].into_iter().filter_map(|send| self.encode(send)).collect()
    }

    fn send_tagged(&mut self, command: u16, params: [f64; 7], show_error: bool, tag: u64, now_ms: u64) -> Vec<Vec<u8>> {
        let outs = self.commands.send(Command { component: self.component, command, command_int: false, frame: guidedcmd::FRAME_GLOBAL, params, show_error, tag }, now_ms);
        self.handle(outs, now_ms)
    }

    fn carry(&mut self, emits: Vec<Emit>, now_ms: u64) -> Vec<Vec<u8>> {
        let target = (self.id, self.component);
        emits
            .into_iter()
            .flat_map(|emit| match emit {
                Emit::Command { command, params, command_int, frame, show_error } => {
                    let outs = self.commands.send(Command { component: self.component, command, command_int, frame, params, show_error, tag: 0 }, now_ms);
                    self.handle(outs, now_ms)
                }
                Emit::SetMode { base_mode, custom_mode } => self.encode(&Outbound::SetMode { system: self.id, base_mode, custom_mode }).into_iter().collect(),
                Emit::PositionTargetLocalNed { frame, type_mask, x, y, z } => self.encode(&Outbound::PositionTargetLocalNed { target, frame, type_mask, x, y, z }).into_iter().collect(),
                Emit::GuidedMissionItem { latitude, longitude, altitude_relative } => self.encode(&Outbound::GuidedMissionItem { target, latitude, longitude, altitude_relative }).into_iter().collect(),
            })
            .collect()
    }

    fn handle(&mut self, outs: Vec<Out>, now_ms: u64) -> Vec<Vec<u8>> {
        let target = (self.id, self.component);
        outs.into_iter()
            .flat_map(|out| match out {
                Out::Send { command: crate::operatorcontrol::REQUEST_OPERATOR_CONTROL, params, .. } => self.encode(&Outbound::RawCommandLong { target, command: crate::operatorcontrol::REQUEST_OPERATOR_CONTROL, params }).into_iter().collect(),
                Out::Send { command, command_int: false, params, .. } => self.encode(&Outbound::CommandLong { target, command, params }).into_iter().collect(),
                Out::Send { command, command_int: true, frame, params, x, y, .. } => self.encode(&Outbound::CommandInt { target, command, frame, params, x, y }).into_iter().collect(),
                Out::ShowError(text) => {
                    self.pending_notices.push((crate::noticeboard::MESSAGE, text.clone()));
                    self.note(text);
                    Vec::new()
                }
                Out::Result { tag: FACTORY_RESET_TAG, result, .. } => {
                    let reset = result == RESULT_ACCEPTED;
                    crate::noticeboard::post(crate::noticeboard::MESSAGE, "", if reset { "Reset successful" } else { "Reset failed" });
                    self.sensor_refresh_due = reset.then_some(now_ms + SENSOR_REFRESH_DELAY_MS);
                    Vec::new()
                }
                Out::Result { tag: REBOOT_TAG, result, .. } => {
                    match result {
                        RESULT_ACCEPTED => crate::corelinks::close_vehicle_at(now_ms, self.id, self.link_states.iter().map(|(link, _, _)| *link).collect(), "the vehicle accepted a reboot"),
                        _ => self.pending_notices.push((crate::noticeboard::MESSAGE, "Vehicle reboot failed.".to_string())),
                    }
                    Vec::new()
                }
                Out::Result { command: crate::operatorcontrol::REQUEST_OPERATOR_CONTROL, failure: failure @ (Failure::NoResponse | Failure::Duplicate), .. } => {
                    let text = match failure {
                        Failure::Duplicate => "Waiting for previous operator control request",
                        _ => "No response to operator control request",
                    };
                    self.pending_notices.push((crate::noticeboard::MESSAGE, text.to_string()));
                    Vec::new()
                }
                Out::Result { command: sensorcal::CMD_DO_CANCEL_MAG_CAL, failure: Failure::NoResponse, result, .. } => {
                    let calibration = self.calibrate.on_ack(sensorcal::CMD_DO_CANCEL_MAG_CAL, result, now_ms);
                    self.follow_calibration(calibration, now_ms)
                }
                Out::Result { command: CMD_DO_REPOSITION, result, failure: Failure::ResultOnly, .. } => {
                    self.reposition_supported = match result {
                        RESULT_ACCEPTED => Some(true),
                        RESULT_UNSUPPORTED => Some(false),
                        _ => self.reposition_supported,
                    };
                    Vec::new()
                }
                Out::RequestResult { message_id: MSG_AUTOPILOT_VERSION, .. } => {
                    let actions = self.connect.on_autopilot_version(&self.connect_link(), &self.connect_vehicle(), self.autopilot_version.as_ref());
                    self.follow_connect(actions, now_ms)
                }
                Out::RequestResult { message_id: MSG_PROTOCOL_VERSION, .. } => {
                    let actions = self.connect.on_protocol_version(&self.connect_link(), &self.connect_vehicle(), self.max_proto_version);
                    self.follow_connect(actions, now_ms)
                }
                Out::Result { command: CMD_LOGGING_START, result, .. } if result != RESULT_ACCEPTED => {
                    self.log_denied |= result == RESULT_DENIED;
                    self.discard_log(match result {
                        RESULT_DENIED => "Start MAVLink log command denied.".to_string(),
                        other => format!("Start MAVLink log command failed: {other}"),
                    });
                    Vec::new()
                }
                Out::RequestResult { message_id: MSG_COMPONENT_METADATA, failure, .. } if failure != crate::mavcmd::RequestFailure::None => self.step_done(connect::Step::ComponentInformation, now_ms),
                Out::RequestResult { message_id: MSG_AVAILABLE_MODES, failure, .. } if failure != crate::mavcmd::RequestFailure::None => {
                    let outs = self.modes.on_message(false, None);
                    self.follow_modes(outs, now_ms)
                }
                _ => Vec::new(),
            })
            .collect()
    }

    pub fn pump(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        self.pump_with(now_ms, None, 0)
    }

    fn pump_with(&mut self, now_ms: u64, remote_inputs: Option<&RemoteInputs>, now_s: u64) -> Vec<Vec<u8>> {
        let ticked = self.commands.tick(now_ms);
        self.control.tick(now_ms);
        let mut bytes = self.handle(ticked, now_ms);
        if self.control.revert_due(now_ms, mavout::gcs_system()) {
            let revert = Outbound::RawCommandLong { target: (self.id, self.component), command: crate::operatorcontrol::REQUEST_OPERATOR_CONTROL, params: [0.0, 1.0, 0.0, crate::operatorcontrol::DEFAULT_REQUEST_TIMEOUT_SECS as f64, 0.0, 0.0, 0.0] };
            bytes.extend(self.encode(&revert));
        }
        bytes.extend(self.tick_rc_override(now_ms));
        bytes.extend(self.tick_airframe_reboot(now_ms));
        bytes.extend(self.tick_stream_rates(now_ms));
        bytes.extend(self.tick_autotune(now_ms));
        let event_retry = self.events.receiver.on_tick(now_ms);
        bytes.extend(self.event_request(event_retry));
        let actuator = self.actuator_test.tick(now_ms);
        bytes.extend(self.actuator_request(actuator));
        let spin = self.motor_assignment.tick(now_ms);
        bytes.extend(self.motor_spin(spin));
        self.cameras.high_latency = self.commands.high_latency;
        let camera_due = self.cameras.tick(now_ms);
        bytes.extend(self.camera_commands(camera_due));
        let was_busy = self.onboard_logs.busy();
        let log_due = self.onboard_logs.on_timeout(now_ms);
        bytes.extend(self.onboard_log_outs(was_busy, log_due, now_ms));
        if self.chunk_due.is_some_and(|due| now_ms >= due) {
            self.chunk_due = None;
            let ardupilot = self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT;
            let expired: Vec<Vec<u8>> = self.status_text.expire_pending().into_iter().map(|status| calibration_as_info(status, ardupilot)).flat_map(|status| self.take_status(status, now_ms)).collect();
            bytes.extend(expired);
        }
        if self.gimbal_rate_due.is_some_and(|due| now_ms >= due) {
            let outs = crate::gimbal::lock().set_rates(None, None, now_ms);
            bytes.extend(self.gimbal_outs(outs, now_ms).unwrap_or_default());
        }
        let stalled = self.calibrate.tick(now_ms);
        bytes.extend(self.follow_calibration(stalled, now_ms));
        if self.odid_due.is_some_and(|due| now_ms >= due) {
            self.odid_due = None;
            let outs = self.remote.on_odid_timeout();
            self.follow_remote(outs, now_ms);
        }
        if self.odid_send_due.is_some_and(|due| now_ms >= due) {
            self.odid_send_due = Some(now_ms + ODID_SEND_MS);
            bytes.extend(self.send_remote_id(remote_inputs, now_ms, now_s));
        }
        if self.initial_due.is_some_and(|due| now_ms >= due) {
            self.initial_due = None;
            let actions = self.params.on_initial_timeout();
            bytes.extend(self.follow_params(actions, now_ms));
        }
        if self.terrain_due.is_some_and(|due| now_ms >= due) {
            bytes.extend(self.send_terrain(now_ms));
        }
        if self.sensor_refresh_due.is_some_and(|due| now_ms >= due) {
            self.sensor_refresh_due = None;
            let component = self.component;
            let actions: Vec<params::Action> = self.parameters(component).into_iter().filter(|(name, _)| sensor_parameter(name)).flat_map(|(name, _)| self.params.refresh(component, &name)).collect();
            bytes.extend(self.follow_params(actions, now_ms));
        }
        if self.waiting_due.is_some_and(|due| now_ms >= due) {
            self.waiting_due = None;
            let actions = self.params.on_waiting_timeout();
            bytes.extend(self.follow_params(actions, now_ms));
        }
        let due: Vec<u8> = [PLAN_MISSION, PLAN_FENCE, PLAN_RALLY].into_iter().filter(|k| self.plans[*k as usize].due.is_some_and(|due| now_ms >= due)).collect();
        due.into_iter().for_each(|kind| {
            self.plans[kind as usize].due = None;
            let outs = self.plans[kind as usize].transfer.on_timeout();
            bytes.extend(self.follow_plan(kind, outs, now_ms));
        });
        if self.files.due_ms.is_some_and(|due| now_ms >= due) {
            self.files.due_ms = None;
            let steps = self.files.job.as_mut().map(crate::filejobs::Job::on_timeout).unwrap_or_default();
            bytes.extend(self.follow_files(steps, now_ms));
        }
        if self.ftp_due.is_some_and(|due| now_ms >= due) {
            self.ftp_due = None;
            let outs = self.fetch.as_mut().map(|f| f.download.on_timeout()).unwrap_or_default();
            bytes.extend(self.follow_ftp(outs, now_ms));
        }
        let slow = self.fetch.as_ref().filter(|f| compmeta::too_slow(now_ms.saturating_sub(f.started_ms), f.progress)).map(|f| f.kind);
        if let Some(kind) = slow {
            let outs = self.fetch.as_mut().map(|f| f.download.cancel()).unwrap_or_default();
            bytes.extend(self.follow_ftp(outs, now_ms));
            self.fetch = None;
            self.ftp_due = None;
            bytes.extend(self.fetch_failed(kind, "Component metadata download abandoned: too slow.".to_string(), now_ms));
        }
        let emits = self.guided.advance(&self.observed(), now_ms);
        bytes.extend(self.carry(emits, now_ms));
        bytes
    }

    pub fn guided_snapshot(&self) -> Value {
        let mut snapshot = self.guided.snapshot();
        snapshot["errors"] = json!(self.errors.iter().rev().take(10).collect::<Vec<_>>());
        snapshot["repositionSupported"] = json!(self.reposition_supported);
        snapshot
    }

    fn spoken_status(&self, status: &StatusText, now_ms: u64) -> Option<String> {
        let px4 = self.autopilot == crate::modes::AUTOPILOT_PX4;
        let text = crate::messagelog::admitted(px4, self.events.supports_checks(status.component), status.severity, &status.text)?;
        let repeated = is_prearm(&text, status.severity) && self.prearm_spoken.get(&text).is_some_and(|at| now_ms.saturating_sub(*at) < PREARM_REPEAT_MS);
        let asked = status.text.starts_with('#') || status.severity <= SEVERITY_NOTICE;
        (asked && !repeated).then_some(text)
    }

    fn announce_battery(&mut self, id: u8, charge_state: u8) {
        let lowest = *self.battery_announced.entry(id).or_insert(charge_state);
        let message = match charge_state {
            BATTERY_OK => {
                self.battery_announced.insert(id, charge_state);
                None
            }
            BATTERY_LOW if charge_state > lowest => Some("battery {} level low"),
            BATTERY_CRITICAL if charge_state > lowest => Some("battery {} level is critical"),
            BATTERY_EMERGENCY if charge_state > lowest => Some("battery {} level emergency"),
            BATTERY_FAILED if charge_state > lowest => Some("battery {} failed"),
            BATTERY_UNHEALTHY if charge_state > lowest => Some("battery {} unhealthy"),
            _ => None,
        };
        if let Some(message) = message {
            self.battery_announced.insert(id, charge_state);
            let numbered = if self.batteries.by_id.len() > 1 { id.to_string() } else { String::new() };
            crate::speech::say("warning");
            crate::speech::say(&format!("{} {} ", self.speech_prefix, message.replace("{}", &numbered)).to_lowercase());
        }
    }

    fn announce_fence(&mut self, breached: bool, breach_type: u8, now_ms: u64) {
        let kind = match breach_type {
            FENCE_BREACH_NONE => return,
            FENCE_BREACH_MINALT => "minimum altitude",
            FENCE_BREACH_MAXALT => "maximum altitude",
            FENCE_BREACH_BOUNDARY => "boundary",
            _ => "",
        };
        match breached {
            true if now_ms.saturating_sub(self.fence_quiet_ms) > FENCE_SPEECH_GAP_MS => {
                self.fence_quiet_ms = now_ms;
                crate::speech::say(&format!("{kind} fence breached"));
            }
            true => {}
            false => self.fence_quiet_ms = now_ms,
        }
    }

    pub fn announce(&mut self, prefix: &str) {
        self.speech_prefix = prefix.to_string();
        let (mode, armed, lost) = (self.flight_mode(), self.armed(), self.connection_lost);
        let (last_mode, last_armed, last_lost) = self.announced.clone();
        if !mode.is_empty() && last_mode.as_deref() != Some(mode.as_str()) {
            crate::speech::say(&format!("{prefix} {mode} flight mode").to_lowercase());
        }
        if armed != last_armed {
            crate::speech::say(&format!("{prefix} {}", if armed { "armed" } else { "disarmed" }).to_lowercase());
        }
        if lost != last_lost && (lost || self.link_states.len() <= 1) {
            crate::speech::say(&format!("{prefix}{}", if lost { "Communication lost" } else { "Communication regained" }).to_lowercase());
        }
        self.announced = (if mode.is_empty() { last_mode } else { Some(mode) }, armed, lost);
    }

    fn take_status(&mut self, status: StatusText, now_ms: u64) -> Vec<Vec<u8>> {
        if let Some(spoken) = self.spoken_status(&status, now_ms) {
            crate::speech::say(&spoken.to_lowercase());
        }
        self.log_status(&status);
        crate::escal::on_text(self.id, &status.text);
        crate::apmsubmotors::on_text(self.flight_mode() == self.announced_name(crate::apmsubmotors::MOTOR_DETECTION_MODE), &status.text);
        let actions = self.calibrate.on_text(&status.text, now_ms);
        let bytes = self.follow_calibration(actions, now_ms);
        self.note_prearm(&status.text, status.severity, status.component, now_ms);
        self.recent.push(status);
        if self.recent.len() > MAX_MESSAGES {
            self.recent.remove(0);
        }
        bytes
    }

    fn note_prearm(&mut self, text: &str, severity: u8, component: u8, now_ms: u64) {
        if !is_prearm(text, severity) || self.events.supports_checks(component) {
            return;
        }
        let recently = self.prearm_spoken.get(text).is_some_and(|at| now_ms.saturating_sub(*at) < PREARM_REPEAT_MS);
        if !recently {
            self.prearm_spoken.insert(text.to_string(), now_ms);
            self.prearm = Some((text.to_string(), now_ms));
        }
    }

    fn direct_link_alive(&self) -> bool {
        self.link_states.iter().any(|(link, _, lost)| !lost && !self.link_kinds.cloud.contains(link) && !self.link_kinds.high_latency.contains(link))
    }

    fn best_primary_link(&self) -> Option<LinkId> {
        let kinds = &self.link_kinds;
        let live: Vec<LinkId> = self.link_states.iter().filter(|(_, _, lost)| !lost).map(|(link, _, _)| *link).collect();
        let normal = |cloud: bool| live.iter().copied().find(|link| kinds.cloud.contains(link) == cloud && !kinds.high_latency.contains(link));
        let held_high_latency = self.primary_link.filter(|link| kinds.high_latency.contains(link) && self.link_states.iter().any(|(id, _, _)| id == link));
        live.iter().copied().find(|link| kinds.usb_direct.contains(link))
            .or_else(|| normal(false))
            .or_else(|| normal(true))
            .or(held_high_latency)
            .or_else(|| live.iter().copied().find(|link| kinds.high_latency.contains(link)))
    }

    fn update_primary_link(&mut self) -> bool {
        let held = self.primary_link.and_then(|id| self.link_states.iter().find(|(link, _, _)| *link == id)).copied();
        let kinds = &self.link_kinds;
        let keep = held.is_some_and(|(link, _, lost)| !lost && !kinds.high_latency.contains(&link) && !(kinds.cloud.contains(&link) && self.direct_link_alive()));
        if keep {
            return false;
        }
        let best = self.best_primary_link();
        if held.is_some() && best.is_none() || best == self.primary_link {
            return false;
        }
        let switched = held.is_some() && best != self.primary_link;
        let stop = self.primary_link.filter(|link| self.link_kinds.high_latency.contains(link)).map(|link| (link, 0.0));
        let start = best.filter(|link| self.link_kinds.high_latency.contains(link)).map(|link| (link, 1.0));
        let target = (self.id, COMP_AUTOPILOT1);
        let frames: Vec<(LinkId, Vec<u8>)> = stop.into_iter().chain(start).filter_map(|(link, on)| self.encode(&Outbound::CommandLong { target, command: CMD_CONTROL_HIGH_LATENCY, params: [on, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] }).map(|bytes| (link, bytes))).collect();
        self.link_frames.extend(frames);
        self.primary_link = best;
        if let Some(sending) = best {
            self.link = sending;
        }
        self.commands.high_latency = best.is_some_and(|link| self.link_kinds.high_latency.contains(&link));
        switched
    }

    fn say_link(&self, text: &str) {
        crate::speech::say(&format!("{}{text}", self.speech_prefix).to_lowercase());
    }

    fn link_role(&self, link: LinkId) -> &'static str {
        if self.primary_link == Some(link) { "primary" } else { "secondary" }
    }

    fn announce_switch(&mut self, text: &str) {
        self.say_link(text);
        self.pending_notices.push((crate::noticeboard::MESSAGE, format!("{}{text}", self.speech_prefix)));
    }

    pub fn is_standby(&self, link: LinkId) -> bool {
        let cloud = &self.link_kinds.cloud;
        self.primary_link.is_some_and(|primary| primary != link && self.link_states.len() >= 2 && (cloud.contains(&link) || cloud.contains(&primary)))
    }

    pub fn note_link(&mut self, link: LinkId, now_ms: u64) {
        match self.link_states.iter_mut().find(|(id, _, _)| *id == link) {
            Some(state) => {
                state.1 = now_ms;
                if state.2 {
                    state.2 = false;
                    self.connection_lost = false;
                    log::info!("Communication regained on link {link}");
                    if self.link_states.len() > 1 {
                        self.say_link(&format!("Communication regained on {} link", self.link_role(link)));
                    }
                    if self.update_primary_link() {
                        self.announce_switch("Switching communication to new primary link");
                    }
                }
            }
            None => {
                self.link_states.push((link, now_ms, false));
                self.update_primary_link();
            }
        }
    }

    pub fn check_links(&mut self, now_ms: u64) {
        self.commands.high_latency = self.primary_link.or(Some(self.link)).is_some_and(|link| self.link_kinds.high_latency.contains(&link));
        if !self.comm_lost_enabled || self.calibrate.mutes_comm_lost() {
            return;
        }
        let high_latency = self.link_kinds.high_latency.clone();
        let cloud = self.link_kinds.cloud.clone();
        let silenced: Vec<LinkId> = self.link_states.iter().filter(|(link, last, lost)| !lost && !high_latency.contains(link) && now_ms.saturating_sub(*last) > LINK_SILENT_MS).map(|(link, _, _)| *link).collect();
        let several = self.link_states.len() > 1;
        silenced.iter().for_each(|link| {
            log::warn!("Communication lost on link {link}");
            if several {
                self.say_link(&format!("Communication lost on {} link.", self.link_role(*link)));
            }
        });
        self.link_states.iter_mut().filter(|(link, _, _)| silenced.contains(link)).for_each(|state| state.2 = true);
        self.connection_lost = !self.link_states.is_empty() && self.link_states.iter().all(|(_, _, lost)| *lost);
        let was_relayed = self.primary_link.is_some_and(|link| cloud.contains(&link));
        if self.update_primary_link() {
            let back_to_direct = was_relayed && self.primary_link.is_some_and(|link| !cloud.contains(&link));
            let text = if back_to_direct { "Switching communication back to the direct link." } else { "Switching communication to secondary link." };
            self.announce_switch(text);
        }
    }

    fn log_extension(&self) -> &'static str {
        match self.autopilot {
            crate::modes::AUTOPILOT_PX4 if self.parameter(self.component, "SYS_LOGGER").is_some_and(|p| p.as_f64() == 0.0) => ".px4log",
            crate::modes::AUTOPILOT_PX4 => ".ulg",
            _ => ".bin",
        }
    }

    fn onboard_log_outs(&mut self, was_busy: bool, outs: Vec<crate::onboardlogs::Out>, now_ms: u64) -> Vec<Vec<u8>> {
        use crate::onboardlogs::Out;
        let target = (self.id, self.component);
        let sent: Vec<Vec<u8>> = outs
            .into_iter()
            .flat_map(|out| {
                let send = match out {
                    Out::RequestList { start, end } => Outbound::LogRequestList { target, start, end },
                    Out::RequestData { id, offset, count } => Outbound::LogRequestData { target, id, offset, count },
                    Out::RequestEnd => Outbound::LogRequestEnd { target },
                    Out::Erase => Outbound::LogErase { target },
                    Out::FtpList(path) => return self.log_file_job(LogFileJob::List(path), now_ms),
                    Out::FtpDownload { path, local } => return self.log_file_job(LogFileJob::Download(path, local), now_ms),
                    Out::FtpDelete(path) => return self.log_file_job(LogFileJob::Delete(path), now_ms),
                    Out::FtpCancel => {
                        let steps = self.files.job.as_mut().filter(|_| self.files_for_logs).map(crate::filejobs::Job::cancel).unwrap_or_default();
                        return self.follow_files(steps, now_ms);
                    }
                };
                self.encode(&send).into_iter().collect()
            })
            .collect();
        let busy = self.onboard_logs.busy();
        if busy != was_busy {
            self.comm_lost_enabled = !busy;
        }
        sent
    }

    fn log_file_job(&mut self, wanted: LogFileJob, now_ms: u64) -> Vec<Vec<u8>> {
        let (component, seq) = (self.component, self.ftp_seq);
        let started = match (self.files.busy() || self.fetch.is_some(), &wanted) {
            (true, _) => Err(FILES_BUSY.to_string()),
            (false, LogFileJob::List(path)) if self.ftp_list_time_unsupported => crate::filejobs::Job::list(component, path, seq),
            (false, LogFileJob::List(path)) => crate::filejobs::Job::list_with_time(component, path, seq),
            (false, LogFileJob::Download(path, _)) => crate::filejobs::Job::download_sized(component, path, seq, true),
            (false, LogFileJob::Delete(path)) => crate::filejobs::Job::delete(component, path, seq),
        };
        match started {
            Ok((job, steps)) => {
                self.download_to = match &wanted {
                    LogFileJob::Download(_, local) => Some(local.to_string_lossy().into_owned()),
                    _ => None,
                };
                self.files_for_logs = true;
                self.files.job = Some(job);
                self.files.progress = 0.0;
                self.files.generation += 1;
                self.follow_files(steps, now_ms)
            }
            Err(error) => self.log_file_job_done(&wanted.kind(), Err(error), now_ms),
        }
    }

    fn log_file_job_done(&mut self, kind: &LogFileKind, result: Result<crate::filejobs::Outcome, String>, now_ms: u64) -> Vec<Vec<u8>> {
        let was_busy = self.onboard_logs.busy();
        let outs = match (kind, result) {
            (LogFileKind::List, Ok(crate::filejobs::Outcome::Listed(entries))) => self.onboard_logs.on_ftp_listed(Ok(entries), self.ftp_list_time_unsupported, now_ms),
            (LogFileKind::List, Err(error)) => self.onboard_logs.on_ftp_listed(Err(error), self.ftp_list_time_unsupported, now_ms),
            (LogFileKind::Download, result) => self.onboard_logs.on_ftp_downloaded(result.map(|_| ()), now_ms),
            (LogFileKind::Delete, _) => self.onboard_logs.on_ftp_deleted(now_ms),
            (LogFileKind::List, Ok(_)) => Vec::new(),
        };
        self.onboard_log_outs(was_busy, outs, now_ms)
    }

    pub fn onboard_log_action(&mut self, action: &str, folder: Option<&str>, now_ms: u64) -> Vec<Vec<u8>> {
        let was_busy = self.onboard_logs.busy();
        let extension = self.log_extension();
        self.onboard_logs.ftp_capable = self.capabilities_known && self.capabilities & crate::connect::CAP_FTP != 0;
        self.onboard_logs.set_ftp_fallback_root(match self.autopilot {
            crate::modes::AUTOPILOT_PX4 => Some(crate::onboardlogs::PX4_LOG_ROOT),
            crate::modes::AUTOPILOT_ARDUPILOT => Some(crate::onboardlogs::APM_LOG_ROOT),
            _ => None,
        });
        let outs = match action {
            "logRefresh" => self.onboard_logs.refresh(now_ms),
            "logDownload" => self.onboard_logs.download(std::path::Path::new(folder.unwrap_or("")), extension, now_ms),
            "logCancel" => self.onboard_logs.cancel(),
            "logEraseAll" => self.onboard_logs.erase_all(now_ms),
            "logEraseSelected" => self.onboard_logs.erase_selected(),
            _ => Vec::new(),
        };
        self.onboard_log_outs(was_busy, outs, now_ms)
    }

    fn log_status(&mut self, status: &StatusText) {
        let px4 = self.autopilot == crate::modes::AUTOPILOT_PX4;
        if let Some(text) = crate::messagelog::admitted(px4, self.events.supports_checks(status.component), status.severity, &status.text) {
            if status.severity <= SEVERITY_ERROR {
                self.pending_notices.push((crate::noticeboard::VEHICLE_ERROR, text.clone()));
            }
            self.message_log.record(status.component, status.severity, text, crate::messagelog::clock_now());
        }
    }

    pub fn rccal_vehicle(&self) -> crate::rccal::Vehicle {
        let class = crate::plandoc::vehicle_class(i64::from(self.vehicle_type));
        crate::rccal::Vehicle { px4: self.autopilot == crate::modes::AUTOPILOT_PX4, multi_rotor: class == crate::cmdinfo::VehicleClass::MultiRotor, helicopter: self.vehicle_type == 4, rover: class == crate::cmdinfo::VehicleClass::Rover }
    }

    pub fn load_rccal(&mut self) {
        if self.rccal_loaded || !self.parameters_ready() {
            return;
        }
        let vehicle = self.rccal_vehicle();
        let mode = crate::settingsstore::stored_text("RadioCalibration/TransmitterMode").and_then(|t| t.trim().parse().ok()).unwrap_or(2);
        let mut cal = crate::rccal::RcCal::for_vehicle(&vehicle, mode);
        cal.channel_values(&self.rccal.rc_values(), 0);
        cal.read_stored(&vehicle, &|name: &str| self.params.value(self.component, name).map(|p| p.as_f64()));
        (self.rccal, self.rccal_loaded) = (cal, true);
    }

    fn note_onboard_log(&mut self, message: &MavMessage, now_ms: u64) -> Vec<Vec<u8>> {
        let was_busy = self.onboard_logs.busy();
        let apm = self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT;
        let outs = match message {
            MavMessage::LOG_ENTRY(d) => {
                self.onboard_logs.on_entry(apm, d.time_utc, d.size, d.id, d.num_logs, now_ms);
                Vec::new()
            }
            MavMessage::LOG_DATA(d) => self.onboard_logs.on_data(d.ofs, d.id, &d.data[..usize::from(d.count).min(d.data.len())], now_ms),
            _ => return Vec::new(),
        };
        self.onboard_log_outs(was_busy, outs, now_ms)
    }

    fn camera_action(&mut self, action: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        use crate::cameraproto::Axis;
        let direction = action.get("direction").and_then(Value::as_i64).unwrap_or(0) as i32;
        let axis = match action.get("axis").and_then(Value::as_str) {
            Some("focus") => Axis::Focus,
            _ => Axis::Zoom,
        };
        let wrap = |current: Option<usize>, count: usize| -> Option<usize> {
            (count > 0).then(|| (current.unwrap_or(0) as i64 + i64::from(direction)).rem_euclid(count as i64) as usize)
        };
        let commands = match action.get("op").and_then(Value::as_str).unwrap_or("") {
            "step" => self.cameras.step(axis, direction, now_ms),
            "slew" => self.cameras.slew(axis, direction, now_ms),
            "stepCamera" => {
                let next = wrap(self.cameras.selected_index(), self.cameras.count()).and_then(|i| self.cameras.compid_at(i));
                return match next.map(|compid| self.cameras.select(compid)) {
                    Some(Ok(())) => Ok(Vec::new()),
                    _ => Err("No camera is connected.".to_string()),
                };
            }
            "selectStream" => {
                let index = action.get("stream").and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok());
                let chosen = self.cameras.selected().zip(index).and_then(|(camera, i)| camera.listed_streams().get(i).map(|stream| stream.stream_id));
                match chosen {
                    Some(stream) => self.cameras.select_stream(stream, now_ms),
                    None => Err(crate::cameraproto::Refusal::UnknownStream),
                }
            }
            "stepStream" => {
                let (streams, current) = self.cameras.selected().map(|c| (c.streams.iter().map(|s| s.stream_id).collect::<Vec<_>>(), c.selected_stream)).unwrap_or_default();
                let at = current.and_then(|id| streams.iter().position(|s| *s == id));
                match wrap(at, streams.len()).and_then(|i| streams.get(i).copied()) {
                    Some(stream) => self.cameras.select_stream(stream, now_ms),
                    None => Err(crate::cameraproto::Refusal::UnknownStream),
                }
            }
            "takePhoto" => {
                let interval = action.get("interval").and_then(Value::as_f64);
                let count = action.get("count").and_then(Value::as_u64).and_then(|c| u32::try_from(c).ok()).unwrap_or(1);
                self.cameras.take_photo(interval, count, now_ms)
            }
            "stopTakePhoto" => self.cameras.stop_take_photo(now_ms),
            "startRecording" => self.cameras.start_recording(now_ms),
            "stopRecording" => self.cameras.stop_recording(now_ms),
            "toggleRecording" => match self.cameras.selected().is_some_and(|camera| camera.recording()) {
                true => self.cameras.stop_recording(now_ms),
                false => self.cameras.start_recording(now_ms),
            },
            "setMode" => self.cameras.set_mode(action.get("mode").and_then(Value::as_u64).and_then(|m| u8::try_from(m).ok()).unwrap_or(crate::cameraproto::MODE_PHOTO), now_ms),
            "toggleMode" => {
                let next = match self.cameras.selected().and_then(|camera| camera.mode) {
                    Some(crate::cameraproto::MODE_VIDEO) => crate::cameraproto::MODE_PHOTO,
                    _ => crate::cameraproto::MODE_VIDEO,
                };
                self.cameras.set_mode(next, now_ms)
            }
            "level" => self.cameras.set_level(axis, action.get("percent").and_then(Value::as_f64).unwrap_or(0.0), now_ms),
            "trackRect" | "trackPoint" | "stopTracking" => return self.camera_tracking(action),
            "reset" => self.cameras.reset_settings(now_ms),
            "format" => self.cameras.format_storage(action.get("storage").and_then(Value::as_u64).and_then(|s| u8::try_from(s).ok()).unwrap_or(1), now_ms),
            other => return Err(format!("Unknown camera action {other:?}")),
        };
        commands.map(|c| self.camera_commands(c)).map_err(|refusal| format!("The camera refused: {}", refusal.token()))
    }

    fn camera_tracking(&mut self, action: &Value) -> Result<Vec<Vec<u8>>, String> {
        use crate::cameraproto::Command;
        use crate::cameratrack::{CMD_STOP_TRACKING, CMD_TRACK_POINT, CMD_TRACK_RECTANGLE, MSG_TRACKING_IMAGE_STATUS, STATUS_INTERVAL_US};
        let compid = self.cameras.selected().map(|camera| camera.compid).ok_or("No camera is connected.")?;
        let number = |object: &Value, key: &str| object.get(key).and_then(Value::as_f64).ok_or(format!("Tracking needs {key}."));
        let interval = |rate: f64| Command { compid, command: crate::mavcmd::CMD_SET_MESSAGE_INTERVAL, params: [MSG_TRACKING_IMAGE_STATUS, rate, 0.0, 0.0, 0.0, 0.0, 0.0] };
        let commands = match action.get("op").and_then(Value::as_str) {
            Some("trackRect") => {
                let rect = &action["rect"];
                let (x, y) = (number(rect, "x")?, number(rect, "y")?);
                vec![Command { compid, command: CMD_TRACK_RECTANGLE, params: [x, y, x + number(rect, "width")?, y + number(rect, "height")?, 0.0, 0.0, 0.0] }, interval(STATUS_INTERVAL_US)]
            }
            Some("trackPoint") => {
                let point = &action["point"];
                vec![Command { compid, command: CMD_TRACK_POINT, params: [number(point, "x")?, number(point, "y")?, action.get("radius").and_then(Value::as_f64).unwrap_or(0.0), 0.0, 0.0, 0.0, 0.0] }, interval(STATUS_INTERVAL_US)]
            }
            _ => {
                self.camera_tracking_image = None;
                vec![Command { compid, command: CMD_STOP_TRACKING, params: [0.0; 7] }, interval(-1.0)]
            }
        };
        Ok(self.camera_commands(commands))
    }

    fn camera_commands(&mut self, commands: Vec<crate::cameraproto::Command>) -> Vec<Vec<u8>> {
        commands
            .into_iter()
            .filter_map(|c| {
                self.camera_sent.insert((c.compid, c.command), c.params[0]);
                self.encode(&Outbound::CommandLong { target: (self.id, c.compid), command: c.command, params: c.params })
            })
            .collect()
    }

    fn note_camera(&mut self, compid: u8, message: &MavMessage, now_ms: u64) -> Vec<Vec<u8>> {
        use crate::cameraproto::{CaptureStatusReport, Info, SettingsReport, StorageReport, StreamReport, StreamStatusReport};
        let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim_end_matches('\0').to_string();
        let commands = match message {
            MavMessage::HEARTBEAT(_) => self.cameras.on_heartbeat(compid, now_ms),
            MavMessage::CAMERA_INFORMATION(d) => {
                let info = Info {
                    vendor: text(&d.vendor_name),
                    model: text(&d.model_name),
                    firmware_version: d.firmware_version,
                    focal_length_mm: f64::from(d.focal_length),
                    sensor_size_h_mm: f64::from(d.sensor_size_h),
                    sensor_size_v_mm: f64::from(d.sensor_size_v),
                    resolution_h: d.resolution_h,
                    resolution_v: d.resolution_v,
                    flags: d.flags.bits(),
                    definition_version: d.cam_definition_version,
                    definition_uri: text(&d.cam_definition_uri[..]),
                    gimbal_device_id: d.gimbal_device_id,
                };
                crate::camsettings::wanted(crate::camsettings::Wanted { vehicle: self.id, compid, link: self.link, vendor: info.vendor.clone(), model: info.model.clone(), version: info.definition_version, uri: info.definition_uri.clone() });
                self.cameras.on_camera_information(compid, info, now_ms);
                Vec::new()
            }
            MavMessage::CAMERA_FOV_STATUS(d) => {
                let aspect = self.cameras.camera(compid).and_then(|camera| camera.info.aspect_vertical_over_horizontal());
                if let Some(vertical) = crate::cameraproto::vertical_field_of_view_degrees(f64::from(d.hfov), aspect) {
                    [("settings.gimbalControllerSettings.cameraHFov", f64::from(d.hfov)), ("settings.gimbalControllerSettings.cameraVFov", vertical)]
                        .into_iter()
                        .filter(|(path, value)| crate::settingsstore::raw_setting(path).and_then(|v| v.as_f64()) != Some(value.trunc()))
                        .for_each(|(path, value)| crate::settingsstore::set_raw(path, &json!(value.trunc() as u32)));
                }
                Vec::new()
            }
            MavMessage::CAMERA_SETTINGS(d) => {
                self.cameras.on_camera_settings(compid, SettingsReport { mode_id: d.mode_id as u8, zoom_percent: f64::from(d.zoomLevel), focus_percent: f64::from(d.focusLevel) }, now_ms);
                Vec::new()
            }
            MavMessage::STORAGE_INFORMATION(d) => {
                let report = StorageReport { storage_id: d.storage_id, storage_count: d.storage_count, status: d.status as u8, total_capacity_mib: f64::from(d.total_capacity), available_capacity_mib: f64::from(d.available_capacity) };
                self.cameras.on_storage_information(compid, report, now_ms);
                Vec::new()
            }
            MavMessage::CAMERA_CAPTURE_STATUS(d) => {
                let report = CaptureStatusReport { image_status: d.image_status, video_status: d.video_status, image_interval_s: f64::from(d.image_interval), recording_time_ms: d.recording_time_ms, available_capacity_mib: f64::from(d.available_capacity) };
                self.cameras.on_capture_status(compid, report, now_ms);
                Vec::new()
            }
            MavMessage::BATTERY_STATUS(d) if crate::cameraproto::is_camera_component(compid) => {
                self.cameras.on_battery_status(compid, d.battery_remaining, now_ms);
                Vec::new()
            }
            MavMessage::VIDEO_STREAM_INFORMATION(d) => {
                let report = StreamReport {
                    stream_id: d.stream_id,
                    count: d.count,
                    kind: d.mavtype as u8,
                    encoding: d.encoding as u8,
                    flags: d.flags.bits(),
                    framerate_hz: f64::from(d.framerate),
                    resolution_h: d.resolution_h,
                    resolution_v: d.resolution_v,
                    bitrate_bps: d.bitrate,
                    rotation_deg: f64::from(d.rotation),
                    hfov_deg: f64::from(d.hfov),
                    name: text(&d.name[..]),
                    uri: text(&d.uri[..]),
                };
                self.cameras.on_video_stream_information(compid, report, now_ms);
                Vec::new()
            }
            MavMessage::PARAM_EXT_ACK(d) => {
                crate::camsettings::on_ack(self.id, compid, &crate::camsettings::parameter_name(&d.param_id[..]), d.param_type as u8, &d.param_value[..], d.param_result as u8, now_ms);
                Vec::new()
            }
            MavMessage::PARAM_EXT_VALUE(d) => {
                crate::camsettings::on_value(self.id, compid, &crate::camsettings::parameter_name(&d.param_id[..]), d.param_type as u8, &d.param_value[..], now_ms);
                Vec::new()
            }
            MavMessage::VIDEO_STREAM_STATUS(d) => {
                let report = StreamStatusReport { stream_id: d.stream_id, flags: d.flags.bits(), framerate_hz: f64::from(d.framerate), resolution_h: d.resolution_h, resolution_v: d.resolution_v, bitrate_bps: d.bitrate, rotation_deg: f64::from(d.rotation), hfov_deg: f64::from(d.hfov) };
                self.cameras.on_video_stream_status(compid, report, now_ms);
                Vec::new()
            }
            _ => Vec::new(),
        };
        self.camera_commands(commands)
    }

    #[allow(deprecated)]
    fn note_mission_index(&mut self, message: &MavMessage) {
        let index = match message {
            MavMessage::MISSION_CURRENT(d) => i32::from(d.seq),
            MavMessage::HIGH_LATENCY(d) => i32::from(d.wp_num),
            MavMessage::HIGH_LATENCY2(d) => i32::from(d.wp_num),
            MavMessage::HEARTBEAT(_) => {
                if self.mission_cached_last != -1 && self.flight_mode() == crate::vehiclefacade::mission_flight_mode(self.autopilot, self.vehicle_type, &self.flight_modes) {
                    self.mission_last_current = self.mission_cached_last;
                    self.mission_cached_last = -1;
                }
                return;
            }
            _ => return,
        };
        self.mission_current = index;
        if self.mission_current != self.mission_last_current && self.mission_cached_last != self.mission_current {
            self.mission_cached_last = self.mission_current;
        }
    }

    pub fn mission_items(&self) -> &[plantransfer::Item] {
        self.plan_items(PLAN_MISSION)
    }

    pub fn plan_items(&self, kind: u8) -> &[plantransfer::Item] {
        &self.plans[kind as usize].transfer.items
    }

    pub fn fly_items(&self) -> usize {
        let held = self.plans[plantransfer::PLAN_MISSION as usize].transfer.items.len();
        match self.sends_home() {
            true => held.max(1),
            false => held + 1,
        }
    }

    pub fn current_mission_index(&self) -> i32 {
        self.mission_current + i32::from(!self.sends_home())
    }

    pub fn resume_mission_index(&self) -> i32 {
        let resume = self.mission_last_current + i32::from(!self.sends_home());
        let last_sequence = self.fly_items() as i32 - 1;
        if resume > 1 && resume != last_sequence { resume - 1 } else { 0 }
    }

    fn note_trigger_point(&mut self, message: &MavMessage) {
        let point = match message {
            MavMessage::CAMERA_IMAGE_CAPTURED(d) => {
                let first = !self.image_captured_seen;
                self.image_captured_seen = true;
                (!(first && !self.trigger_points.is_empty()) && d.capture_result.bits() == 1).then(|| (f64::from(d.lat) / 1e7, f64::from(d.lon) / 1e7, f64::from(d.alt)))
            }
            MavMessage::CAMERA_FEEDBACK(d) if !self.image_captured_seen => Some((f64::from(d.lat) / 1e7, f64::from(d.lng) / 1e7, f64::from(d.alt_msl))),
            _ => None,
        };
        if let Some(point) = point {
            self.trigger_points.push(point);
            self.trigger_points_appended = true;
        }
    }

    fn clear_trigger_points(&mut self) {
        self.image_captured_seen = false;
        self.trigger_points.clear();
    }

    pub fn orbit_active(&self, now_ms: u64) -> bool {
        self.orbit_heard_ms.is_some_and(|heard| now_ms.saturating_sub(heard) < ORBIT_TELEMETRY_TIMEOUT_MS)
    }

    pub fn speed_limits(&self) -> (bool, bool) {
        let has = |name: &str| self.parameter(self.component, name).is_some();
        match (self.parameters_ready(), self.autopilot) {
            (false, _) => (false, false),
            (true, crate::modes::AUTOPILOT_ARDUPILOT) => (has("WP_SPD") || has("WPNAV_SPEED"), has("AIRSPEED_MIN") && has("AIRSPEED_MAX")),
            (true, crate::modes::AUTOPILOT_PX4) => (has("MPC_XY_VEL_MAX"), has("FW_AIRSPD_MIN") && has("FW_AIRSPD_MAX")),
            _ => (false, false),
        }
    }

    pub fn prearm_error(&self, now_ms: u64) -> String {
        self.prearm.as_ref().filter(|(_, at)| now_ms.saturating_sub(*at) < PREARM_SHOWN_MS).map_or_else(String::new, |(text, _)| text.clone())
    }

    pub fn flight_time(&self, now_ms: u64) -> f64 {
        self.flight_started_ms.map_or(self.flight_seconds, |started| now_ms.saturating_sub(started) as f64 / 1000.0)
    }

    pub fn armed(&self) -> bool {
        self.armed_now
    }

    fn skips_download_flying(&self) -> bool {
        skips_download(self.autopilot, self.armed(), crate::settingsstore::raw_setting("settings.mavlinkSettings.noInitialDownloadWhenFlying").and_then(|v| v.as_bool()).unwrap_or(false))
    }

    fn arming_not_required(&self) -> bool {
        self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT && self.parameter(self.component, "ARMING_REQUIRE").is_some_and(|p| p.as_f64() == 0.0)
    }

    #[allow(deprecated)]
    fn apply(&mut self, header: &MavHeader, message: &MavMessage, timestamp_us: u64, now_ms: u64) -> Vec<Vec<u8>> {
        let was_armed = self.armed_now;
        self.messages += 1;
        *self.by_name.entry(message.message_name().to_string()).or_insert(0) += 1;
        let from = (header.system_id, header.component_id);
        if let MavMessage::HEARTBEAT(h) = message
            && header.system_id == self.id
        {
            if h.autopilot as u8 == crate::modes::AUTOPILOT_ARDUPILOT {
                self.ardupilot_components.insert(header.component_id);
            } else {
                self.ardupilot_components.remove(&header.component_id);
            }
        }
        match message {
            MavMessage::HEARTBEAT(h) if from == (self.id, self.component) => {
                self.heartbeats += 1;
                self.last_heartbeat_us = timestamp_us;
                self.connection_lost = false;
                self.base_mode = h.base_mode.bits();
                if !(self.arming_not_required() && self.status_bits.present & SENSOR_MOTOR_OUTPUTS != 0) {
                    self.armed_now = self.base_mode & ARMED_FLAG != 0;
                }
                self.custom_mode = h.custom_mode;
                self.system_status = h.system_status as u8;
                self.vehicle_type = h.mavtype as u8;
                if self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT && header.component_id == COMP_AUTOPILOT1 {
                    self.flying = self.armed() && matches!(self.system_status, MAV_STATE_ACTIVE | MAV_STATE_CRITICAL | MAV_STATE_EMERGENCY);
                }
            }
            MavMessage::HIGH_LATENCY(d) if from == (self.id, self.component) => {
                self.base_mode = crate::modes::FLAG_CUSTOM;
                self.custom_mode = high_latency_custom_mode(self.autopilot, d.custom_mode as u16);
                self.armed_now = true;
            }
            MavMessage::HIGH_LATENCY2(d) if from == (self.id, self.component) => {
                let apm = d.autopilot as u8 == crate::modes::AUTOPILOT_ARDUPILOT;
                self.base_mode = if apm { d.custom0 as u8 } else { crate::modes::FLAG_CUSTOM };
                self.custom_mode = high_latency_custom_mode(self.autopilot, d.custom_mode);
                self.armed_now = !apm || (d.custom0 as u8) & ARMED_FLAG != 0;
                let reported = high_latency_sensors(d.failure_flags.bits());
                if reported != self.status_bits.enabled {
                    self.status_bits = self.status_bits.after(reported, reported, reported);
                }
            }
            MavMessage::EXTENDED_SYS_STATE(e) if from == (self.id, self.component) => {
                let (flying, landing) = match e.landed_state as u8 {
                    LANDED_ON_GROUND => (Some(false), Some(false)),
                    LANDED_TAKEOFF | LANDED_IN_AIR => (Some(true), Some(false)),
                    LANDED_LANDING => (Some(true), Some(true)),
                    _ => (None, None),
                };
                self.flying = flying.unwrap_or(self.flying);
                self.landing = landing.filter(|_| self.armed()).unwrap_or(self.landing);
                if (19..=25).contains(&self.vehicle_type) {
                    self.vtol_in_forward_flight = e.vtol_state as u8 == VTOL_STATE_FW;
                }
            }
            MavMessage::COMMAND_ACK(a) => {
                if from == (self.id, self.component) {
                    self.guided.on_command_result(a.command as u32 as u16, a.result == mavlink::dialects::ardupilotmega::MavResult::MAV_RESULT_ACCEPTED);
                    if a.command == mavlink::dialects::ardupilotmega::MavCmd::MAV_CMD_DO_SET_MODE {
                        self.mode_ack = Some((a.result as u8, self.mode_ack.map_or(1, |(_, serial)| serial + 1)));
                    }
                }
                if a.result == mavlink::dialects::ardupilotmega::MavResult::MAV_RESULT_ACCEPTED {
                    match a.command {
                        mavlink::dialects::ardupilotmega::MavCmd::MAV_CMD_DO_SET_ROI_LOCATION => self.roi_enabled = true,
                        mavlink::dialects::ardupilotmega::MavCmd::MAV_CMD_FLASH_BOOTLOADER => self.pending_notices.push((crate::noticeboard::MESSAGE, "Bootloader flash succeeded".to_string())),
                        mavlink::dialects::ardupilotmega::MavCmd::MAV_CMD_DO_SET_ROI_NONE => {
                            self.roi_enabled = false;
                            self.roi_coord = None;
                        }
                        _ => {}
                    }
                }
                let camera = match crate::cameraproto::is_camera_component(header.component_id) {
                    true => {
                        let sent = a.command as u32 as u16;
                        let param1 = self.camera_sent.get(&(header.component_id, sent)).copied().unwrap_or(0.0);
                        let follow = self.cameras.on_command_result(header.component_id, sent, param1, a.result as u8, now_ms);
                        self.camera_commands(follow)
                    }
                    false => Vec::new(),
                };
                let calibration = self.calibrate.on_ack(a.command as u32 as u16, a.result as u8, now_ms);
                let announced = self.follow_calibration(calibration, now_ms);
                let outs = self.commands.on_ack(header.component_id, a.command as u32 as u16, a.result as u8, now_ms);
                if a.command as u32 as u16 == CMD_CONFIGURE_ACTUATOR {
                    self.actuator_action_pending = None;
                    if a.result as u8 != RESULT_ACCEPTED {
                        self.pending_notices.push((crate::noticeboard::MESSAGE, "Actuator action command failed".to_string()));
                    }
                }
                let tested = match a.command as u32 as u16 == crate::actuatortest::CMD_ACTUATOR_TEST {
                    true if self.motor_assignment.awaiting_ack() => {
                        if let Some(text) = self.motor_assignment.on_ack(a.result as u8) {
                            self.pending_notices.push((crate::noticeboard::MESSAGE, text.to_string()));
                        }
                        Vec::new()
                    }
                    true => {
                        let (next, message) = self.actuator_test.on_ack(a.result as u8, now_ms);
                        if let Some(text) = message {
                            self.pending_notices.push((crate::noticeboard::MESSAGE, text.to_string()));
                        }
                        self.actuator_request(next)
                    }
                    false => Vec::new(),
                };
                if a.command as u32 as u16 == crate::autotune::CMD_DO_AUTOTUNE_ENABLE {
                    if let Some(text) = self.autotune.on_ack(a.result as u8, a.progress) {
                        self.pending_notices.push((crate::noticeboard::MESSAGE, text.to_string()));
                    }
                }
                let streamed = match a.command as u32 as u16 == CMD_SET_MESSAGE_INTERVAL {
                    true => {
                        let next = self.stream.got_ack();
                        self.send_interval(next, now_ms)
                    }
                    false => Vec::new(),
                };
                return camera.into_iter().chain(announced).chain(self.handle(outs, now_ms)).chain(streamed).chain(tested).collect();
            }
            MavMessage::COMMAND_LONG(c) if c.command as u32 as u16 == sensorcal::CMD_ACCELCAL_VEHICLE_POS => {
                let actions = self.calibrate.on_accel_position(c.param1 as u32, now_ms);
                return self.follow_calibration(actions, now_ms);
            }
            MavMessage::MAG_CAL_PROGRESS(p) => {
                self.calibrate.on_mag_progress(p.compass_id, p.cal_mask, p.completion_pct, now_ms);
            }
            MavMessage::MAG_CAL_REPORT(r) => {
                let actions = self.calibrate.on_mag_report(r.compass_id, r.cal_status as u8, r.fitness as f64, now_ms);
                return self.follow_calibration(actions, now_ms);
            }
            MavMessage::TERRAIN_REQUEST(r) if from == (self.id, self.component) => {
                if !crate::terrainprotocol::accepts(r.lat, r.lon) {
                    return Vec::new();
                }
                self.terrain_request = Some(crate::terrainprotocol::Request { lat: r.lat, lon: r.lon, grid_spacing: r.grid_spacing, mask: r.mask });
                return self.send_terrain(now_ms);
            }
            MavMessage::CAMERA_TRACKING_IMAGE_STATUS(d) if self.cameras.selected().is_some_and(|camera| camera.compid == header.component_id) => {
                self.camera_tracking_image = crate::cameratrack::tracking_image(d.tracking_status.bits(), d.tracking_mode as u8, (d.point_x, d.point_y, d.radius), (d.rec_top_x, d.rec_top_y, d.rec_bottom_x, d.rec_bottom_y), self.camera_tracking_enabled);
                return Vec::new();
            }
            MavMessage::PING(p) if p.target_system == 0 && p.target_component == 0 => {
                return self.encode(&Outbound::Ping { time_usec: p.time_usec, seq: p.seq, target: (header.system_id, header.component_id) }).into_iter().collect();
            }
            MavMessage::AUTOPILOT_VERSION(v) => {
                if self.autopilot == crate::modes::AUTOPILOT_PX4 && !self.version_notified {
                    if let Some(notice) = crate::connectnotices::outdated_px4(v.flight_sw_version) {
                        self.version_notified = true;
                        crate::noticeboard::post(crate::noticeboard::MESSAGE, "", notice);
                    }
                }
                let outs = self.commands.on_message(header.component_id, MSG_AUTOPILOT_VERSION);
                if outs.is_empty() {
                    return Vec::new();
                }
                self.capabilities = v.capabilities.bits();
                self.autopilot_version = Some(AutopilotVersion { capabilities: v.capabilities.bits(), flight_sw_version: v.flight_sw_version, flight_custom_version: v.flight_custom_version, uid: v.uid, vendor_id: v.vendor_id, product_id: v.product_id });
                return self.handle(outs, now_ms);
            }
            MavMessage::PROTOCOL_VERSION(p) => {
                let outs = self.commands.on_message(header.component_id, MSG_PROTOCOL_VERSION);
                if outs.is_empty() {
                    return Vec::new();
                }
                self.max_proto_version = Some(p.max_version as u32);
                return self.handle(outs, now_ms);
            }
            MavMessage::COMPONENT_METADATA(m) => {
                if self.commands.on_message(header.component_id, MSG_COMPONENT_METADATA).is_empty() {
                    return Vec::new();
                }
                let uri = m.uri.to_str().unwrap_or("").to_string();
                return self.start_fetch(TYPE_GENERAL, &uri, now_ms);
            }
            MavMessage::FILE_TRANSFER_PROTOCOL(f) if mavout::for_us(f.target_system) => {
                if let Some(fetch) = self.fetch.as_mut().filter(|fetch| fetch.download.component == header.component_id) {
                    let outs = fetch.download.on_payload(&f.payload);
                    return self.follow_ftp(outs, now_ms);
                }
                let Some(job) = self.files.job.as_mut().filter(|job| job.component() == header.component_id) else { return Vec::new() };
                let steps = job.on_payload(&f.payload);
                return self.follow_files(steps, now_ms);
            }
            MavMessage::AVAILABLE_MODES(m) => {
                if self.commands.on_message(header.component_id, MSG_AVAILABLE_MODES).is_empty() {
                    return Vec::new();
                }
                let mode = AvailableMode { custom_mode: m.custom_mode, properties: m.properties.bits(), number_modes: m.number_modes, mode_index: m.mode_index, standard_mode: m.standard_mode as u8, name: m.mode_name.to_str().unwrap_or("").to_string() };
                let outs = self.modes.on_message(true, Some(&mode));
                return self.follow_modes(outs, now_ms);
            }
            MavMessage::AVAILABLE_MODES_MONITOR(m) => {
                let outs = self.modes.on_monitor(m.seq);
                return self.follow_modes(outs, now_ms);
            }
            MavMessage::PARAM_VALUE(p) => {
                let name = p.param_id.to_str().unwrap_or("").to_string();
                let decode = if self.ardupilot_components.contains(&header.component_id) { ParamValue::decode_cast } else { ParamValue::decode };
                let Some(value) = decode(p.param_type as u8, p.param_value) else { return Vec::new() };
                let actions = self.params.on_param_value(header.component_id, &name, p.param_count, p.param_index, value);
                return self.follow_params(actions, now_ms);
            }
            MavMessage::LOGGING_DATA(d) => {
                let length = (d.length as usize).min(d.data.len());
                return self.log_data(d.sequence, d.first_message_offset, &d.data[..length], now_ms);
            }
            MavMessage::LOGGING_DATA_ACKED(d) => {
                let length = (d.length as usize).min(d.data.len());
                let mut bytes = self.log_data(d.sequence, d.first_message_offset, &d.data[..length], now_ms);
                bytes.extend(self.encode(&Outbound::LoggingAck { target: (self.id, self.component), sequence: d.sequence }));
                return bytes;
            }
            MavMessage::OPEN_DRONE_ID_ARM_STATUS(a) => {
                let outs = self.remote.on_arm_status(self.id, header.system_id, header.component_id, a.status as u8, a.error.to_str().unwrap_or(""));
                self.follow_remote(outs, now_ms);
            }
            MavMessage::HOME_POSITION(h) => {
                self.home_altitude = Some(h.altitude as f64 / 1000.0);
                self.home = Some((h.latitude as f64 / 1e7, h.longitude as f64 / 1e7, h.altitude as f64 / 1000.0));
            }
            MavMessage::MISSION_COUNT(m) if mavout::for_us(m.target_system) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let outs = self.plans[kind as usize].transfer.on_count(m.count);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::MISSION_ITEM_INT(m) if mavout::for_us(m.target_system) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let scale = |v: i32| if m.frame as u8 == plantransfer::FRAME_MISSION { v as f64 } else { v as f64 * 1e-7 };
                let item = plantransfer::Item { seq: m.seq, frame: m.frame as u8, command: m.command as u32 as u16, current: m.current != 0, auto_continue: m.autocontinue != 0, params: [m.param1 as f64, m.param2 as f64, m.param3 as f64, m.param4 as f64, scale(m.x), scale(m.y), m.z as f64] };
                let outs = self.plans[kind as usize].transfer.on_item(item);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::MISSION_REQUEST_INT(m) if mavout::for_us(m.target_system) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let outs = self.plans[kind as usize].transfer.on_request(m.seq);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::MISSION_REQUEST(m) if mavout::for_us(m.target_system) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let outs = self.plans[kind as usize].transfer.on_request(m.seq);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::MISSION_ACK(m) if mavout::for_us(m.target_system) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let outs = self.plans[kind as usize].transfer.on_ack(m.mavtype as u8);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::SYS_STATUS(s) => {
                let (present, enabled, health) = (s.onboard_control_sensors_present.bits(), s.onboard_control_sensors_enabled.bits(), s.onboard_control_sensors_health.bits());
                self.sensors.update(present, enabled, health);
                if from == (self.id, self.component) {
                    self.status_bits = self.status_bits.after(present, enabled, health);
                    if self.arming_not_required() {
                        self.armed_now = enabled & SENSOR_MOTOR_OUTPUTS != 0;
                    }
                }
            }
            MavMessage::DATA_TRANSMISSION_HANDSHAKE(h) => {
                crate::flowimage::on_handshake(self.id, crate::flowimage::Handshake { kind: h.mavtype as u8, size: h.size, width: h.width, height: h.height, packets: h.packets, payload: h.payload });
            }
            MavMessage::ENCAPSULATED_DATA(d) => {
                crate::flowimage::on_data(self.id, d.seqnr, &d.data);
            }
            MavMessage::STATUSTEXT(t) => {
                let end = t.text.iter().position(|b| *b == 0).unwrap_or(t.text.len());
                let received = self.status_text.receive(header.component_id, t.severity as u8, t.id, t.chunk_seq, &t.text[..end]);
                self.chunk_due = self.status_text.has_pending().then_some(now_ms + CHUNKED_TEXT_TIMEOUT_MS);
                if !received.is_empty() {
                    let ardupilot = self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT;
                    return received.into_iter().map(|status| calibration_as_info(status, ardupilot)).flat_map(|status| self.take_status(status, now_ms)).collect();
                }
            }
            _ => {}
        }
        match message {
            MavMessage::BATTERY_STATUS(b) if !crate::cameraproto::is_camera_component(header.component_id) => self.announce_battery(b.id, b.charge_state as u8),
            MavMessage::FENCE_STATUS(f) => self.announce_fence(f.breach_status == 1, f.breach_type as u8, now_ms),
            _ => {}
        }
        if let Some((battery, home)) = self.streams_watched_ms {
            self.streams_watched_ms = Some(match message {
                MavMessage::BATTERY_STATUS(_) => (now_ms, home),
                MavMessage::HOME_POSITION(_) => (battery, now_ms),
                _ => (battery, home),
            });
        }
        self.gps.apply(message);
        self.gps2.apply_second(message);
        self.batteries.apply(message);
        if self.facts.apply(from, message) && matches!(message, MavMessage::GLOBAL_POSITION_INT(_)) {
            crate::track::observe(i64::from(self.id), self.armed(), self.facts.coordinate.map(|(latitude, longitude, _)| (latitude, longitude)));
        }
        self.wind.apply(message);
        self.setpoint.apply(message);
        self.hygrometer.apply(message);
        self.generator.apply(message);
        self.efi.apply(message);
        self.rpm.apply(message);
        if self.is_ardusub() {
            self.sub_info.apply(message);
        }
        self.escs.apply(message);
        if let MavMessage::TERRAIN_REPORT(d) = message {
            self.terrain_blocks = (d.pending, d.loaded);
        }
        self.temperature.apply(message);
        self.vibration.apply(message);
        self.radio.apply((header.system_id, header.component_id), message);
        self.obstacle.apply(message, now_ms);
        if !was_armed && self.armed_now {
            self.clear_trigger_points();
            (self.flight_started_ms, self.flight_seconds) = (Some(now_ms), 0.0);
            self.battery_announced.clear();
        }
        if was_armed && !self.armed_now {
            (self.flight_seconds, self.flight_started_ms) = (self.flight_time(now_ms), None);
            let disable_video = crate::settingsstore::raw_setting("settings.videoSettings.disableWhenDisarmed").and_then(|v| v.as_bool()).unwrap_or(false);
            if disable_video && !self.replay && !crate::qthost::present() {
                crate::settingsstore::written("Video/streamEnabled", "false");
            }
        }
        self.note_trigger_point(message);
        self.note_mission_index(message);
        if let MavMessage::ORBIT_EXECUTION_STATUS(d) = message {
            self.orbit_heard_ms = Some(now_ms);
            self.orbit_circle = Some((d.radius, d.x, d.y));
        }
        self.rc_rssi.apply(message, self.ardupilot_components.contains(&header.component_id));
        self.distance.apply(message);
        self.local.apply(message);
        self.local_setpoint.apply_target(message);
        self.estimator.apply(message);
        let event_requests = self.apply_events(from, message, now_ms);
        if let MavMessage::RC_CHANNELS(c) = message {
            let raw = [c.chan1_raw, c.chan2_raw, c.chan3_raw, c.chan4_raw, c.chan5_raw, c.chan6_raw, c.chan7_raw, c.chan8_raw, c.chan9_raw, c.chan10_raw, c.chan11_raw, c.chan12_raw, c.chan13_raw, c.chan14_raw, c.chan15_raw, c.chan16_raw, c.chan17_raw, c.chan18_raw];
            let valid = raw.iter().filter(|v| **v != u16::MAX).count();
            if raw.iter().position(|v| *v == u16::MAX).is_none_or(|at| at == valid) {
                self.rc_values = raw[..valid].to_vec();
                self.rccal.channel_values(&crate::rccal::clamped(&raw[..valid]), now_ms);
            }
        }
        if let MavMessage::SERVO_OUTPUT_RAW(s) = message {
            self.servo_outputs = crate::apmservos::outputs(&[s.servo1_raw, s.servo2_raw, s.servo3_raw, s.servo4_raw, s.servo5_raw, s.servo6_raw, s.servo7_raw, s.servo8_raw, s.servo9_raw, s.servo10_raw, s.servo11_raw, s.servo12_raw, s.servo13_raw, s.servo14_raw, s.servo15_raw, s.servo16_raw]);
        }
        if let MavMessage::SERIAL_CONTROL(d) = message {
            if d.device == mavlink::dialects::ardupilotmega::SerialControlDev::SERIAL_CONTROL_DEV_SHELL {
                if let Some(data) = d.data.get(..usize::from(d.count)) {
                    self.shell.receive(data);
                }
            }
        }
        let camera = self.note_camera(header.component_id, message, now_ms);
        let gimbal = self.note_gimbal(header.component_id, message, now_ms);
        camera.into_iter().chain(gimbal).chain(self.note_onboard_log(message, now_ms)).chain(event_requests).collect()
    }

    fn apply_events(&mut self, from: (u8, u8), message: &MavMessage, now_ms: u64) -> Vec<Vec<u8>> {
        if from != (self.id, self.component) {
            return Vec::new();
        }
        let requested = match message {
            MavMessage::EVENT(e) => {
                let incoming = crate::libevents::Incoming {
                    sequence: e.sequence,
                    timestamp_ms: e.event_time_boot_ms,
                    for_us: (e.destination_system == 0 || e.destination_system == mavout::gcs_system()) && (e.destination_component == 0 || e.destination_component == mavout::GCS_COMPONENT),
                };
                let (deliver, requested) = self.events.receiver.on_event(&incoming, now_ms);
                if deliver && let Some(delivered) = self.events.deliver(crate::libevents::Raw { id: e.id, arguments: e.arguments, log_levels: e.log_levels }, self.component) {
                    self.event_delivered(delivered);
                }
                requested
            }
            MavMessage::CURRENT_EVENT_SEQUENCE(c) => {
                let reset = c.flags.bits() & 1 != 0;
                self.events.receiver.on_current(c.sequence, reset, now_ms)
            }
            MavMessage::RESPONSE_EVENT_ERROR(r) if (r.target_system, r.target_component) == (mavout::gcs_system(), mavout::GCS_COMPONENT) => {
                self.events.checks.reset();
                self.events.receiver.on_error(r.sequence, r.sequence_oldest_available, now_ms)
            }
            MavMessage::CURRENT_MODE(m) => {
                if m.intended_custom_mode != 0 {
                    self.intended_custom_mode = m.intended_custom_mode;
                    self.refresh_arming_report();
                }
                None
            }
            MavMessage::HEARTBEAT(_) => {
                self.refresh_arming_report();
                None
            }
            _ => None,
        };
        self.event_request(requested)
    }

    fn event_request(&mut self, sequence: Option<u16>) -> Vec<Vec<u8>> {
        let target = (self.id, self.component);
        sequence.and_then(|sequence| self.encode(&Outbound::RequestEvent { target, sequence })).into_iter().collect()
    }

    fn event_delivered(&mut self, delivered: crate::libevents::Delivered) {
        match delivered {
            crate::libevents::Delivered::Checks => self.refresh_arming_report(),
            crate::libevents::Delivered::Message { severity, text } => {
                if severity <= SEVERITY_ERROR {
                    self.pending_notices.push((crate::noticeboard::VEHICLE_ERROR, text.clone()));
                }
                self.message_log.record_html(self.component, severity, text, crate::messagelog::clock_now());
            }
        }
    }

    fn refresh_arming_report(&mut self) {
        let mode = if self.intended_custom_mode != 0 { self.intended_custom_mode } else { self.custom_mode };
        let takeoff = self.custom_mode_named("Takeoff");
        let mission = self.custom_mode_named("Mission");
        self.events.refresh(self.component, mode, takeoff, mission);
    }

    fn note_gimbal(&mut self, compid: u8, message: &MavMessage, now_ms: u64) -> Vec<Vec<u8>> {
        use crate::gimbal::{DeviceAttitude, ManagerInformation, ManagerStatus};
        let outs = {
            let mut gimbals = crate::gimbal::lock();
            match message {
                MavMessage::HEARTBEAT(_) => {
                    gimbals.set_ready(self.connected);
                    gimbals.set_heading(Some(self.facts.heading as f32).filter(|h| h.is_finite()), now_ms);
                    gimbals.on_heartbeat(compid, now_ms)
                }
                MavMessage::GIMBAL_MANAGER_INFORMATION(d) => gimbals.on_manager_information(
                    ManagerInformation { compid, device_id: d.gimbal_device_id, capability_flags: d.cap_flags.bits(), limits_rad: Some([d.roll_min, d.roll_max, d.pitch_min, d.pitch_max, d.yaw_min, d.yaw_max]) },
                    now_ms,
                ),
                MavMessage::GIMBAL_MANAGER_STATUS(d) => gimbals.on_manager_status(
                    ManagerStatus { compid, device_id: d.gimbal_device_id, primary_sysid: d.primary_control_sysid, primary_compid: d.primary_control_compid, secondary_sysid: d.secondary_control_sysid, secondary_compid: d.secondary_control_compid },
                    now_ms,
                ),
                MavMessage::GIMBAL_DEVICE_ATTITUDE_STATUS(d) => gimbals.on_device_attitude_status(
                    DeviceAttitude {
                        compid,
                        device_id: d.gimbal_device_id,
                        flags: u32::from(d.flags.bits()),
                        q: d.q,
                        angular_velocity_rad_s: Some([d.angular_velocity_x, d.angular_velocity_y, d.angular_velocity_z]),
                        failure_flags: d.failure_flags.bits(),
                        delta_yaw_rad: Some(d.delta_yaw).filter(|v| v.is_finite()),
                    },
                    now_ms,
                ),
                _ => return Vec::new(),
            }
        };
        self.gimbal_outs(outs, now_ms).unwrap_or_default()
    }

    fn gimbal_outs(&mut self, outs: Vec<crate::gimbal::Out>, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        use crate::gimbal::Out;
        if outs.contains(&Out::StopRateRepeat) {
            self.gimbal_rate_due = None;
        }
        if outs.contains(&Out::StartRateRepeat) {
            self.gimbal_rate_due = Some(now_ms + GIMBAL_RATE_REPEAT_MS);
        }
        const MAV_CMD_REQUEST_MESSAGE: u16 = 512;
        const MAV_CMD_SET_MESSAGE_INTERVAL: u16 = 511;
        if let Some(reason) = outs.iter().find_map(|out| match out {
            Out::Refused { reason, .. } => Some(*reason),
            _ => None,
        }) {
            return Err(reason.to_string());
        }
        let target = self.id;
        Ok(outs
            .into_iter()
            .filter_map(|out| match out {
                Out::Command { component, command, params, .. } => Some(Outbound::CommandLong { target: (target, component), command, params }),
                Out::RequestMessage { component, message } => Some(Outbound::CommandLong { target: (target, component), command: MAV_CMD_REQUEST_MESSAGE, params: [f64::from(message), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] }),
                Out::MessageInterval { component, message, interval_us } => Some(Outbound::CommandLong { target: (target, component), command: MAV_CMD_SET_MESSAGE_INTERVAL, params: [f64::from(message), interval_us, 0.0, 0.0, 0.0, 0.0, 0.0] }),
                _ => None,
            })
            .filter_map(|send| self.encode(&send))
            .collect())
    }

    fn gimbal_action(&mut self, action: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        let outs = {
            let mut gimbals = crate::gimbal::lock();
            let flag = |key: &str| action.get(key).and_then(Value::as_bool).unwrap_or(false);
            match action.get("op").and_then(Value::as_str).unwrap_or("") {
                "center" => gimbals.center(now_ms),
                "tilt90" => gimbals.send_pitch_body_yaw(-90.0, 0.0, true, now_ms),
                "retract" => gimbals.set_retract(true, now_ms),
                "yawLock" => gimbals.set_yaw_lock(flag("lock"), now_ms),
                "rates" => {
                    let rate = |key: &str| action.get(key).and_then(Value::as_f64).map(|v| v as f32);
                    gimbals.set_rates(rate("pitch"), rate("yaw"), now_ms)
                }
                "acquire" => gimbals.acquire_control(),
                "release" => gimbals.release_control(),
                "onScreen" => {
                    let number = |key: &str| action.get(key).and_then(Value::as_f64).map(|v| v as f32);
                    let screen = match flag("point") {
                        true => crate::gimbal::Screen::Point { h_fov: number("hFov").unwrap_or(0.0), v_fov: number("vFov").unwrap_or(0.0) },
                        false => crate::gimbal::Screen::Drag { slide_speed: number("slide").unwrap_or(0.0) },
                    };
                    gimbals.on_screen_control(number("pan").unwrap_or(0.0), number("tilt").unwrap_or(0.0), screen, now_ms)
                }
                "select" => {
                    let number = |key: &str| action.get(key).and_then(Value::as_u64).and_then(|v| u8::try_from(v).ok());
                    match number("managerCompid").zip(number("deviceId")) {
                        Some((manager_compid, device_id)) => gimbals.set_active(crate::gimbal::PairId { manager_compid, device_id }),
                        None => return Err("Name the gimbal by its manager component and device id.".to_string()),
                    }
                }
                other => return Err(format!("Unknown gimbal action {other:?}")),
            }
        };
        self.gimbal_outs(outs, now_ms)
    }

    pub fn snapshot(&self) -> Value {
        let firmware = self.firmware().map(|f| json!({ "version": f.version.map(|(a, b, c, d)| format!("{a}.{b}.{c} ({d})")), "gitHash": f.git_hash }));
        let flight_modes: Vec<Value> = self.flight_modes.iter().map(|m| json!({ "name": m.name, "customMode": m.custom_mode, "standardMode": m.standard_mode, "canBeSet": m.can_be_set, "advanced": m.advanced })).collect();
        let parameters = json!({ "ready": self.params.ready(), "progress": self.params_progress, "count": self.params.names(self.component).len() });
        json!({
            "id": self.id,
            "component": self.component,
            "link": self.link,
            "autopilot": self.autopilot,
            "vehicleType": self.vehicle_type,
            "armed": self.armed(),
            "customMode": self.custom_mode,
            "flightMode": self.flight_mode(),
            "baseMode": self.base_mode,
            "systemStatus": self.system_status,
            "heartbeats": self.heartbeats,
            "messagesReceived": self.messages,
            "lastHeartbeatUs": self.last_heartbeat_us,
            "connectionLost": self.connection_lost,
            "gps": { "latitude": self.gps.latitude, "longitude": self.gps.longitude, "hdop": self.gps.hdop, "vdop": self.gps.vdop, "courseOverGround": self.gps.course_over_ground, "lock": self.gps.lock, "count": self.gps.count },
            "batteries": self.batteries.by_id.iter().map(|(id, b)| json!({ "id": id, "voltage": b.voltage, "current": b.current, "percentRemaining": b.percent_remaining, "mahConsumed": b.mah_consumed, "temperature": b.temperature, "instantPower": b.instant_power })).collect::<Vec<_>>(),
            "attitude": { "roll": self.facts.roll, "pitch": self.facts.pitch, "heading": self.facts.heading, "rollRate": self.facts.roll_rate, "pitchRate": self.facts.pitch_rate, "yawRate": self.facts.yaw_rate },
            "coordinate": self.facts.coordinate.map(|(lat, lon, alt)| json!({ "latitude": lat, "longitude": lon, "altitude": alt })),
            "altitudeRelative": self.facts.altitude_relative,
            "altitudeAMSL": self.facts.altitude_amsl,
            "airSpeed": self.facts.air_speed,
            "groundSpeed": self.facts.ground_speed,
            "climbRate": self.facts.climb_rate,
            "throttlePct": self.facts.throttle_pct,
            "distanceToNextWP": self.facts.distance_to_next_wp,
            "rangeFinderDist": self.facts.range_finder_dist,
            "wind": { "direction": self.wind.direction, "speed": self.wind.speed, "verticalSpeed": self.wind.vertical_speed },
            "temperature": { "temperature1": self.temperature.temperature1, "temperature2": self.temperature.temperature2, "temperature3": self.temperature.temperature3 },
            "localPosition": { "x": self.local.x, "y": self.local.y, "z": self.local.z, "vx": self.local.vx, "vy": self.local.vy, "vz": self.local.vz },
            "unhealthySensors": self.sensors.unhealthy_bits(),
            "initialConnectComplete": self.connected,
            "connectProgress": self.connect_progress,
            "connectStep": self.connect.current().map(|s| format!("{s:?}")),
            "capabilities": self.capabilities,
            "maxProtoVersion": self.max_proto_version,
            "firmware": firmware,
            "flightModes": flight_modes,
            "parameters": parameters,
            "mission": self.plan_state(PLAN_MISSION),
            "fence": self.plan_state(PLAN_FENCE),
            "rally": self.plan_state(PLAN_RALLY),
            "home": self.home.map(|(lat, lon, alt)| json!({ "latitude": lat, "longitude": lon, "altitude": alt })),
            "remoteId": self.remote_snapshot(),
            "log": self.log_snapshot(),
            "componentInformation": { "types": self.metadata_types.keys().collect::<Vec<_>>(), "parameterMetadata": self.parameter_metadata.as_ref().map(|p| p.named.len() + p.indexed.len()).unwrap_or(0) },
            "messages": self.recent.iter().rev().take(50).map(|m| json!({ "component": m.component, "severity": m.severity, "text": m.text })).collect::<Vec<_>>(),
            "byName": self.by_name,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkKinds {
    pub cloud: Vec<LinkId>,
    pub high_latency: Vec<LinkId>,
    pub usb_direct: Vec<LinkId>,
}

const SENSOR_REFRESH_DELAY_MS: u64 = 1000;
const REBOOT_TAG: u64 = 0x5245_424F_4F54;
const FACTORY_RESET_TAG: u64 = 0x5245_5345_5446;
const STORAGE_RESET_FACTORY: f64 = 3.0;
const STORAGE_MISSION_UNTOUCHED: f64 = -1.0;

const GIMBAL_RATE_REPEAT_MS: u64 = 500;

fn sensor_parameter(name: &str) -> bool {
    name.starts_with("CAL_") || name.starts_with("SENS_")
}

fn heartbeat_info(message: &MavMessage) -> Option<(u8, u8)> {
    match message {
        MavMessage::HEARTBEAT(h) => Some((h.mavtype as u8, h.autopilot as u8)),
        MavMessage::HIGH_LATENCY(_) => Some((TYPE_GENERIC, AUTOPILOT_GENERIC)),
        MavMessage::HIGH_LATENCY2(h) => Some((h.mavtype as u8, h.autopilot as u8)),
        _ => None,
    }
}

const HL_FAILURE_TO_SENSOR: [(u16, u32); 6] = [(1, 32), (2, 16), (4, 8), (8, 2), (16, 1), (32, 4)];

fn high_latency_sensors(failure_flags: u16) -> u32 {
    HL_FAILURE_TO_SENSOR.iter().filter(|(failure, _)| failure_flags & failure != 0).fold(0, |sensors, (_, sensor)| sensors | sensor)
}

fn high_latency_custom_mode(autopilot: u8, mode: u16) -> u32 {
    match autopilot {
        crate::modes::AUTOPILOT_PX4 => u32::from(mode) << 16,
        _ => u32::from(mode),
    }
}

const APM_CALIBRATION_PROMPTS: [&str; 2] = ["Place vehicle", "Calibration successful"];
const SEVERITY_INFO: u8 = 6;

fn calibration_as_info(status: crate::statustext::StatusText, ardupilot: bool) -> crate::statustext::StatusText {
    match ardupilot && APM_CALIBRATION_PROMPTS.iter().any(|prompt| status.text.contains(prompt)) {
        true => crate::statustext::StatusText { severity: SEVERITY_INFO, ..status },
        false => status,
    }
}

#[derive(Debug, Default)]
pub struct Hub {
    vehicles: BTreeMap<u8, Vehicle>,
    link_kinds: LinkKinds,
    arrival: Vec<u8>,
    active: Option<u8>,
    host_selects: bool,
    listed: Option<Vec<u8>>,
    selected: Vec<u8>,
    remote_inputs: Option<RemoteInputs>,
    log_inputs: LogInputs,
    rtcm: crate::rtcm::Fragmenter,
    link_counts: BTreeMap<LinkId, crate::linkcount::LinkCount>,
    v1_links: BTreeMap<LinkId, V1Watch>,
}

#[derive(Debug, Default, Clone, Copy)]
struct V1Watch {
    first_seen_ms: Option<u64>,
    v2_seen: bool,
    reported: bool,
    due: bool,
}

const MAVLINK_V1_GRACE_MS: u64 = 10_000;

pub fn v1_dropped(v2: bool, message: &MavMessage) -> bool {
    !v2 && !matches!(message, MavMessage::HEARTBEAT(_) | MavMessage::RADIO_STATUS(_))
}

pub fn mavlink_v1_notice(link: &str, application: &str) -> String {
    let application = if application.is_empty() { "QGroundControl" } else { application };
    format!("MAVLink v1 traffic detected on link '{link}'. {application} only supports MAVLink v2. Please ensure your vehicle is configured to use MAVLink v2.")
}

fn v1_watched(watch: V1Watch, v2: bool, now_ms: u64) -> V1Watch {
    match (v2, watch.v2_seen || watch.reported) {
        (true, _) => V1Watch { v2_seen: true, ..watch },
        (false, true) => watch,
        (false, false) => {
            let first = watch.first_seen_ms.unwrap_or(now_ms);
            let due = now_ms.saturating_sub(first) >= MAVLINK_V1_GRACE_MS;
            V1Watch { first_seen_ms: Some(first), reported: due, due, ..watch }
        }
    }
}

pub static HUB: LazyLock<Mutex<Hub>> = LazyLock::new(|| Mutex::new(Hub::default()));

impl Hub {
    pub fn inject_rtcm(&mut self, rtcm: &[u8]) -> Vec<(LinkId, Vec<u8>)> {
        let packets = self.rtcm.fragments(rtcm);
        let links: std::collections::BTreeSet<LinkId> = self.vehicles.values().filter(|v| !v.replay).map(|v| v.link).collect();
        links.iter().flat_map(|link| packets.iter().filter_map(|data| mavout::encode_next(&Outbound::GpsRtcmData { data: data.clone() }).map(|bytes| (*link, bytes)))).collect()
    }

    pub fn on_frame(&mut self, origin: Origin, header: &MavHeader, message: &MavMessage, timestamp_us: u64, now_ms: u64) -> Vec<(LinkId, Vec<u8>)> {
        if origin.v2 || v1_dropped(origin.v2, message) {
            let watched = v1_watched(self.v1_links.get(&origin.link).copied().unwrap_or_default(), origin.v2, now_ms);
            self.v1_links.insert(origin.link, watched);
        }
        if v1_dropped(origin.v2, message) {
            return Vec::new();
        }
        if origin.v2 {
            let counted = self.link_counts.get(&origin.link).cloned().unwrap_or_default().counted(header.system_id, header.component_id, header.sequence);
            if let (Some(status), Some(vehicle)) = (counted.status(), self.vehicles.get_mut(&header.system_id).filter(|v| !v.is_standby(origin.link))) {
                vehicle.link_status = status;
            }
            self.link_counts.insert(origin.link, counted);
        }
        let mut bytes = Vec::new();
        crate::adsb::on_message(message, now_ms);
        if let Some((kind, autopilot)) = heartbeat_info(message) {
            let excluded_type = matches!(kind, TYPE_GCS | TYPE_ONBOARD_CONTROLLER | TYPE_GIMBAL | TYPE_ADSB);
            if header.component_id == COMP_AUTOPILOT1 && !excluded_type && autopilot != AUTOPILOT_INVALID && header.system_id != 0 && !self.vehicles.contains_key(&header.system_id) {
                let mut vehicle = Vehicle::new(header.system_id, header.component_id, autopilot, kind, origin.link, origin.replay);
                vehicle.link_kinds = self.link_kinds.clone();
                vehicle.commands.high_latency = self.link_kinds.high_latency.contains(&origin.link);
                if kind == TYPE_SUBMARINE && !origin.replay && !crate::qthost::present() {
                    sub_video_defaults();
                }
                if origin.v2 {
                    vehicle.max_proto_version = Some(PROTO_MAVLINK2);
                }
                bytes.extend(vehicle.begin_connect(now_ms));
                if header.system_id == crate::mavout::gcs_system() {
                    crate::noticeboard::post_from_vehicle(crate::noticeboard::MESSAGE, &format!("Warning: A vehicle is using the same system id as {}: {}", crate::noticeboard::application_name(), header.system_id));
                }
                self.vehicles.insert(header.system_id, vehicle);
                self.arrival.push(header.system_id);
                if self.vehicles.len() > 1 {
                    crate::noticeboard::post_from_vehicle(crate::noticeboard::MESSAGE, &format!("Connected to Vehicle {}", header.system_id));
                }
                if !self.host_selects {
                    self.active.get_or_insert(header.system_id);
                }
            }
        }
        if let MavMessage::RADIO_STATUS(_) = message {
            self.vehicles.values_mut().filter(|v| v.link == origin.link && v.id != header.system_id).for_each(|v| v.radio.apply((header.system_id, header.component_id), message));
        }
        let inputs = self.remote_inputs.as_ref();
        let Some(vehicle) = self.vehicles.get_mut(&header.system_id) else { return Vec::new() };
        if !matches!(message, MavMessage::RADIO_STATUS(_)) {
            vehicle.note_link(origin.link, now_ms);
        }
        if vehicle.is_standby(origin.link) {
            let link = vehicle.link;
            return bytes.into_iter().map(|bytes| (link, bytes)).collect();
        }
        if origin.v2 {
            vehicle.max_proto_version = Some(PROTO_MAVLINK2);
        }
        bytes.extend(vehicle.apply(header, message, timestamp_us, now_ms));
        let armed = vehicle.armed();
        let auto = self.log_inputs.auto_start && vehicle.autopilot == crate::modes::AUTOPILOT_PX4;
        match (auto, vehicle.was_armed, armed) {
            (true, false, true) if vehicle.log.is_none() => match vehicle.start_log(&self.log_inputs, now_ms) {
                Ok(frames) => bytes.extend(frames),
                Err(reason) => vehicle.log_error = Some(reason),
            },
            (true, true, false) if vehicle.log.is_some() => bytes.extend(vehicle.stop_log(now_ms).unwrap_or_default()),
            _ => {}
        }
        vehicle.was_armed = armed;
        let link = vehicle.link;
        bytes.extend(vehicle.pump_with(now_ms, inputs, timestamp_us / 1_000_000));
        match vehicle.replay {
            true => Vec::new(),
            false => bytes.into_iter().map(|bytes| (link, bytes)).collect(),
        }
    }

    pub fn heard_on(&self, link: LinkId) -> bool {
        self.vehicles.values().any(|v| v.link == link || v.link_states.iter().any(|(id, _, _)| *id == link))
    }

    pub fn link_closed(&mut self, link: LinkId) {
        crate::signing::lock().closed(link);
        self.keep_vehicles_on(|candidate| candidate != link);
    }

    pub fn retain_links(&mut self, open: &[LinkId]) {
        self.link_counts.retain(|link, _| open.contains(link));
        self.v1_links.retain(|link, _| open.contains(link));
        self.keep_vehicles_on(|candidate| open.contains(&candidate));
    }

    fn keep_vehicles_on(&mut self, still_open: impl Fn(LinkId) -> bool) {
        self.vehicles.values_mut().for_each(|v| {
            v.link_states.retain(|(link, _, _)| still_open(*link));
            let _ = v.update_primary_link();
            if !still_open(v.link) {
                if let Some(next) = v.primary_link.or_else(|| v.link_states.first().map(|(link, _, _)| *link)) {
                    v.link = next;
                }
            }
        });
        let gone: Vec<u8> = self.vehicles.values().filter(|v| !still_open(v.link)).map(|v| v.id).collect();
        gone.iter().for_each(|id| self.remove(*id));
    }

    pub fn take_v1_reports(&mut self) -> Vec<LinkId> {
        let due: Vec<LinkId> = self.v1_links.iter().filter(|(_, w)| w.due).map(|(link, _)| *link).collect();
        due.iter().for_each(|link| {
            if let Some(watch) = self.v1_links.get_mut(link) {
                watch.due = false;
            }
        });
        due
    }

    pub fn take_notices(&mut self) -> Vec<(&'static str, String)> {
        let prefixed = self.fleet_count() > 1;
        self.vehicles
            .values_mut()
            .flat_map(|v| {
                let id = v.id;
                std::mem::take(&mut v.pending_notices).into_iter().map(move |(kind, text)| match (kind, prefixed) {
                    (crate::noticeboard::VEHICLE_ERROR, true) => (kind, format!("Vehicle {id}: {text}")),
                    _ => (kind, text),
                })
            })
            .collect()
    }

    pub fn tick(&mut self, now_ms: u64) -> Vec<(LinkId, Vec<u8>)> {
        self.tick_with(now_ms, 0)
    }

    pub fn tick_with(&mut self, now_ms: u64, now_s: u64) -> Vec<(LinkId, Vec<u8>)> {
        let count = self.vehicles.len();
        self.vehicles.values_mut().filter(|vehicle| !vehicle.replay).for_each(|vehicle| {
            let prefix = crate::speech::vehicle_prefix(vehicle.id, count);
            vehicle.announce(&prefix);
        });
        let inputs = self.remote_inputs.as_ref();
        self.vehicles.values_mut().flat_map(|vehicle| {
            let (link, replay) = (vehicle.link, vehicle.replay);
            let switching = std::mem::take(&mut vehicle.link_frames);
            switching.into_iter().chain(vehicle.pump_with(now_ms, inputs, now_s).into_iter().map(move |bytes| (link, bytes))).filter(move |_| !replay).collect::<Vec<_>>()
        }).collect()
    }

    pub fn gcs_moved(&mut self, latitude: f64, longitude: f64, altitude: f64, now_ms: u64) -> Vec<(LinkId, Vec<u8>)> {
        let enabled = crate::settingsstore::raw_setting("settings.flyViewSettings.updateHomePosition").and_then(|v| v.as_bool()).unwrap_or(false);
        self.home_follows_gcs(enabled, (latitude, longitude, altitude), now_ms)
    }

    fn home_follows_gcs(&mut self, enabled: bool, (latitude, longitude, altitude): (f64, f64, f64), now_ms: u64) -> Vec<(LinkId, Vec<u8>)> {
        if !enabled {
            return Vec::new();
        }
        self.vehicles
            .values_mut()
            .filter(|vehicle| vehicle.facts.coordinate.is_some() && matches!(vehicle.autopilot, crate::modes::AUTOPILOT_PX4 | crate::modes::AUTOPILOT_ARDUPILOT) && !vehicle.replay)
            .flat_map(|vehicle| {
                let params = [0.0, 0.0, 0.0, 0.0, latitude, longitude, altitude];
                let outs = vehicle.commands.send(Command { component: vehicle.component, command: guidedcmd::CMD_DO_SET_HOME, command_int: false, frame: 0, params, show_error: false, tag: 0 }, now_ms);
                let link = vehicle.link;
                vehicle.handle(outs, now_ms).into_iter().map(move |bytes| (link, bytes))
            })
            .collect()
    }

    pub fn set_remote_inputs(&mut self, settings: remoteid::Settings, fix: GcsFix, now_ms: u64) {
        self.remote_inputs = Some(RemoteInputs { settings, fix, pushed_ms: now_ms });
    }

    pub fn log_request(&mut self, id: Option<u8>, request: &Value, now_ms: u64) -> Result<Vec<(LinkId, Vec<u8>)>, String> {
        if let Some(path) = request.get("path").and_then(Value::as_str) {
            self.log_inputs.path = path.to_string();
        }
        if let Some(extension) = request.get("extension").and_then(Value::as_str) {
            self.log_inputs.extension = format!(".{}", extension.trim_start_matches('.'));
        }
        if let Some(auto) = request.get("autoStart").and_then(Value::as_bool) {
            self.log_inputs.auto_start = auto;
        }
        let Some(action) = request.get("action").and_then(Value::as_str) else { return Ok(Vec::new()) };
        let chosen = id.or(self.active).ok_or_else(|| "No vehicle is connected through the core.".to_string())?;
        let inputs = self.log_inputs.clone();
        let vehicle = self.vehicles.get_mut(&chosen).ok_or_else(|| format!("Vehicle {chosen} is not connected through the core."))?;
        let link = vehicle.link;
        let frames = match action {
            "start" => vehicle.start_log(&inputs, now_ms)?,
            "stop" => vehicle.stop_log(now_ms)?,
            other => return Err(format!("Unknown log action {other:?}")),
        };
        Ok(frames.into_iter().map(|bytes| (link, bytes)).collect())
    }

    pub fn calibrate_request(&mut self, id: Option<u8>, request: &Value, now_ms: u64) -> Result<Vec<(LinkId, Vec<u8>)>, String> {
        let chosen = id.or(self.active).ok_or_else(|| "No vehicle is connected through the core.".to_string())?;
        let vehicle = self.vehicles.get_mut(&chosen).ok_or_else(|| format!("Vehicle {chosen} is not connected through the core."))?;
        let link = vehicle.link;
        Ok(vehicle.calibrate_request(request, now_ms)?.into_iter().map(|bytes| (link, bytes)).collect())
    }

    pub fn calibration_snapshot(&self, id: Option<u8>) -> Value {
        let vehicle = match id { Some(id) => self.vehicles.get(&id), None => self.active() };
        json!({
            "kind": "object",
            "class": "CoreCalibration",
            "available": vehicle.is_some(),
            "vehicleId": vehicle.map(|v| v.id),
            "calibration": vehicle.map(Vehicle::calibration_snapshot).unwrap_or(Value::Null),
        })
    }

    pub fn remote_request(&mut self, id: Option<u8>, request: &Value) -> Result<(), String> {
        let chosen = id.or(self.active).ok_or_else(|| "No vehicle is connected through the core.".to_string())?;
        let vehicle = self.vehicles.get_mut(&chosen).ok_or_else(|| format!("Vehicle {chosen} is not connected through the core."))?;
        let declare = request.get("emergency").and_then(Value::as_bool).ok_or_else(|| "A remote ID request needs an emergency flag.".to_string())?;
        vehicle.remote.set_emergency(declare);
        Ok(())
    }

    pub fn remote_snapshot(&self, id: Option<u8>) -> Value {
        let vehicle = match id { Some(id) => self.vehicles.get(&id), None => self.active() };
        json!({
            "kind": "object",
            "class": "CoreRemoteId",
            "available": vehicle.is_some(),
            "vehicleId": vehicle.map(|v| v.id),
            "inputs": self.remote_inputs.as_ref().map(|i| json!({ "region": i.settings.region, "locationType": i.settings.location_type, "gcsFixValid": i.fix.valid, "gcsFixAgeMs": i.fix.age_ms.saturating_add(now_ms().saturating_sub(i.pushed_ms)) })),
            "remoteId": vehicle.map(Vehicle::remote_snapshot).unwrap_or(Value::Null),
        })
    }

    pub fn guided(&mut self, id: Option<u8>, action: &Value, now_ms: u64) -> Result<Vec<(LinkId, Vec<u8>)>, String> {
        let chosen = id.or(self.active).ok_or_else(|| "No vehicle is connected through the core.".to_string())?;
        let vehicle = self.vehicles.get_mut(&chosen).ok_or_else(|| format!("Vehicle {chosen} is not connected through the core."))?;
        let link = vehicle.link;
        vehicle.start_guided(action, now_ms).map(|frames| frames.into_iter().map(|bytes| (link, bytes)).collect())
    }

    pub fn parameter_request(&mut self, id: Option<u8>, request: &Value, now_ms: u64) -> Result<Vec<(LinkId, Vec<u8>)>, String> {
        let chosen = id.or(self.active).ok_or_else(|| "No vehicle is connected through the core.".to_string())?;
        let vehicle = self.vehicles.get_mut(&chosen).ok_or_else(|| format!("Vehicle {chosen} is not connected through the core."))?;
        let link = vehicle.link;
        vehicle.parameter_request(request, now_ms).map(|frames| frames.into_iter().map(|bytes| (link, bytes)).collect())
    }

    pub fn write_mission(&mut self, id: Option<u8>, items: Vec<plantransfer::Item>, now_ms: u64) -> Result<Vec<(LinkId, Vec<u8>)>, String> {
        let chosen = id.or(self.active).ok_or_else(|| "No vehicle is connected through the core.".to_string())?;
        let vehicle = self.vehicles.get_mut(&chosen).ok_or_else(|| format!("Vehicle {chosen} is not connected through the core."))?;
        if vehicle.commands.high_latency {
            return Err(HIGH_LATENCY_UPLOAD.to_string());
        }
        let link = vehicle.link;
        vehicle.write_mission(items, now_ms).map(|frames| frames.into_iter().map(|bytes| (link, bytes)).collect())
    }

    pub fn mission_request(&mut self, id: Option<u8>, request: &Value, now_ms: u64) -> Result<Vec<(LinkId, Vec<u8>)>, String> {
        let chosen = id.or(self.active).ok_or_else(|| "No vehicle is connected through the core.".to_string())?;
        let vehicle = self.vehicles.get_mut(&chosen).ok_or_else(|| format!("Vehicle {chosen} is not connected through the core."))?;
        let link = vehicle.link;
        vehicle.mission_request(request, now_ms).map(|frames| frames.into_iter().map(|bytes| (link, bytes)).collect())
    }

    pub fn carries(&self, id: u8) -> bool {
        self.vehicles.contains_key(&id)
    }

    pub fn guided_snapshot(&self, id: Option<u8>) -> Value {
        let chosen = match id { Some(id) => self.vehicles.get(&id), None => self.active() };
        json!({
            "kind": "object",
            "class": "CoreGuided",
            "available": chosen.is_some(),
            "vehicleId": chosen.map(|v| v.id),
            "guided": chosen.map(Vehicle::guided_snapshot).unwrap_or(Value::Null),
        })
    }

    pub fn set_link_flag(&mut self, flag: &str, on: bool) -> bool {
        self.active.and_then(|id| self.vehicles.get_mut(&id)).map(|v| match flag {
            "communicationLostEnabled" => v.comm_lost_enabled = on,
            _ => v.auto_disconnect = on,
        }).is_some()
    }

    pub fn sensors_json(&self) -> Option<Value> {
        let vehicle = self.active()?;
        let snapshot = vehicle.calibration_snapshot();
        let parameter = |name: &str| vehicle.parameter(vehicle.component, name).map(|p| p.as_f64());
        let needs = match vehicle.autopilot {
            crate::modes::AUTOPILOT_ARDUPILOT => crate::sensorcal::apm_setup_needs(&parameter),
            crate::modes::AUTOPILOT_PX4 => {
                use crate::cmdinfo::VehicleClass::{FixedWing, Vtol};
                let class = crate::plandoc::vehicle_class(i64::from(vehicle.vehicle_type));
                crate::sensorcal::px4_setup_needs(&parameter, matches!(class, FixedWing | Vtol) || vehicle.vehicle_type == MAV_TYPE_AIRSHIP)
            }
            _ => return None,
        };
        Some(crate::sensorcal::qt_shape(&snapshot, &needs))
    }

    pub fn radio_json(&mut self) -> Option<Value> {
        let vehicle = self.active.and_then(|id| self.vehicles.get_mut(&id))?;
        vehicle.load_rccal();
        Some(vehicle.rccal.json())
    }

    pub fn set_transmitter_mode(&mut self, mode: i64) -> bool {
        self.active.and_then(|id| self.vehicles.get_mut(&id)).map(|v| v.rccal.transmitter_mode = if (1..=4).contains(&mode) { mode as i32 } else { 2 }).is_some()
    }

    pub fn set_centered_throttle(&mut self, centered: bool) -> bool {
        self.active.and_then(|id| self.vehicles.get_mut(&id)).map(|v| v.rccal.set_centered_throttle(centered)).is_some()
    }

    pub fn reset_message_log(&mut self) {
        if let Some(vehicle) = self.active.and_then(|id| self.vehicles.get_mut(&id)) {
            vehicle.message_log.reset_all();
        }
    }

    pub fn clear_message_log(&mut self) {
        if let Some(vehicle) = self.active.and_then(|id| self.vehicles.get_mut(&id)) {
            vehicle.message_log.clear();
        }
    }

    pub fn with_onboard_logs<T>(&mut self, change: impl FnOnce(&mut crate::onboardlogs::OnboardLogs) -> T) -> Option<T> {
        self.active.and_then(|id| self.vehicles.get_mut(&id)).map(|v| change(&mut v.onboard_logs))
    }

    pub fn set_camera_tracking(&mut self, on: bool) -> bool {
        self.active.and_then(|id| self.vehicles.get_mut(&id)).map(|v| v.camera_tracking_enabled = on).is_some()
    }

    pub fn select_camera(&mut self, index: usize) -> bool {
        self.active.and_then(|id| self.vehicles.get_mut(&id)).and_then(|v| v.cameras.compid_at(index).map(|compid| v.cameras.select(compid).is_ok())).unwrap_or(false)
    }

    pub fn set_check_list_state(&mut self, state: i64) -> bool {
        self.active.and_then(|id| self.vehicles.get_mut(&id)).map(|v| v.check_list_state = state).is_some()
    }

    pub fn active_id(&self) -> Option<u8> {
        self.active
    }

    pub fn take_parameters_announce(&mut self) -> Option<(bool, bool)> {
        let vehicle = self.active.and_then(|id| self.vehicles.get_mut(&id)).filter(|v| v.parameters_announce_due)?;
        vehicle.parameters_announce_due = false;
        let hitl = vehicle.parameter(vehicle.component, "SYS_HITL").is_some_and(|p| p.as_f64() != 0.0);
        Some((vehicle.autopilot == crate::modes::AUTOPILOT_PX4, hitl))
    }

    pub fn active(&self) -> Option<&Vehicle> {
        self.active.and_then(|id| self.vehicles.get(&id))
    }

    pub fn note_cellular(&mut self, system: u8, message: &MavMessage, raw: &[u8]) {
        if let (MavMessage::CELLULAR_STATUS(cellular), Some(vehicle)) = (message, self.vehicles.get_mut(&system)) {
            vehicle.aircast.apply(cellular, crate::vehiclefact::cellular_rx_rate(raw));
        }
    }

    pub fn check_links(&mut self, now_ms: u64, kinds: &LinkKinds) {
        self.link_kinds = kinds.clone();
        self.vehicles.values_mut().for_each(|v| {
            v.link_kinds = kinds.clone();
            v.check_links(now_ms);
        });
        let closed: Vec<u8> = self.vehicles.values().filter(|v| v.connection_lost && v.auto_disconnect).map(|v| v.id).collect();
        closed.iter().for_each(|id| self.remove(*id));
    }

    pub fn links_of_other_vehicles(&self, id: u8) -> Vec<LinkId> {
        self.vehicles.values().filter(|v| v.id != id).flat_map(|v| v.link_states.iter().map(|(link, _, _)| *link)).collect()
    }

    pub fn remove(&mut self, id: u8) {
        self.vehicles.remove(&id);
        self.arrival.retain(|known| *known != id);
        self.selected.retain(|known| *known != id);
        if self.active == Some(id) && !self.host_selects {
            self.active = self.arrival.first().copied();
        }
    }

    pub fn set_active(&mut self, id: Option<u8>) {
        self.host_selects = true;
        if self.active != id {
            if let Some(vehicle) = id.and_then(|id| self.vehicles.get_mut(&id)) {
                vehicle.shell = crate::shell::Shell::default();
            }
        }
        self.active = id;
    }

    pub fn set_listed(&mut self, ids: Vec<u8>) {
        self.listed = Some(ids);
    }

    pub fn set_selected(&mut self, ids: Vec<u8>) {
        self.selected = ids;
    }

    pub fn select_vehicle(&mut self, id: u8) {
        if self.vehicles.contains_key(&id) && !self.selected.contains(&id) {
            self.selected.push(id);
        }
    }

    pub fn deselect_vehicle(&mut self, id: u8) {
        self.selected.retain(|known| *known != id);
    }

    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    pub fn selected_member(&self, index: usize) -> Option<&Vehicle> {
        self.selected.get(index).and_then(|id| self.vehicles.get(id))
    }

    pub fn fleet_count(&self) -> usize {
        self.listed.as_ref().unwrap_or(&self.arrival).len()
    }

    pub fn listed_count(&self) -> Option<usize> {
        self.listed.as_ref().map(Vec::len)
    }

    pub fn listed(&self, index: usize) -> Option<&Vehicle> {
        self.listed.as_ref().unwrap_or(&self.arrival).get(index).and_then(|id| self.vehicles.get(id))
    }

    pub fn on_extra(&mut self, header: &MavHeader, msgid: u32, payload: &[u8]) {
        let Some(vehicle) = self.vehicles.get_mut(&header.system_id) else { return };
        match (msgid, payload.first(), payload.get(1)) {
            (crate::operatorcontrol::CONTROL_STATUS, Some(flags), Some(main)) => vehicle.control.on_status(*flags, *main),
            (crate::gpsfacts::GNSS_INTEGRITY, _, _) => {
                let (receiver, integrity) = crate::gpsfacts::integrity_of(payload);
                let target = match receiver {
                    0 => Some(&mut vehicle.gps),
                    1 => Some(&mut vehicle.gps2),
                    _ => None,
                };
                if let Some(gps) = target {
                    gps.integrity = integrity;
                    vehicle.integrity_heard_ms = Some(now_ms());
                }
            }
            (COMMAND_ACK_ID, _, _) => {
                let command = payload.get(0..2).and_then(|b| b.try_into().ok()).map_or(0, u16::from_le_bytes);
                let result = payload.get(2).copied().unwrap_or(0);
                if command == crate::operatorcontrol::REQUEST_OPERATOR_CONTROL {
                    let outs = vehicle.commands.on_ack(header.component_id, command, result, now_ms());
                    vehicle.handle(outs, now_ms());
                }
            }
            (COMMAND_LONG_ID, _, _) => {
                let param = |i: usize| payload.get(i * 4..i * 4 + 4).and_then(|b| b.try_into().ok()).map_or(0.0, f32::from_le_bytes);
                let command = payload.get(28..30).and_then(|b| b.try_into().ok()).map_or(0, u16::from_le_bytes);
                let target = payload.get(30).copied().unwrap_or(0);
                if command == crate::operatorcontrol::REQUEST_OPERATOR_CONTROL && target == mavout::gcs_system() {
                    let default_secs = crate::settingsstore::raw_setting("settings.flyViewSettings.requestControlTimeout").and_then(|v| v.as_i64()).unwrap_or(crate::operatorcontrol::DEFAULT_REQUEST_TIMEOUT_SECS);
                    vehicle.control.on_request(param(0) as u8, param(2) != 0.0, param(3), default_secs, now_ms());
                }
            }
            _ => {}
        }
    }

    pub fn any_pending_parameter_writes(&self) -> bool {
        self.vehicles.values().any(Vehicle::pending_parameter_writes)
    }

    pub fn vehicle_ids(&self) -> Vec<u8> {
        self.arrival.clone()
    }

    pub fn in_arrival_order(&self) -> Vec<&Vehicle> {
        self.arrival.iter().filter_map(|id| self.vehicles.get(id)).collect()
    }

    pub fn snapshot(&self) -> Value {
        self.snapshot_of(None)
    }

    pub fn snapshot_of(&self, id: Option<u8>) -> Value {
        let chosen = match id {
            Some(id) => self.vehicles.get(&id),
            None => self.active(),
        };
        json!({
            "kind": "object",
            "class": "CoreVehicle",
            "available": chosen.is_some(),
            "heard": chosen.map(|vehicle| !vehicle.connection_lost),
            "vehicleIds": self.vehicles.keys().collect::<Vec<_>>(),
            "vehicle": chosen.map(Vehicle::snapshot).unwrap_or(Value::Null),
        })
    }
}

pub fn now_us() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_micros() as u64).unwrap_or(0)
}

static STARTED: LazyLock<std::time::Instant> = LazyLock::new(std::time::Instant::now);

pub fn skips_download(autopilot: u8, armed: bool, skip_when_flying: bool) -> bool {
    skip_when_flying && armed && autopilot != crate::modes::AUTOPILOT_PX4
}

pub fn now_ms() -> u64 {
    STARTED.elapsed().as_millis() as u64
}

pub fn lock() -> MutexGuard<'static, Hub> {
    HUB.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn core_vehicle_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let hub = lock();
    hub.snapshot_of(args.first().and_then(|a| a.trim().parse().ok()))
}

pub fn core_cameras_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let hub = lock();
    let vehicle = match args.first().map(|a| a.trim().parse().ok()) { Some(id) => id.and_then(|id: u8| hub.vehicles.get(&id)), None => hub.active() };
    vehicle.map_or_else(|| json!({ "kind": "object", "class": "Cameras", "available": false }), |v| v.cameras.snapshot(now_ms()))
}

pub fn core_guided_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    lock().guided_snapshot(args.first().and_then(|a| a.trim().parse().ok()))
}

pub fn core_mission_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let hub = lock();
    let vehicle = match args.first().map(|a| a.trim().parse().ok()) { Some(id) => id.and_then(|id: u8| hub.vehicles.get(&id)), None => hub.active() };
    json!({
        "kind": "object",
        "class": "CoreMission",
        "available": vehicle.is_some(),
        "vehicleId": vehicle.map(|v| v.id),
        "plans": vehicle.map(Vehicle::mission_snapshot).unwrap_or(Value::Null),
    })
}

pub fn core_calibration_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    lock().calibration_snapshot(args.first().and_then(|a| a.trim().parse().ok()))
}

pub fn core_remote_id_view(backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let (settings, fix) = crate::remoteidview::inputs(backend, now_us() / 1000);
    let mut hub = lock();
    hub.set_remote_inputs(settings, fix, now_ms());
    hub.remote_snapshot(args.first().and_then(|a| a.trim().parse().ok()))
}

pub fn core_parameters_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let hub = lock();
    let vehicle = match args.first().map(|a| a.trim().parse().ok()) { Some(id) => id.and_then(|id: u8| hub.vehicles.get(&id)), None => hub.active() };
    let listed: serde_json::Map<String, Value> = vehicle.map(|v| v.parameters(v.component).into_iter().map(|(name, value)| (name, json!({ "value": value.as_f64(), "type": value.param_type() }))).collect()).unwrap_or_default();
    json!({
        "kind": "object",
        "class": "CoreParameters",
        "available": vehicle.is_some(),
        "vehicleId": vehicle.map(|v| v.id),
        "ready": vehicle.is_some_and(|v| v.params.ready()),
        "count": listed.len(),
        "parameters": listed,
    })
}

pub fn core_parameter_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let (id, name) = match args {
        [id, name] => (Some(id.trim().parse().ok()), name.trim()),
        [name] => (None, name.trim()),
        _ => (None, ""),
    };
    let hub = lock();
    let vehicle = match id { Some(id) => id.and_then(|id: u8| hub.vehicles.get(&id)), None => hub.active() };
    let value = vehicle.and_then(|v| v.parameter(v.component, name));
    json!({
        "kind": "object",
        "class": "CoreParameter",
        "available": value.is_some(),
        "vehicleId": vehicle.map(|v| v.id),
        "name": name,
        "value": value.map(ParamValue::as_f64),
        "type": value.map(ParamValue::param_type),
        "ready": vehicle.is_some_and(|v| v.params.ready()),
        "meta": vehicle.and_then(|v| v.parameter_meta(name, value)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_armed_vehicle_skips_the_download_only_when_asked_and_not_on_px4() {
        assert!(skips_download(crate::modes::AUTOPILOT_ARDUPILOT, true, true));
        assert!(!skips_download(crate::modes::AUTOPILOT_ARDUPILOT, false, true));
        assert!(!skips_download(crate::modes::AUTOPILOT_ARDUPILOT, true, false));
        assert!(!skips_download(crate::modes::AUTOPILOT_PX4, true, true), "PX4 tries its hash-check cache instead");
    }

    #[test]
    fn a_prearm_text_is_recognised_as_vehicle_handle_status_text_does() {
        assert!(is_prearm("PreArm: RC not calibrated", 4));
        assert!(is_prearm("Preflight Fail: No connection to the GCS", 4), "PX4 words it preflight, in any case");
        assert!(is_prearm("PREFLIGHT FAIL", 2), "critical is severe enough");
        assert!(!is_prearm("Preflight Fail", 1), "an alert is more severe than the rule admits");
        assert!(!is_prearm("Takeoff detected", 6));
    }

    #[test]
    fn the_sample_log_builds_one_vehicle_with_its_facts_and_counts() {
        let bytes = crate::samplelog::bytes();
        let mut hub = Hub::default();
        let summary = crate::tlog::parse(&bytes);
        crate::tlog::for_each(&bytes, |ts, header, message| {
            hub.on_frame(origin(0), header, message, ts, ts / 1000);
        });
        let snapshot = hub.snapshot();
        assert_eq!(snapshot["available"], true);
        let vehicle = hub.active().unwrap();
        assert!(vehicle.heartbeats > 0);
        assert_eq!(vehicle.heartbeats as usize, summary.by_name.get("HEARTBEAT").copied().unwrap_or(0).min(vehicle.heartbeats as usize));
        assert!(vehicle.messages <= summary.frames as u64 && vehicle.messages > 1000);
        let lat = vehicle.gps.latitude.unwrap();
        assert!((-90.0..=90.0).contains(&lat));
        assert!((0.0..360.0).contains(&vehicle.facts.heading));
        assert!(snapshot["vehicle"]["byName"]["HEARTBEAT"].as_u64().unwrap() > 0);
        assert!(snapshot["vehicle"]["batteries"].is_array());
        let count = hub.snapshot()["vehicleIds"].as_array().unwrap().len();
        assert_eq!(count, 1, "the sample log carries one vehicle");
        hub.remove(vehicle.id);
        assert_eq!(hub.snapshot()["available"], false);
    }

    #[test]
    fn mavlink_v1_frames_are_dropped_and_reported_once_after_the_grace_period() {
        use mavlink::dialects::ardupilotmega::{EXTENDED_SYS_STATE_DATA, MavLandedState};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let v1 = Origin { link: 5, replay: false, v2: false };
        hub.on_frame(v1, &header, &copter_heartbeat(0, false), 0, 0);
        assert_eq!(hub.vehicle_ids(), vec![1], "a v1 HEARTBEAT still brings the vehicle up, as MAVLinkProtocol lets it through");
        let state = MavMessage::EXTENDED_SYS_STATE(EXTENDED_SYS_STATE_DATA { landed_state: MavLandedState::MAV_LANDED_STATE_IN_AIR, ..Default::default() });
        assert!(hub.on_frame(v1, &header, &state, 0, 1_000).is_empty());
        assert!(!hub.active().unwrap().flying, "a v1 message other than HEARTBEAT and RADIO_STATUS is dropped");
        assert!(hub.take_v1_reports().is_empty(), "ArduPilot starts in v1, so nothing is said inside the grace period");
        hub.on_frame(v1, &header, &state, 0, 11_000);
        assert_eq!(hub.take_v1_reports(), vec![5]);
        hub.on_frame(v1, &header, &state, 0, 30_000);
        assert!(hub.take_v1_reports().is_empty(), "it is said once per link");
        let mut upgraded = Hub::default();
        upgraded.on_frame(v1, &header, &state, 0, 0);
        upgraded.on_frame(Origin { v2: true, ..v1 }, &header, &copter_heartbeat(0, false), 0, 5_000);
        upgraded.on_frame(v1, &header, &state, 0, 20_000);
        assert!(upgraded.take_v1_reports().is_empty(), "a link that has spoken v2 never warns");
        assert!(mavlink_v1_notice("Radio", "").contains("link 'Radio'. QGroundControl only supports MAVLink v2."));
    }

    #[test]
    fn flying_follows_the_landed_state_and_landing_moves_only_while_armed() {
        use mavlink::dialects::ardupilotmega::{EXTENDED_SYS_STATE_DATA, HEARTBEAT_DATA, MavAutopilot, MavLandedState, MavModeFlag, MavState, MavType};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let beat = |armed: bool, status: MavState| {
            let mut h = HEARTBEAT_DATA::default();
            h.mavtype = MavType::MAV_TYPE_QUADROTOR;
            h.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
            h.system_status = status;
            h.base_mode = if armed { MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED } else { MavModeFlag::empty() };
            MavMessage::HEARTBEAT(h)
        };
        let landed = |state: MavLandedState| MavMessage::EXTENDED_SYS_STATE(EXTENDED_SYS_STATE_DATA { landed_state: state, ..Default::default() });
        let state = |hub: &Hub| (hub.active().unwrap().flying, hub.active().unwrap().landing);
        hub.on_frame(origin(0), &header, &beat(false, MavState::MAV_STATE_STANDBY), 0, 0);
        hub.on_frame(origin(0), &header, &landed(MavLandedState::MAV_LANDED_STATE_LANDING), 1, 0);
        assert_eq!(state(&hub), (true, false), "a disarmed vehicle never starts landing");
        hub.on_frame(origin(0), &header, &beat(true, MavState::MAV_STATE_ACTIVE), 2, 0);
        hub.on_frame(origin(0), &header, &landed(MavLandedState::MAV_LANDED_STATE_LANDING), 3, 0);
        assert_eq!(state(&hub), (true, true));
        hub.on_frame(origin(0), &header, &beat(false, MavState::MAV_STATE_STANDBY), 4, 0);
        assert_eq!(state(&hub), (true, true), "a PX4 heartbeat does not decide flying and disarming does not clear landing");
        hub.on_frame(origin(0), &header, &landed(MavLandedState::MAV_LANDED_STATE_ON_GROUND), 5, 0);
        assert_eq!(state(&hub), (false, true));
    }

    #[test]
    fn ready_to_fly_latches_until_the_prearm_bit_is_reported_again() {
        let fresh = StatusBits { all_healthy: true, ..StatusBits::default() };
        let without = fresh.after(0x20, 0x20, 0);
        assert_eq!((without.ready_to_fly_available, without.ready_to_fly, without.all_healthy, without.unhealthy(), without.requires_gps_fix()), (false, false, false, 0x20, true));
        let ready = without.after(SENSOR_PREARM_CHECK, SENSOR_PREARM_CHECK, SENSOR_PREARM_CHECK);
        assert!(ready.ready_to_fly_available && ready.ready_to_fly);
        let gone = ready.after(0, 0, 0);
        assert!(gone.ready_to_fly_available && gone.ready_to_fly && gone.all_healthy, "a status without the prearm bit leaves the last answer");
    }

    #[test]
    fn an_ardupilot_that_needs_no_arming_is_armed_by_its_motor_outputs() {
        use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavAutopilot, MavModeFlag, MavSysStatusSensor, MavType, SYS_STATUS_DATA};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut h = HEARTBEAT_DATA::default();
        h.mavtype = MavType::MAV_TYPE_FIXED_WING;
        h.autopilot = MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA;
        h.base_mode = MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED;
        hub.on_frame(origin(0), &header, &MavMessage::HEARTBEAT(h.clone()), 0, 0);
        assert!(hub.active().unwrap().armed(), "without ARMING_REQUIRE the heartbeat decides");
        hub.on_frame(origin(0), &header, &param_value("ARMING_REQUIRE", 1, 0, 0.0), 1, 0);
        assert!(hub.active().unwrap().parameter(1, "ARMING_REQUIRE").is_some());
        let motors = |enabled: bool| MavMessage::SYS_STATUS(SYS_STATUS_DATA {
            onboard_control_sensors_present: MavSysStatusSensor::MAV_SYS_STATUS_SENSOR_MOTOR_OUTPUTS,
            onboard_control_sensors_enabled: if enabled { MavSysStatusSensor::MAV_SYS_STATUS_SENSOR_MOTOR_OUTPUTS } else { MavSysStatusSensor::empty() },
            ..Default::default()
        });
        hub.on_frame(origin(0), &header, &motors(false), 1, 0);
        assert!(!hub.active().unwrap().armed());
        hub.on_frame(origin(0), &header, &MavMessage::HEARTBEAT(h), 2, 0);
        assert!(!hub.active().unwrap().armed(), "once motor outputs are reported the heartbeat's armed bit is ignored");
        hub.on_frame(origin(0), &header, &motors(true), 3, 0);
        assert!(hub.active().unwrap().armed());
    }

    #[test]
    fn only_a_vtol_follows_the_forward_flight_state() {
        use mavlink::dialects::ardupilotmega::{EXTENDED_SYS_STATE_DATA, HEARTBEAT_DATA, MavAutopilot, MavType, MavVtolState};
        let forward = MavMessage::EXTENDED_SYS_STATE(EXTENDED_SYS_STATE_DATA { vtol_state: MavVtolState::MAV_VTOL_STATE_FW, ..Default::default() });
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let in_forward_flight = |mavtype: MavType| {
            let mut hub = Hub::default();
            let mut h = HEARTBEAT_DATA::default();
            h.mavtype = mavtype;
            h.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
            hub.on_frame(origin(0), &header, &MavMessage::HEARTBEAT(h), 0, 0);
            hub.on_frame(origin(0), &header, &forward, 1, 0);
            hub.active().unwrap().vtol_in_forward_flight
        };
        assert!(in_forward_flight(MavType::MAV_TYPE_VTOL_TILTROTOR));
        assert!(!in_forward_flight(MavType::MAV_TYPE_QUADROTOR));
    }

    #[test]
    fn an_ardupilot_heartbeat_decides_flying_from_armed_and_status() {
        use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavAutopilot, MavModeFlag, MavState, MavType};
        let mut hub = Hub::default();
        let beat = |component: u8, armed: bool, status: MavState| {
            let mut h = HEARTBEAT_DATA::default();
            h.mavtype = MavType::MAV_TYPE_QUADROTOR;
            h.autopilot = MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA;
            h.system_status = status;
            h.base_mode = if armed { MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED } else { MavModeFlag::empty() };
            (MavHeader { system_id: 1, component_id: component, sequence: 0 }, MavMessage::HEARTBEAT(h))
        };
        let send = |hub: &mut Hub, (header, message): (MavHeader, MavMessage)| hub.on_frame(origin(0), &header, &message, 0, 0);
        send(&mut hub, beat(1, true, MavState::MAV_STATE_STANDBY));
        assert!(!hub.active().unwrap().flying);
        send(&mut hub, beat(1, true, MavState::MAV_STATE_CRITICAL));
        assert!(hub.active().unwrap().flying);
        send(&mut hub, beat(1, false, MavState::MAV_STATE_ACTIVE));
        assert!(!hub.active().unwrap().flying, "a disarmed ArduPilot is on the ground whatever its status");
    }

    #[test]
    fn an_rc_override_repeats_every_200_ms_and_a_release_sends_zero_four_times() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        let sent = |bytes: &[Vec<u8>]| bytes.iter().filter_map(|b| match decode(b) {
            MavMessage::RC_CHANNELS_OVERRIDE(o) => Some((o.chan6_raw, o.chan1_raw)),
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(sent(&vehicle.start_guided(&json!({ "action": "rcOverride", "channel": 6, "pwm": 2500 }), 1_000).unwrap()), vec![(2200, u16::MAX)], "the PWM is clamped and unheld channels are left to the pilot");
        assert!(vehicle.rc_override.contains_key(&6));
        assert!(sent(&vehicle.tick_rc_override(1_100)).is_empty());
        assert_eq!(sent(&vehicle.tick_rc_override(1_200)), vec![(2200, u16::MAX)]);
        assert_eq!(sent(&vehicle.start_guided(&json!({ "action": "rcRelease" }), 1_250).unwrap()), vec![(0, u16::MAX)]);
        let released: Vec<_> = [1_400, 1_600, 1_800, 2_000].into_iter().flat_map(|t| sent(&vehicle.tick_rc_override(t))).collect();
        assert_eq!(released, vec![(0, u16::MAX); 3], "Qt repeats the release three ticks and then forgets the channels");
        assert!(vehicle.rc_override.is_empty() && vehicle.rc_due.is_none());
        assert!(vehicle.start_guided(&json!({ "action": "rcOverride", "channel": 19, "pwm": 1500 }), 3_000).is_err());
    }

    #[test]
    fn the_virtual_joystick_sends_manual_control_scaled_as_vehicle_does() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        let sent = vehicle.start_guided(&json!({ "action": "virtualJoystick", "roll": 0.25, "pitch": -0.5, "yaw": 1.0, "thrust": 0.5 }), 1_000).unwrap();
        let control = sent.iter().find_map(|b| match decode(b) {
            MavMessage::MANUAL_CONTROL(m) => Some(m),
            _ => None,
        }).expect("a MANUAL_CONTROL frame");
        assert_eq!((control.target, control.x, control.y, control.z, control.r), (1, -500, 250, 500, 1000), "pitch is x, roll y, thrust z, yaw r, each times 1000");
        assert_eq!((control.buttons, control.enabled_extensions), (0, 0));
    }

    #[test]
    fn losing_a_relayed_primary_switches_back_to_the_direct_link() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 7, component_id: 1, sequence: 0 };
        hub.on_frame(Origin { link: 11, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(Origin { link: 12, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 3_000);
        hub.check_links(4_000, &LinkKinds { cloud: vec![11], ..LinkKinds::default() });
        assert_eq!(hub.vehicles[&7].primary_link, Some(12));
        assert!(crate::speech::spoken_lines().iter().any(|line| line == "switching communication back to the direct link."));
    }

    #[test]
    fn closing_one_of_two_links_keeps_the_vehicle_on_the_other() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(Origin { link: 1, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(Origin { link: 2, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 100);
        hub.link_closed(1);
        let vehicle = hub.active().expect("MultiVehicleManager removes a vehicle only when allLinksRemoved fires");
        assert_eq!((vehicle.link, vehicle.primary_link), (2, Some(2)), "it is now reached on the link that is still open");
        hub.retain_links(&[]);
        assert!(hub.active().is_none(), "with no link left it goes");
    }

    #[test]
    fn the_primary_link_stays_until_it_goes_quiet_and_then_moves_to_a_live_one() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(Origin { link: 1, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(Origin { link: 2, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 100);
        assert_eq!(hub.active().unwrap().primary_link, Some(1), "the first link heard is primary and a second one does not take over");
        hub.on_frame(Origin { link: 2, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 3_000);
        hub.check_links(4_000, &LinkKinds::default());
        let vehicle = hub.active().unwrap();
        assert_eq!((vehicle.primary_link, vehicle.link_states.iter().map(|(_, _, lost)| *lost).collect::<Vec<_>>()), (Some(2), vec![true, false]), "link 1 was silent past 3.5 s");
        hub.on_frame(Origin { link: 1, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 4_100);
        assert_eq!(hub.active().unwrap().primary_link, Some(2), "a regained link does not take the primary back");
        let spoken = crate::speech::spoken_lines().join("\n");
        ["communication lost on primary link.", "switching communication to secondary link.", "communication regained on secondary link"]
            .iter()
            .for_each(|line| assert!(spoken.contains(line), "{line} was not spoken"));
        assert!(!spoken.contains("new primary link"), "the held primary did not change on regain");
    }

    #[test]
    fn auto_disconnect_closes_a_vehicle_on_total_comm_loss() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        hub.set_link_flag("autoDisconnect", true);
        hub.check_links(10 * LINK_SILENT_MS, &LinkKinds::default());
        assert!(hub.active().is_none(), "VehicleLinkManager closes the vehicle when every link is lost and autoDisconnect is set");
    }

    #[test]
    fn a_live_direct_link_takes_the_primary_from_the_cloud_relay() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let kinds = LinkKinds { cloud: vec![1], high_latency: vec![3], usb_direct: vec![4] };
        hub.on_frame(Origin { link: 1, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 0);
        hub.check_links(100, &kinds);
        hub.on_frame(Origin { link: 2, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 200);
        assert_eq!(hub.active().unwrap().primary_link, Some(2), "_updatePrimaryLink leaves a cloud primary once a direct link is alive");
        assert_eq!(hub.active().unwrap().link, 2, "commands go out on the primary link, as Vehicle::sendMessageOnLinkThreadSafe does with the primary");
        hub.on_frame(Origin { link: 4, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 300);
        assert_eq!(hub.active().unwrap().primary_link, Some(2), "a live direct primary is kept");
        hub.check_links(3_000, &kinds);
        hub.on_frame(Origin { link: 3, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 3_100);
        assert_eq!(hub.active().unwrap().primary_link, Some(2), "a high-latency link never displaces a live normal one");
    }

    #[test]
    fn a_message_heard_on_the_standby_cloud_relay_is_not_handled_twice() {
        use mavlink::dialects::ardupilotmega::{MavSeverity, STATUSTEXT_DATA};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let kinds = LinkKinds { cloud: vec![1], ..LinkKinds::default() };
        hub.on_frame(Origin { link: 1, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 0);
        hub.check_links(100, &kinds);
        hub.on_frame(Origin { link: 2, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 200);
        let text = MavMessage::STATUSTEXT(STATUSTEXT_DATA { severity: MavSeverity::MAV_SEVERITY_INFO, text: (*b"Hello once\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0").into(), ..STATUSTEXT_DATA::default() });
        hub.on_frame(Origin { link: 2, replay: false, v2: true }, &header, &text, 0, 300);
        hub.on_frame(Origin { link: 1, replay: false, v2: true }, &header, &text, 0, 310);
        assert_eq!(hub.active().unwrap().recent.len(), 1, "VehicleLinkManager::isStandby drops what the relay repeats while the direct link is primary");
    }

    #[test]
    fn switching_onto_and_off_a_high_latency_link_starts_and_stops_its_transmission() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let kinds = LinkKinds { high_latency: vec![3], ..LinkKinds::default() };
        hub.on_frame(Origin { link: 2, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 0);
        hub.check_links(100, &kinds);
        hub.on_frame(Origin { link: 3, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 200);
        let control = |frames: Vec<(LinkId, Vec<u8>)>| frames.iter().filter_map(|(link, bytes)| match decode(bytes) {
            MavMessage::COMMAND_LONG(c) if c.command as u16 == CMD_CONTROL_HIGH_LATENCY => Some((*link, c.param1)),
            _ => None,
        }).collect::<Vec<_>>();
        assert!(control(hub.tick(300)).is_empty(), "a live normal link keeps the primary");
        hub.check_links(10_000, &kinds);
        assert_eq!(hub.active().unwrap().primary_link, Some(3));
        assert_eq!(control(hub.tick(10_100)), vec![(3, 1.0)], "VehicleLinkManager::_updatePrimaryLink starts transmission on a high-latency primary");
        hub.on_frame(Origin { link: 2, replay: false, v2: true }, &header, &copter_heartbeat(0, false), 0, 10_200);
        assert_eq!(control(hub.tick(10_300)), vec![(3, 0.0)], "and stops it on that link when leaving it");
    }

    #[test]
    fn any_traffic_keeps_a_link_alive_and_a_high_latency_link_never_times_out() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(origin(0), &header, &MavMessage::ATTITUDE(Default::default()), 0, 3_000);
        hub.check_links(5_000, &LinkKinds::default());
        assert!(!hub.active().unwrap().connection_lost, "VehicleLinkManager restarts a link's timer on any message, not only HEARTBEAT");
        hub.check_links(7_000, &LinkKinds::default());
        assert!(hub.active().unwrap().connection_lost);
        hub.on_frame(origin(0), &header, &MavMessage::ATTITUDE(Default::default()), 0, 7_100);
        assert!(!hub.active().unwrap().connection_lost, "any traffic on a lost link regains it");
        hub.check_links(60_000, &LinkKinds { high_latency: vec![0], ..LinkKinds::default() });
        assert!(!hub.active().unwrap().connection_lost, "a high-latency link is never counted lost");
    }

    #[test]
    fn a_disabled_comm_lost_check_neither_loses_nor_regains_the_vehicle() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        hub.set_link_flag("communicationLostEnabled", false);
        hub.check_links(10 * LINK_SILENT_MS, &LinkKinds::default());
        assert!(!hub.active().unwrap().connection_lost, "Qt's check returns early while the check is off");
        hub.set_link_flag("communicationLostEnabled", true);
        hub.check_links(10 * LINK_SILENT_MS, &LinkKinds::default());
        assert!(hub.active().unwrap().connection_lost);
    }

    #[test]
    fn an_accepted_roi_command_turns_roi_on_and_an_accepted_none_turns_it_off() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let ack = |command, result| MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command, result, ..Default::default() });
        hub.guided(None, &json!({ "action": "roi", "latitude": 47.4, "longitude": 8.5, "altitude": 0.0, "frame": 3 }), 1).unwrap();
        assert_eq!(hub.active().unwrap().roi_coord, Some((47.4, 8.5, 0.0)), "Vehicle::guidedModeROI records the point when it sends, before any ack");
        hub.on_frame(origin(0), &header, &ack(MavCmd::MAV_CMD_DO_SET_ROI_LOCATION, MavResult::MAV_RESULT_DENIED), 1, 0);
        assert!(!hub.active().unwrap().roi_enabled);
        hub.on_frame(origin(0), &header, &ack(MavCmd::MAV_CMD_DO_SET_ROI_LOCATION, MavResult::MAV_RESULT_ACCEPTED), 2, 0);
        assert!(hub.active().unwrap().roi_enabled);
        hub.on_frame(origin(0), &header, &ack(MavCmd::MAV_CMD_DO_SET_ROI_NONE, MavResult::MAV_RESULT_ACCEPTED), 3, 0);
        assert!(!hub.active().unwrap().roi_enabled);
        assert_eq!(hub.active().unwrap().roi_coord, None, "an accepted ROI_NONE clears the point, as Vehicle::_handleCommandAck does");
    }

    #[test]
    fn the_resume_index_is_committed_only_by_a_heartbeat_in_the_mission_mode() {
        use mavlink::dialects::ardupilotmega::MISSION_CURRENT_DATA;
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, true), 0, 0);
        hub.on_frame(origin(0), &header, &MavMessage::MISSION_CURRENT(MISSION_CURRENT_DATA { seq: 4, ..Default::default() }), 1, 0);
        let vehicle = hub.active().unwrap();
        assert_eq!((vehicle.current_mission_index(), vehicle.mission_last_current), (4, -1), "ArduPilot sends home, so the index is the sequence");
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, true), 2, 0);
        assert_eq!(hub.active().unwrap().mission_last_current, -1, "a heartbeat outside Auto leaves the jump to a landing sequence uncommitted");
        hub.on_frame(origin(0), &header, &copter_heartbeat(3, true), 3, 0);
        assert_eq!(hub.active().unwrap().mission_last_current, 4);
        assert_eq!(hub.active().unwrap().fly_items(), 1, "with nothing loaded the fly view holds only its settings item");
        assert_eq!(hub.active().unwrap().resume_mission_index(), 3, "resume at the item before the one it was heading to");
    }

    #[test]
    fn camera_trigger_points_follow_qt_and_arming_clears_them() {
        use mavlink::dialects::ardupilotmega::{CAMERA_FEEDBACK_DATA, CAMERA_IMAGE_CAPTURED_DATA, MavBool};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(origin(0), &header, &MavMessage::CAMERA_FEEDBACK(CAMERA_FEEDBACK_DATA { lat: 10, lng: 20, alt_msl: 5.0, ..Default::default() }), 1, 0);
        hub.on_frame(origin(0), &header, &MavMessage::CAMERA_IMAGE_CAPTURED(CAMERA_IMAGE_CAPTURED_DATA { capture_result: MavBool::MAV_BOOL_TRUE, lat: 30, ..Default::default() }), 2, 0);
        assert_eq!(hub.active().unwrap().trigger_points.len(), 1, "the first captured image after a feedback point is taken as its duplicate");
        hub.on_frame(origin(0), &header, &MavMessage::CAMERA_IMAGE_CAPTURED(CAMERA_IMAGE_CAPTURED_DATA { capture_result: MavBool::MAV_BOOL_TRUE, lat: 40, ..Default::default() }), 3, 0);
        hub.on_frame(origin(0), &header, &MavMessage::CAMERA_IMAGE_CAPTURED(CAMERA_IMAGE_CAPTURED_DATA { capture_result: MavBool::MAV_BOOL_FALSE, lat: 50, ..Default::default() }), 4, 0);
        hub.on_frame(origin(0), &header, &MavMessage::CAMERA_FEEDBACK(CAMERA_FEEDBACK_DATA { lat: 60, ..Default::default() }), 5, 0);
        assert_eq!(hub.active().unwrap().trigger_points.iter().map(|p| (p.0 * 1e7).round() as i32).collect::<Vec<_>>(), vec![10, 40], "a failed capture and feedback after captures arrive add nothing");
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, true), 6, 0);
        assert!(hub.active().unwrap().trigger_points.is_empty() && hub.active().unwrap().trigger_points_appended, "arming starts a fresh set; the list stays marked as changed");
    }

    #[test]
    fn an_orbit_is_active_until_its_status_stops_for_three_seconds() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        assert!(!hub.active().unwrap().orbit_active(0));
        hub.on_frame(origin(0), &header, &MavMessage::ORBIT_EXECUTION_STATUS(Default::default()), 0, 1_000);
        let vehicle = hub.active().unwrap();
        assert_eq!((vehicle.orbit_active(3_999), vehicle.orbit_active(4_000)), (true, false));
    }

    #[test]
    fn the_host_chooses_the_active_vehicle_once_it_has_said_anything() {
        let mut hub = Hub::default();
        assert_eq!(hub.active_id(), None, "with nothing heard there is no vehicle to answer for");
        hub.on_frame(origin(0), &MavHeader { system_id: 7, component_id: 1, sequence: 0 }, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(origin(0), &MavHeader { system_id: 3, component_id: 1, sequence: 0 }, &copter_heartbeat(0, false), 1, 0);
        assert_eq!(hub.active().map(|v| v.id), Some(7), "without a host the first vehicle heard is active");
        assert_eq!(hub.in_arrival_order().iter().map(|v| v.id).collect::<Vec<_>>(), vec![7, 3], "the fleet is listed in arrival order, as MultiVehicleManager appends it");
        hub.set_active(Some(3));
        assert_eq!(hub.active().map(|v| v.id), Some(3));
        hub.remove(3);
        assert_eq!(hub.listed_count(), None, "the list is the host's until it reports one");
        hub.set_listed(vec![7]);
        assert_eq!((hub.listed_count(), hub.listed(0).map(|v| v.id), hub.listed(1).map(|v| v.id)), (Some(1), Some(7), None));
        assert_eq!(hub.active().map(|v| v.id), None, "once the host chooses, losing its choice leaves nothing active until it chooses again");
        hub.set_active(Some(7));
        assert_eq!(hub.active().map(|v| v.id), Some(7));
    }

    #[test]
    fn another_stations_control_request_is_held_until_it_times_out_or_is_answered() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let request = |from: f32, allow: f32, timeout: f32, target: u8| -> Vec<u8> {
            [from, 0.0, allow, timeout, 0.0, 0.0, 0.0].iter().flat_map(|p| p.to_le_bytes()).chain(crate::operatorcontrol::REQUEST_OPERATOR_CONTROL.to_le_bytes()).chain([target, 0, 0]).collect()
        };
        let now = now_ms();
        hub.on_extra(&header, COMMAND_LONG_ID, &request(9.0, 1.0, 20.0, mavout::gcs_system()));
        assert_eq!(hub.active().unwrap().control.incoming_json(now), Value::Null, "GCSControlIndicator drops the popup until the vehicle has reported its control status");
        hub.on_extra(&header, crate::operatorcontrol::CONTROL_STATUS, &[0, mavout::gcs_system()]);
        hub.on_extra(&header, COMMAND_LONG_ID, &request(9.0, 1.0, 20.0, 42));
        assert_eq!(hub.active().unwrap().control.incoming_json(now), Value::Null, "a request addressed to another station is not ours to answer");
        hub.on_extra(&header, COMMAND_LONG_ID, &request(9.0, 1.0, 20.0, mavout::gcs_system()));
        let held = hub.active().unwrap().control.incoming_json(now_ms());
        assert_eq!((held["systemId"].as_u64(), held["allowTakeover"].as_bool(), held["timeoutMs"].as_u64()), (Some(9), Some(true), Some(20_000)));
        assert_eq!(hub.active().unwrap().control.incoming_json(now_ms() + 20_001), Value::Null, "it expires with its own timeout");
        hub.on_extra(&header, COMMAND_LONG_ID, &request(9.0, 0.0, 0.0, mavout::gcs_system()));
        assert_eq!(hub.active().unwrap().control.incoming_json(now_ms())["timeoutMs"].as_u64(), Some(10_000), "no timeout in the request falls back to requestControlTimeout, in seconds");
        hub.vehicles.get_mut(&1).unwrap().control.answered();
        assert_eq!(hub.active().unwrap().control.incoming_json(now_ms()), Value::Null);
    }

    #[test]
    fn a_prearm_text_is_kept_while_the_events_metadata_does_not_declare_arming_checks() {
        use mavlink::dialects::ardupilotmega::CURRENT_EVENT_SEQUENCE_DATA;
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(origin(0), &header, &MavMessage::CURRENT_EVENT_SEQUENCE(CURRENT_EVENT_SEQUENCE_DATA::default()), 1, 1);
        let vehicle = hub.active().unwrap();
        assert!(!vehicle.events.supports_checks(1), "an event sequence alone is not the health_and_arming_check protocol");
        assert!(crate::messagelog::admitted(true, vehicle.events.supports_checks(1), 2, "Preflight Fail: Accel uncalibrated").is_some(), "Vehicle::_handleStatusText drops it only when _healthAndArmingChecksSupported(component)");
    }

    #[test]
    fn rc_values_keep_the_contiguous_channels_and_ignore_a_frame_with_a_gap() {
        use mavlink::dialects::ardupilotmega::RC_CHANNELS_DATA;
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let unused = u16::MAX;
        let rc = |c1, c2, c3| MavMessage::RC_CHANNELS(RC_CHANNELS_DATA { chan1_raw: c1, chan2_raw: c2, chan3_raw: c3, chan4_raw: unused, chan5_raw: unused, chan6_raw: unused, chan7_raw: unused, chan8_raw: unused, chan9_raw: unused, chan10_raw: unused, chan11_raw: unused, chan12_raw: unused, chan13_raw: unused, chan14_raw: unused, chan15_raw: unused, chan16_raw: unused, chan17_raw: unused, chan18_raw: unused, ..Default::default() });
        hub.on_frame(origin(0), &header, &rc(1100, 1500, 1900), 1, 1);
        assert_eq!(hub.active().map(|v| v.rc_values.clone()), Some(vec![1100, 1500, 1900]));
        hub.on_frame(origin(0), &header, &rc(1200, unused, 1800), 2, 2);
        assert_eq!(hub.active().map(|v| v.rc_values.clone()), Some(vec![1100, 1500, 1900]), "Vehicle.cc publishes nothing from a frame whose channels are not contiguous");
    }

    #[test]
    fn shell_output_is_kept_and_a_frame_claiming_more_than_it_carries_is_discarded() {
        use mavlink::dialects::ardupilotmega::{SERIAL_CONTROL_DATA, SerialControlDev};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let shell = |text: &[u8], count: u8| {
            let mut data = [0u8; 70];
            data[..text.len()].copy_from_slice(text);
            MavMessage::SERIAL_CONTROL(SERIAL_CONTROL_DATA { device: SerialControlDev::SERIAL_CONTROL_DEV_SHELL, count, data, ..Default::default() })
        };
        hub.on_frame(origin(0), &header, &shell(b"lost", 88), 1, 1);
        hub.on_frame(origin(0), &header, &shell(b"nsh> ", 5), 2, 2);
        assert_eq!(hub.active().map(|v| v.shell.lines()), Some(vec!["nsh> ".to_string()]), "Vehicle.cc discards a SERIAL_CONTROL whose count overruns its data rather than truncating it");
    }

    #[test]
    fn a_selection_holds_only_heard_vehicles_once_each_and_drops_one_that_goes() {
        let mut hub = Hub::default();
        hub.on_frame(origin(0), &MavHeader { system_id: 7, component_id: 1, sequence: 0 }, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(origin(0), &MavHeader { system_id: 3, component_id: 1, sequence: 0 }, &copter_heartbeat(0, false), 1, 0);
        hub.select_vehicle(3);
        hub.select_vehicle(3);
        hub.select_vehicle(9);
        hub.select_vehicle(7);
        assert_eq!((hub.selected_count(), hub.selected_member(0).map(|v| v.id), hub.selected_member(1).map(|v| v.id)), (2, Some(3), Some(7)), "selection keeps the order it was made in, as selectedVehicles appends");
        hub.remove(3);
        assert_eq!((hub.selected_count(), hub.selected_member(0).map(|v| v.id)), (1, Some(7)), "a vehicle that goes is deselected");
        hub.deselect_vehicle(7);
        assert_eq!(hub.selected_count(), 0);
    }

    #[test]
    fn a_px4_factory_reset_rereads_the_sensor_parameters_a_second_after_it_is_accepted() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, HEARTBEAT_DATA, MavAutopilot, MavCmd, MavResult, MavType};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut beat = HEARTBEAT_DATA::default();
        beat.mavtype = MavType::MAV_TYPE_QUADROTOR;
        beat.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        hub.on_frame(origin(0), &header, &MavMessage::HEARTBEAT(beat), 0, 0);
        [("CAL_ACC0_ID", 0), ("SENS_BOARD_ROT", 1), ("RTL_RETURN_ALT", 2)].iter().for_each(|(name, index)| {
            hub.on_frame(origin(0), &header, &param_value(name, 3, *index, 1.0), 0, 10);
        });
        let ack = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_PREFLIGHT_STORAGE, result: MavResult::MAV_RESULT_ACCEPTED, ..Default::default() });
        hub.on_frame(origin(0), &header, &ack, 0, 50);
        assert!(hub.active().unwrap().sensor_refresh_due.is_none(), "a storage ack nobody asked this page for, a parameter reset say, is not the factory reset");
        hub.vehicles.get_mut(&1).unwrap().start_guided(&json!({ "action": "factoryReset" }), 60).unwrap();
        hub.on_frame(origin(0), &header, &ack, 0, 100);
        let reads = |frames: Vec<(LinkId, Vec<u8>)>| -> Vec<String> { frames.iter().filter_map(|(_, b)| match decode(b) { MavMessage::PARAM_REQUEST_READ(r) => Some(r.param_id.to_str().unwrap().to_string()), _ => None }).collect() };
        assert!(reads(hub.tick(1_099)).is_empty(), "SensorsComponentController waits a second before _refreshParams");
        let mut reread = reads(hub.tick(1_100));
        reread.sort();
        assert_eq!(reread, ["CAL_ACC0_ID", "SENS_BOARD_ROT"], "only CAL_* and SENS_* are bulk-refreshed");
    }

    #[test]
    fn sensor_parameters_are_the_cal_and_sens_families() {
        assert!(sensor_parameter("CAL_MAG0_ROT") && sensor_parameter("SENS_DPRES_OFF") && !sensor_parameter("RTL_RETURN_ALT"));
    }

    #[test]
    fn an_unanswered_control_request_says_so_and_a_second_one_waits() {
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let ask = json!({ "action": "requestControl", "allowTakeover": false, "timeout": 0, "safeTimeout": 10 });
        let sent = hub.guided(None, &ask, 0).unwrap();
        let command_long = |frame: &[u8]| frame.first() == Some(&0xFD) && frame.get(7..10) == Some(&[76, 0, 0][..]);
        assert!(sent.len() == 1 && command_long(&sent[0].1), "sent raw at once: the mavlink crate has no MAV_CMD for 32100");
        hub.guided(None, &ask, 10).unwrap();
        let notices = |hub: &Hub| hub.active().unwrap().pending_notices.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>();
        assert_eq!(notices(&hub), ["Waiting for previous operator control request"], "Vehicle::_requestOperatorControlAckHandler on a duplicate");
        let mut answered = Hub::default();
        answered.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        answered.guided(None, &ask, 0).unwrap();
        let ack: Vec<u8> = crate::operatorcontrol::REQUEST_OPERATOR_CONTROL.to_le_bytes().into_iter().chain([0]).collect();
        answered.on_extra(&header, COMMAND_ACK_ID, &ack);
        (1..=10u64).for_each(|step| { answered.tick(step * crate::mavcmd::ACK_TIMEOUT_MS + 1); });
        assert!(notices(&answered).is_empty(), "an ack the mavlink crate cannot parse still settles the request");
        (1..=10u64).for_each(|step| { hub.tick(step * crate::mavcmd::ACK_TIMEOUT_MS + 1); });
        assert!(notices(&hub).contains(&"No response to operator control request".to_string()));
    }

    #[test]
    fn a_ping_request_is_answered_and_a_ping_response_is_not() {
        use mavlink::dialects::ardupilotmega::PING_DATA;
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let ping = |target_system: u8| MavMessage::PING(PING_DATA { time_usec: 42, seq: 7, target_system, target_component: 0 });
        let answered = hub.on_frame(origin(0), &header, &ping(0), 1, 1);
        assert!(matches!(decode(&answered[0].1), MavMessage::PING(p) if (p.time_usec, p.seq, p.target_system, p.target_component) == (42, 7, 1, 1)), "Vehicle::_handlePing echoes the request back to its sender");
        assert!(hub.on_frame(origin(0), &header, &ping(255), 2, 2).is_empty(), "a response addressed to someone is not a request");
    }

    #[test]
    fn a_high_latency_vehicle_is_placed_and_moded_from_its_reports() {
        use mavlink::dialects::ardupilotmega::{HIGH_LATENCY2_DATA, MavAutopilot, MavType};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 3, component_id: 1, sequence: 0 };
        let report = |autopilot: MavAutopilot, custom0: i8| MavMessage::HIGH_LATENCY2(HIGH_LATENCY2_DATA { latitude: 474_000_000, longitude: 85_000_000, altitude: 520, custom_mode: 5, groundspeed: 50, heading: 45, mavtype: MavType::MAV_TYPE_QUADROTOR, autopilot, custom0, ..Default::default() });
        hub.on_frame(origin(7), &header, &report(MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA, 0x81u8 as i8), 0, 0);
        let vehicle = hub.vehicles.get(&3).expect("MAVLinkProtocol treats HIGH_LATENCY2 as a heartbeat");
        assert_eq!((vehicle.custom_mode, vehicle.armed()), (5, true), "ArduPilot carries base mode and arming in custom0");
        assert_eq!(vehicle.facts.coordinate, Some((47.4, 8.5, 520.0)));
        assert_eq!((vehicle.facts.ground_speed, vehicle.facts.heading), (10.0, 90.0), "HL2 speeds in 0.2 m/s and heading in 2 degree steps");
        hub.on_frame(origin(7), &header, &report(MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA, 0x01), 1, 1);
        assert!(!hub.vehicles[&3].armed());
        let mut iridium = Hub::default();
        iridium.check_links(0, &LinkKinds { high_latency: vec![7], ..LinkKinds::default() });
        let first = iridium.on_frame(origin(7), &header, &report(MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA, 0x01), 0, 0);
        assert!(iridium.vehicles[&3].commands.high_latency, "the vehicle knows its link is high latency, as Vehicle::isHighLatency reads the primary link");
        assert!(!first.iter().any(|(_, b)| matches!(decode(b), MavMessage::PARAM_REQUEST_LIST(_) | MavMessage::MISSION_REQUEST_LIST(_))), "and InitialConnectStateMachine skips the parameter and plan loads over it");
        iridium.check_links(1, &LinkKinds::default());
        assert!(!iridium.vehicles[&3].commands.high_latency, "a link reconfigured as normal stops being treated as high latency on the next check");
        assert_eq!(high_latency_custom_mode(crate::modes::AUTOPILOT_PX4, 0x0304), 0x0304_0000, "PX4 packs main and sub mode into the high half");
        assert_eq!(high_latency_sensors(1 | 32), 32 | 4, "QGCMAVLink::highLatencyFailuresToMavSysStatus: GPS and magnetometer failures become present, enabled sensors");
    }

    #[test]
    fn a_terrain_request_before_gps_lock_is_ignored() {
        use mavlink::dialects::ardupilotmega::TERRAIN_REQUEST_DATA;
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(origin(0), &header, &MavMessage::TERRAIN_REQUEST(TERRAIN_REQUEST_DATA { mask: 1, lat: 0, lon: 0, grid_spacing: 100 }), 1, 1);
        assert_eq!((hub.active().unwrap().terrain_request, hub.active().unwrap().terrain_due), (None, None), "TerrainProtocolHandler drops lat=0, lon=0");
    }

    #[test]
    fn ardupilot_calibration_prompts_are_lowered_to_info() {
        use mavlink::dialects::ardupilotmega::{MavSeverity, STATUSTEXT_DATA};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let text = |words: &str| MavMessage::STATUSTEXT(STATUSTEXT_DATA { severity: MavSeverity::MAV_SEVERITY_CRITICAL, text: mavout::chars(words), ..Default::default() });
        hub.on_frame(origin(0), &header, &text("Place vehicle level and press any key."), 1_000, 1);
        hub.on_frame(origin(0), &header, &text("Calibration successful"), 2_000, 2);
        hub.on_frame(origin(0), &header, &text("PreArm: RC not calibrated"), 3_000, 3);
        let severities: Vec<u8> = hub.active().unwrap().recent.iter().map(|s| s.severity).collect();
        assert_eq!(severities, [6, 6, 2], "APMFirmwarePlugin::_handleIncomingStatusText lowers only the calibration prompts");
        assert_eq!(calibration_as_info(calibration_text("Place vehicle"), false).severity, 2, "PX4 keeps its own severity");
    }

    fn calibration_text(text: &str) -> crate::statustext::StatusText {
        crate::statustext::StatusText { component: 1, severity: 2, text: text.to_string() }
    }

    #[test]
    fn a_prearm_complaint_shows_for_35_seconds_and_a_repeat_within_10_does_not_restart_it() {
        use mavlink::dialects::ardupilotmega::{MavSeverity, STATUSTEXT_DATA};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        let text = |words: &str| MavMessage::STATUSTEXT(STATUSTEXT_DATA { severity: MavSeverity::MAV_SEVERITY_CRITICAL, text: mavout::chars(words), ..Default::default() });
        hub.on_frame(origin(0), &header, &text("PreArm: RC not calibrated"), 1_000_000, 1_000);
        hub.on_frame(origin(0), &header, &text("PreArm: RC not calibrated"), 9_000_000, 9_000);
        let vehicle = hub.active().unwrap();
        assert_eq!(vehicle.prearm_error(35_999), "PreArm: RC not calibrated");
        assert_eq!(vehicle.prearm_error(36_000), "", "the repeat inside ten seconds did not restart the clock");
        hub.on_frame(origin(0), &header, &text("Arming motors"), 40_000_000, 40_000);
        assert_eq!(hub.active().unwrap().prearm_error(40_000), "");
    }

    #[test]
    fn a_ground_station_heartbeat_does_not_make_a_vehicle() {
        use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavAutopilot, MavType};
        let mut hub = Hub::default();
        let mut gcs = HEARTBEAT_DATA::default();
        gcs.mavtype = MavType::MAV_TYPE_GCS;
        gcs.autopilot = MavAutopilot::MAV_AUTOPILOT_INVALID;
        hub.on_frame(origin(0), &MavHeader { system_id: 255, component_id: 190, sequence: 0 }, &MavMessage::HEARTBEAT(gcs), 0, 0);
        assert_eq!(hub.snapshot()["available"], false);
        let mut companion = HEARTBEAT_DATA::default();
        companion.mavtype = MavType::MAV_TYPE_QUADROTOR;
        companion.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        hub.on_frame(origin(0), &MavHeader { system_id: 1, component_id: 191, sequence: 0 }, &MavMessage::HEARTBEAT(companion.clone()), 1, 0);
        assert_eq!(hub.snapshot()["available"], false, "a heartbeat from a non-autopilot component makes no vehicle");
        companion.mavtype = MavType::MAV_TYPE_ONBOARD_CONTROLLER;
        hub.on_frame(origin(0), &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(companion), 2, 0);
        assert_eq!(hub.snapshot()["available"], false, "an onboard controller type makes no vehicle");
        let mut quad = HEARTBEAT_DATA::default();
        quad.mavtype = MavType::MAV_TYPE_QUADROTOR;
        quad.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        quad.base_mode = mavlink::dialects::ardupilotmega::MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED;
        hub.on_frame(origin(0), &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 5, 0);
        let snapshot = hub.snapshot();
        assert_eq!((snapshot["vehicle"]["armed"].as_bool(), snapshot["vehicle"]["autopilot"].as_u64(), snapshot["vehicle"]["heartbeats"].as_u64()), (Some(true), Some(12), Some(1)));
        hub.check_links(LINK_SILENT_MS, &LinkKinds::default());
        assert!(!hub.snapshot()["vehicle"]["connectionLost"].as_bool().unwrap());
        hub.check_links(LINK_SILENT_MS + 1, &LinkKinds::default());
        assert_eq!((hub.snapshot()["available"].as_bool(), hub.snapshot()["vehicle"]["connectionLost"].as_bool()), (Some(true), Some(true)), "a silent vehicle is kept and flagged, as the Qt head keeps it until its link closes");
        assert_eq!(hub.snapshot()["heard"], false, "a head reading only the top-level flags drew a frozen aircraft as a live one, because the liveness answer sat a level below the one it reached for");
        hub.on_frame(origin(0), &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 3_500_007, 0);
        assert_eq!(hub.snapshot()["vehicle"]["connectionLost"], false);
        assert_eq!(hub.snapshot()["heard"], true, "heard is the liveness answer beside available, which only says a record exists");
        let mut two = Hub::default();
        two.on_frame(origin(0), &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 5, 0);
        two.on_frame(origin(0), &MavHeader { system_id: 7, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad), 6, 0);
        assert_eq!(two.snapshot()["vehicle"]["id"], 1);
        assert_eq!(two.snapshot_of(Some(7))["vehicle"]["id"], 7);
        assert_eq!(two.snapshot_of(Some(9))["available"], false);
        assert_eq!(two.guided_snapshot(Some(9))["available"], false, "guided answered for a vehicle nobody asked about");
        assert_eq!(two.calibration_snapshot(Some(9))["available"], false, "calibration answered for a vehicle nobody asked about");
        assert_eq!(two.remote_snapshot(Some(9))["available"], false, "remote id answered for a vehicle nobody asked about");
        assert_eq!(two.guided_snapshot(None)["vehicleId"], 1, "asking for no vehicle in particular still means the active one");
        assert_eq!(two.guided_snapshot(Some(7))["vehicleId"], 7);
    }

    use num_traits::FromPrimitive;

    fn origin(link: LinkId) -> Origin {
        Origin { link, replay: false, v2: true }
    }

    fn decode(bytes: &[u8]) -> MavMessage {
        mavlink::read_versioned_msg::<MavMessage, _>(&mut mavlink::peek_reader::PeekReader::new(bytes), mavlink::ReadVersion::Single(mavlink::MavlinkVersion::V2)).unwrap().1
    }

    fn copter_heartbeat(custom_mode: u32, armed: bool) -> MavMessage {
        use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavAutopilot, MavModeFlag, MavType};
        let mut h = HEARTBEAT_DATA::default();
        h.mavtype = MavType::MAV_TYPE_QUADROTOR;
        h.autopilot = MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA;
        h.custom_mode = custom_mode;
        h.base_mode = MavModeFlag::from_bits_retain(if armed { 0x81 } else { 0x01 });
        MavMessage::HEARTBEAT(h)
    }

    #[test]
    #[allow(deprecated)]
    fn a_guided_takeoff_runs_through_the_hub_and_sends_on_the_vehicle_link() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, GLOBAL_POSITION_INT_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        assert_eq!(hub.active().unwrap().planning_state().minimum_takeoff_altitude, 3.048, "with no PILOT_TKOFF_ALT the takeoff floor is the firmware default QGC uses");
        let refused = hub.guided(None, &json!({ "action": "takeoff", "altitude": 10.0 }), 1_000).unwrap_err();
        assert_eq!(refused, "Unable to takeoff, vehicle position not known.");
        let position = MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA { lat: 474000000, lon: 85000000, alt: 500_000, relative_alt: 0, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &position, 1_100_000, 1100);
        let started = hub.guided(None, &json!({ "action": "takeoff", "altitude": 10.0 }), 1_100).unwrap();
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].0, 4, "sent on the link the heartbeat came from");
        let MavMessage::COMMAND_LONG(set_mode) = decode(&started[0].1) else { panic!() };
        assert_eq!((set_mode.command, set_mode.param2, set_mode.target_system), (MavCmd::MAV_CMD_DO_SET_MODE, 4.0, 1));
        assert_eq!(hub.guided_snapshot(None)["guided"]["state"], "running");
        assert!(hub.guided(None, &json!({ "action": "land" }), 1_200).is_err(), "one action at a time");
        let ack = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_DO_SET_MODE, result: MavResult::MAV_RESULT_ACCEPTED, ..Default::default() });
        assert!(hub.on_frame(origin(4), &autopilot, &ack, 1_300_000, 1300).is_empty());
        assert_eq!(hub.active().unwrap().mode_ack, Some((0, 1)), "the DO_SET_MODE ack is kept for the mode indicator");
        let armed = hub.on_frame(origin(4), &autopilot, &copter_heartbeat(4, false), 2_000_000, 2000);
        let arming: Vec<(MavCmd, f32)> = armed.iter().filter_map(|(_, bytes)| match decode(bytes) { MavMessage::COMMAND_LONG(c) => Some((c.command, c.param1)), _ => None }).collect();
        assert!(arming.contains(&(MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, 1.0)), "{arming:?}");
        assert!(hub.tick(2_500).is_empty());
        let takeoff = hub.on_frame(origin(4), &autopilot, &copter_heartbeat(4, true), 3_000_000, 3000);
        let sent: Vec<(MavCmd, f32)> = takeoff.iter().filter_map(|(_, bytes)| match decode(bytes) { MavMessage::COMMAND_LONG(c) => Some((c.command, c.param7)), _ => None }).collect();
        assert!(sent.contains(&(MavCmd::MAV_CMD_NAV_TAKEOFF, 10.0)), "the takeoff goes out beside the gimbal manager discovery QGC sends on every heartbeat: {sent:?}");
        assert_eq!(hub.guided_snapshot(Some(1))["guided"]["state"], "done");
        let denied = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_NAV_TAKEOFF, result: MavResult::MAV_RESULT_DENIED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &denied, 3_100_000, 3100);
        assert_eq!(hub.guided_snapshot(None)["guided"]["errors"][0], "Takeoff (MAV_CMD_NAV_TAKEOFF) command denied");
        let goto = hub.guided(None, &json!({ "action": "goto", "latitude": 47.405, "longitude": 8.505 }), 3_200).unwrap();
        assert!(matches!(decode(&goto[0].1), MavMessage::COMMAND_INT(c) if c.command == MavCmd::MAV_CMD_DO_REPOSITION && c.x == 474050000));
        assert!(hub.guided(None, &json!({ "action": "goto", "latitude": 47.5, "longitude": 8.6 }), 3_300).is_err(), "13 km is past the 1000 m Max Go To distance");
        assert!(matches!(decode(&goto[1].1), MavMessage::MISSION_ITEM(i) if i.current == 2));
        let unsupported = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_DO_REPOSITION, result: MavResult::MAV_RESULT_UNSUPPORTED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &unsupported, 3_300_000, 3300);
        assert_eq!(hub.guided_snapshot(None)["guided"]["repositionSupported"], false);
        let rtl = hub.guided(None, &json!({ "action": "rtl" }), 3_400).unwrap();
        assert_eq!(rtl.len(), 1);
        hub.retain_links(&[4]);
        assert_eq!(hub.guided_snapshot(None)["available"], true);
        hub.link_closed(4);
        assert_eq!(hub.guided_snapshot(None)["available"], false, "a vehicle goes with its link, as it does in the Qt head");
        assert!(hub.guided(None, &json!({ "action": "land" }), 3_500).is_err());
        hub.on_frame(origin(6), &autopilot, &copter_heartbeat(4, true), 4_000_000, 4_000);
        assert_eq!(hub.guided(None, &json!({ "action": "land" }), 4_100).unwrap()[0].0, 6, "a heartbeat on a new link makes a fresh vehicle bound to it");
    }

    fn refuse_pack(hub: &mut Hub, autopilot: &MavHeader, frames: &[(LinkId, Vec<u8>)], now_ms: u64) -> Vec<(LinkId, Vec<u8>)> {
        let open = frames.iter().map(|(_, bytes)| decode(bytes)).find_map(|m| match m {
            MavMessage::FILE_TRANSFER_PROTOCOL(f) => ftp::Request::decode(&f.payload),
            _ => None,
        });
        let open = open.expect("ArduPilot's parameters are asked for as a file first");
        assert_eq!((open.opcode, String::from_utf8_lossy(&open.data).trim_end_matches('\0')), (ftp::CMD_OPEN_FILE_RO, params::PACK_URI));
        let refused = ftp::Request { seq: open.seq + 1, session: 0, opcode: ftp::RSP_NAK, req_opcode: ftp::CMD_OPEN_FILE_RO, burst_complete: false, offset: 0, data: vec![10] };
        hub.on_frame(origin(4), autopilot, &ftp_reply(refused), now_ms * 1000, now_ms)
    }

    fn connect_copter(hub: &mut Hub, autopilot: &MavHeader) {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        hub.on_frame(origin(4), autopilot, &copter_heartbeat(5, false), 0, 0);
        let refused = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED, ..Default::default() });
        hub.on_frame(origin(4), autopilot, &refused, 1, 1);
        hub.on_frame(origin(4), autopilot, &refused, 2, 2);
        let opened = hub.on_frame(origin(4), autopilot, &refused, 3, 3);
        refuse_pack(hub, autopilot, &opened, 3);
        hub.on_frame(origin(4), autopilot, &param_value("RTL_ALT", 1, 0, 1500.0), 4, 4);
        let fence_asked = hub.on_frame(origin(4), autopilot, &mission_count(0), 5, 5);
        assert!(matches!(decode(&fence_asked[1].1), MavMessage::MISSION_REQUEST_LIST(r) if r.mission_type as u8 == 1), "ArduPilot's assumed capabilities include the fence, and MAVLink 2 was seen, so the fence is read next");
        let rally_asked = hub.on_frame(origin(4), autopilot, &plan_count(1, 0), 6, 6);
        assert!(matches!(decode(&rally_asked[1].1), MavMessage::MISSION_REQUEST_LIST(r) if r.mission_type as u8 == 2));
        hub.on_frame(origin(4), autopilot, &plan_count(2, 0), 7, 7);
        assert_eq!(hub.snapshot()["vehicle"]["initialConnectComplete"], true);
    }

    fn plan_count(plan: u8, count: u16) -> MavMessage {
        use mavlink::dialects::ardupilotmega::{MISSION_COUNT_DATA, MavMissionType};
        MavMessage::MISSION_COUNT(MISSION_COUNT_DATA { count, target_system: 255, target_component: 190, mission_type: MavMissionType::from_u8(plan).unwrap(), ..Default::default() })
    }

    #[allow(deprecated)]
    fn mission_item(seq: u16, latitude: f64) -> MavMessage {
        use mavlink::dialects::ardupilotmega::{MISSION_ITEM_INT_DATA, MavCmd, MavFrame};
        MavMessage::MISSION_ITEM_INT(MISSION_ITEM_INT_DATA { param1: 0.0, param2: 0.0, param3: 0.0, param4: 0.0, x: (latitude * 1e7) as i32, y: 85000000, z: 50.0, seq, command: MavCmd::MAV_CMD_NAV_WAYPOINT, target_system: 255, target_component: 190, frame: MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT, current: 0, autocontinue: 1, ..Default::default() })
    }

    fn mission_count(count: u16) -> MavMessage {
        use mavlink::dialects::ardupilotmega::MISSION_COUNT_DATA;
        MavMessage::MISSION_COUNT(MISSION_COUNT_DATA { count, target_system: 255, target_component: 190, ..Default::default() })
    }

    fn request_of(bytes: &[u8]) -> (u32, f32) {
        match decode(bytes) {
            MavMessage::COMMAND_LONG(c) if c.command == mavlink::dialects::ardupilotmega::MavCmd::MAV_CMD_REQUEST_MESSAGE => (512, c.param1),
            other => panic!("not a request: {other:?}"),
        }
    }

    fn param_value(name: &str, count: u16, index: u16, value: f32) -> MavMessage {
        use mavlink::dialects::ardupilotmega::{MavParamType, PARAM_VALUE_DATA};
        MavMessage::PARAM_VALUE(PARAM_VALUE_DATA { param_value: value, param_count: count, param_index: index, param_id: mavout::param_id(name), param_type: MavParamType::MAV_PARAM_TYPE_REAL32 })
    }

    #[test]
    fn a_new_copter_is_walked_through_the_connect_sequence_down_to_its_parameters() {
        use mavlink::dialects::ardupilotmega::{AUTOPILOT_VERSION_DATA, COMMAND_ACK_DATA, MavCmd, MavProtocolCapability, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        let first = hub.on_frame(origin(4), &autopilot, &copter_heartbeat(5, false), 1_000_000, 1_000);
        assert_eq!(request_of(&first[0].1), (512, 148.0), "the first thing asked of a new vehicle is its autopilot version");
        let streams: Vec<(u8, u16)> = first.iter().filter_map(|(_, b)| match decode(b) { MavMessage::REQUEST_DATA_STREAM(r) => Some((r.req_stream_id, r.req_message_rate)), _ => None }).collect();
        assert_eq!(streams, [(1, 2), (2, 2), (3, 2), (6, 3), (10, 10), (11, 10), (12, 3)], "APMFirmwarePlugin::initializeStreamRates asks for every stream at its default rate");
        let intervals: Vec<f32> = first.iter().filter_map(|(_, b)| match decode(b) { MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL => Some(c.param1), _ => None }).collect();
        assert_eq!(intervals, [242.0, 245.0], "home position and extended sys state are streamed because ArduPilot does not by default");
        assert_eq!(first.len(), 10);
        let quiet: Vec<u8> = hub.tick(11_500).into_iter().filter_map(|(_, b)| match decode(&b) { MavMessage::REQUEST_DATA_STREAM(r) => Some(r.req_stream_id), _ => None }).collect();
        assert_eq!(quiet.len(), 7, "ten silent seconds without BATTERY_STATUS or HOME_POSITION re-request the streams");
        assert_eq!(hub.snapshot()["vehicle"]["connectStep"], "AutopilotVersion");
        let version = MavMessage::AUTOPILOT_VERSION(AUTOPILOT_VERSION_DATA { capabilities: MavProtocolCapability::from_bits_retain(8 | 4), flight_sw_version: 0x04050600, ..Default::default() });
        let after_version = hub.on_frame(origin(4), &autopilot, &version, 1_100_000, 1_100);
        assert_eq!(request_of(&after_version[0].1), (512, 435.0), "ArduPilot skips the protocol version and goes to standard modes");
        assert_eq!(hub.snapshot()["vehicle"]["capabilities"], 12);
        assert_eq!(hub.snapshot()["vehicle"]["firmware"]["version"], "4.5.6 (0)");
        let unsupported = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED, ..Default::default() });
        let after_modes = hub.on_frame(origin(4), &autopilot, &unsupported, 1_200_000, 1_200);
        assert_eq!(request_of(&after_modes[0].1), (512, 397.0), "modes refused, component metadata asked for next");
        let after_metadata = hub.on_frame(origin(4), &autopilot, &unsupported, 1_250_000, 1_250);
        let listed_after_pack = refuse_pack(&mut hub, &autopilot, &after_metadata, 1_260);
        assert!(matches!(decode(&listed_after_pack[0].1), MavMessage::PARAM_REQUEST_LIST(l) if l.target_system == 1 && l.target_component == 0), "metadata refused and no parameter file, so parameters are requested from every component");
        assert_eq!(hub.snapshot()["vehicle"]["connectStep"], "Parameters");
        assert!(hub.on_frame(origin(4), &autopilot, &param_value("RTL_ALT", 2, 0, 1500.0), 1_300_000, 1_300).is_empty());
        assert_eq!(hub.snapshot()["vehicle"]["parameters"]["ready"], false);
        let listed = hub.on_frame(origin(4), &autopilot, &param_value("WPNAV_SPEED", 2, 1, 250.0), 1_400_000, 1_400);
        let clock: Vec<u64> = listed.iter().filter_map(|(_, b)| match decode(b) { MavMessage::SYSTEM_TIME(t) => Some(t.time_unix_usec), _ => None }).collect();
        assert_eq!(clock.len(), 2, "Vehicle::_parametersReady sends the time twice, for a noisy link");
        assert!(clock.iter().all(|usec| *usec > 1_600_000_000_000_000), "as wall-clock microseconds since the epoch");
        assert!(matches!(decode(&listed[2].1), MavMessage::MISSION_REQUEST_LIST(_)), "with the parameters in, the mission is read from the vehicle");
        assert_eq!(hub.snapshot()["vehicle"]["parameters"], json!({ "ready": true, "progress": 1.0, "count": 2 }));
        assert_eq!(hub.snapshot()["vehicle"]["initialConnectComplete"], false);
        use mavlink::dialects::ardupilotmega::{MISSION_COUNT_DATA, MavMissionType};
        let fence = MavMessage::MISSION_COUNT(MISSION_COUNT_DATA { count: 3, target_system: 255, target_component: 190, mission_type: MavMissionType::MAV_MISSION_TYPE_FENCE, ..Default::default() });
        assert!(hub.on_frame(origin(4), &autopilot, &fence, 1_450_000, 1_450).is_empty(), "a fence count from another transfer on the link is not the mission count");
        let requested = hub.on_frame(origin(4), &autopilot, &mission_count(1), 1_500_000, 1_500);
        assert!(matches!(decode(&requested[0].1), MavMessage::MISSION_REQUEST_INT(r) if r.seq == 0));
        let acked = hub.on_frame(origin(4), &autopilot, &mission_item(0, 47.4), 1_600_000, 1_600);
        assert!(matches!(decode(&acked[0].1), MavMessage::MISSION_ACK(a) if a.mavtype as u8 == 0));
        assert_eq!(acked.len(), 1, "without the fence and rally capability bits nothing more is asked");
        let snapshot = hub.snapshot();
        assert_eq!(snapshot["vehicle"]["initialConnectComplete"], true);
        assert_eq!(snapshot["vehicle"]["mission"], json!({ "inProgress": false, "transaction": null, "count": 1, "progress": 1.0, "error": null }));
        assert_eq!(snapshot["vehicle"]["maxProtoVersion"], 200);
        let plans = hub.active().unwrap().mission_snapshot();
        assert_eq!((plans["mission"]["items"][0]["command"].as_u64(), plans["mission"]["items"][0]["params"][4].as_f64()), (Some(16), Some(47.4)));
        assert_eq!(snapshot["vehicle"]["connectProgress"], 1.0);
        assert_eq!(hub.active().unwrap().parameter(1, "RTL_ALT").map(ParamValue::as_f64), Some(1500.0));
        assert!(hub.tick(10_000).is_empty(), "nothing is pending once connected");
        let written = hub.parameter_request(None, &json!({ "name": "RTL_ALT", "value": 2000.0 }), 11_000).unwrap();
        let MavMessage::PARAM_SET(set) = decode(&written[0].1) else { panic!() };
        assert_eq!((set.param_id.to_str().unwrap(), set.param_value, set.param_type as u8, set.target_component), ("RTL_ALT", 2000.0, 9, 1));
        assert_eq!(hub.parameter_request(None, &json!({ "name": "NEW_ONE", "value": 1.0 }), 11_000).unwrap_err(), "NEW_ONE is not a parameter of component 1.");
        assert!(hub.parameter_request(None, &json!({ "name": "RTL ALT", "value": 1.0 }), 11_000).is_err(), "a name with a space never goes on the wire");
        use mavlink::dialects::ardupilotmega::{MavParamType, PARAM_VALUE_DATA};
        let echoed = MavMessage::PARAM_VALUE(PARAM_VALUE_DATA { param_value: 2000.0, param_count: 2, param_index: 0, param_id: mavout::param_id("RTL_ALT"), param_type: MavParamType::MAV_PARAM_TYPE_INT32 });
        hub.on_frame(origin(4), &autopilot, &echoed, 11_100_000, 11_100);
        assert_eq!(hub.active().unwrap().parameter(1, "RTL_ALT"), Some(ParamValue::I32(2000)), "ArduPilot casts an integer into the float rather than packing its bytes");
        let integer = hub.parameter_request(None, &json!({ "name": "RTL_ALT", "value": 2100.0 }), 11_200).unwrap();
        let MavMessage::PARAM_SET(set) = decode(&integer[0].1) else { panic!() };
        assert_eq!((set.param_value, set.param_type as u8), (2100.0, 6), "and expects the same cast back");
        hub.on_frame(origin(4), &autopilot, &param_value("RTL_ALT", 2, 0, 2000.0), 11_100_000, 11_100);
        assert_eq!(hub.active().unwrap().parameter(1, "RTL_ALT").map(ParamValue::as_f64), Some(2000.0));
        let refreshed = hub.parameter_request(Some(1), &json!({ "name": "WPNAV_SPEED", "refresh": true }), 12_000).unwrap();
        assert!(matches!(decode(&refreshed[0].1), MavMessage::PARAM_REQUEST_READ(r) if r.param_index == -1 && r.param_id.to_str().unwrap() == "WPNAV_SPEED"));
        assert!(hub.parameter_request(Some(9), &json!({ "name": "X", "value": 1.0 }), 12_000).is_err());
    }

    #[test]
    fn rtcm_goes_once_to_each_vehicle_link_as_gps_rtcm_data() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let sent = hub.inject_rtcm(&[0xD3; 10]);
        assert_eq!(sent.len(), 1);
        assert!(matches!(decode(&sent[0].1), MavMessage::GPS_RTCM_DATA(d) if d.len == 10 && d.flags == 0));
        assert!(Hub::default().inject_rtcm(&[0xD3; 10]).is_empty(), "no vehicle, nothing sent");
    }

    #[test]
    fn a_script_directory_listing_runs_over_the_files_slot() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let sent = hub.vehicles.get_mut(&1).unwrap().start_guided(&json!({ "action": "ftp", "op": "list", "path": "/APM/scripts/" }), 20_000).unwrap();
        let request = ftp_request(&sent[0]);
        assert_eq!((request.opcode, request.data.as_slice()), (ftp::CMD_LIST_DIRECTORY, b"/APM/scripts/".as_slice()));
        assert!(hub.vehicles.get_mut(&1).unwrap().start_guided(&json!({ "action": "ftp", "op": "delete", "path": "/APM/scripts/a.lua" }), 20_000).is_err(), "one transfer at a time");
        let listed = ftp::Request { seq: request.seq + 1, opcode: ftp::RSP_ACK, req_opcode: ftp::CMD_LIST_DIRECTORY, data: b"Fhello.lua\t120\0Dmodules\0".to_vec(), ..Default::default() };
        let next = hub.on_frame(origin(4), &autopilot, &ftp_reply(listed), 20_100_000, 20_100);
        let more = ftp_request(&next[0].1);
        let eof = ftp::Request { seq: more.seq + 1, opcode: ftp::RSP_NAK, req_opcode: ftp::CMD_LIST_DIRECTORY, data: vec![ftp::ERR_EOF], ..Default::default() };
        hub.on_frame(origin(4), &autopilot, &ftp_reply(eof), 20_200_000, 20_200);
        let files = &hub.active().unwrap().files;
        assert!(!files.busy());
        assert_eq!(files.last, Some(("/APM/scripts/".to_string(), Ok(crate::filejobs::Outcome::Listed(vec!["Fhello.lua\t120".into(), "Dmodules".into()])))));
    }

    #[test]
    fn a_camera_definition_download_asks_the_camera_and_stays_out_of_the_files_list() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        assert!(vehicle.start_guided(&json!({ "action": "ftp", "op": "download", "path": "/x.xml" }), 20_000).is_err(), "a plain download still needs a local file");
        let sent = vehicle.start_guided(&json!({ "action": "ftp", "op": "download", "path": "mftp://camera.xml", "cameraDefinition": 100 }), 20_000).unwrap();
        assert!(matches!(decode(&sent[0]), MavMessage::FILE_TRANSFER_PROTOCOL(d) if d.target_component == 100), "QGC downloads the definition from the camera's own component");
        assert_eq!(vehicle.camera_definition_from, Some(100));
        assert!(vehicle.start_guided(&json!({ "action": "ftp", "op": "list", "path": "/" }), 20_000).err().as_deref() == Some(FILES_BUSY));
    }

    #[test]
    fn named_refresh_reads_back_only_the_parameters_asked_for() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        let sent = vehicle.start_guided(&json!({ "action": "refreshParameters", "names": ["FOLL_SYSID", "FOLL_OFS_X"] }), 20_000).unwrap();
        let read: Vec<String> = sent.iter().filter_map(|b| match decode(b) { MavMessage::PARAM_REQUEST_READ(r) => Some(r.param_id.to_str().unwrap().to_string()), _ => None }).collect();
        assert_eq!(read, ["FOLL_SYSID", "FOLL_OFS_X"]);
    }

    #[test]
    fn pid_tuning_raises_the_attitude_streams_one_acknowledged_interval_at_a_time() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let interval = |bytes: &[Vec<u8>]| bytes.iter().find_map(|b| match decode(b) { MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL => Some((c.param1, c.param2)), _ => None });
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        assert_eq!(interval(&vehicle.start_guided(&json!({ "action": "pidTuningMode", "mode": 1 }), 20_000).unwrap()), Some((83.0, 10_000.0)));
        let ack = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL, result: MavResult::MAV_RESULT_ACCEPTED, ..Default::default() });
        let next: Vec<Vec<u8>> = hub.on_frame(origin(4), &autopilot, &ack, 20_100_000, 20_100).into_iter().map(|(_, b)| b).collect();
        assert_eq!(interval(&next), Some((31.0, 10_000.0)));
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        assert!(vehicle.start_guided(&json!({ "action": "pidTuningMode", "mode": 7 }), 20_200).is_err());
    }

    #[test]
    fn an_actuator_action_configures_one_function_and_waits_for_its_ack() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        let sent = vehicle.start_guided(&json!({ "action": "actuatorAction", "type": 4, "function": 101 }), 50_000).unwrap();
        assert!(sent.iter().any(|b| matches!(decode(b), MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_CONFIGURE_ACTUATOR && c.param1 == 4.0 && c.param5 == 1101.0)));
        assert!(vehicle.start_guided(&json!({ "action": "actuatorAction", "type": 4, "function": 101 }), 50_100).unwrap().is_empty(), "one action in flight at a time");
        let denied = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_CONFIGURE_ACTUATOR, result: MavResult::MAV_RESULT_DENIED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &denied, 50_200_000, 50_200);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        assert!(vehicle.pending_notices.iter().any(|(_, text)| text == "Actuator action command failed"));
        assert!(!vehicle.start_guided(&json!({ "action": "actuatorAction", "type": 5, "function": 101 }), 50_300).unwrap().is_empty(), "the ack frees the next action");
    }

    #[test]
    fn an_actuator_test_sends_the_value_to_function_plus_1000_on_the_autopilot() {
        use mavlink::dialects::ardupilotmega::MavCmd;
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        vehicle.start_guided(&json!({ "action": "actuatorTest", "op": "active", "on": true }), 40_000).unwrap();
        let sent = vehicle.start_guided(&json!({ "action": "actuatorTest", "op": "set", "function": 101, "value": 0.25 }), 40_000).unwrap();
        let test = sent.iter().find_map(|b| match decode(b) { MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_ACTUATOR_TEST => Some(c), _ => None }).expect("an actuator test command");
        assert_eq!((test.param1, test.param2, test.param5, test.target_component), (0.25, 1.0, 1101.0, 1));
        assert!(vehicle.start_guided(&json!({ "action": "actuatorTest", "op": "spin" }), 40_000).is_err());
    }

    #[test]
    fn autotune_polls_each_second_until_the_vehicle_reports_it_done() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let polls = |bytes: &[Vec<u8>]| bytes.iter().filter(|b| matches!(decode(b), MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_DO_AUTOTUNE_ENABLE && c.param1 == 1.0)).count();
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        assert_eq!(polls(&vehicle.start_guided(&json!({ "action": "autotune" }), 30_000).unwrap()), 1);
        assert_eq!(polls(&vehicle.pump_with(30_500, None, 0)), 0);
        assert_eq!(polls(&vehicle.pump_with(31_000, None, 0)), 1, "the request repeats every second while it runs");
        let ack = |progress: u8, result: MavResult| MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_DO_AUTOTUNE_ENABLE, result, progress, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &ack(30, MavResult::MAV_RESULT_IN_PROGRESS), 31_100_000, 31_100);
        assert_eq!(hub.vehicles[&1].autotune.status, "Autotune: roll");
        hub.on_frame(origin(4), &autopilot, &ack(100, MavResult::MAV_RESULT_ACCEPTED), 31_200_000, 31_200);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        assert_eq!(vehicle.autotune.status, "Autotune: Success");
        assert!(vehicle.pending_notices.iter().any(|(_, text)| text == "Autotune successful."));
        assert_eq!(polls(&vehicle.pump_with(33_000, None, 0)), 0, "finished, so polling stops");
    }

    #[test]
    fn an_accepted_bootloader_flash_says_so() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        let mut hub = Hub::default();
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        hub.on_frame(origin(0), &header, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(origin(0), &header, &MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_FLASH_BOOTLOADER, result: MavResult::MAV_RESULT_ACCEPTED, ..Default::default() }), 1, 1);
        assert!(hub.active().unwrap().pending_notices.iter().any(|(_, t)| t == "Bootloader flash succeeded"), "Vehicle::_handleCommandAck");
    }

    #[test]
    fn a_link_another_vehicle_still_uses_is_not_closed_for_a_reboot() {
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(origin(4), &MavHeader { system_id: 2, component_id: 1, sequence: 0 }, &copter_heartbeat(0, false), 0, 0);
        hub.on_frame(origin(5), &MavHeader { system_id: 2, component_id: 1, sequence: 0 }, &copter_heartbeat(0, false), 0, 0);
        assert_eq!(hub.links_of_other_vehicles(1), [4, 5], "LinkInterface disconnects only once no vehicle holds it");
        assert_eq!(hub.links_of_other_vehicles(2), [4]);
    }

    #[test]
    fn a_reboot_closes_the_vehicle_when_accepted_and_says_so_when_not() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let ack = |result: MavResult| MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN, result, ..Default::default() });
        hub.vehicles.get_mut(&1).unwrap().start_guided(&json!({ "action": "reboot" }), 20_000).unwrap();
        hub.on_frame(origin(4), &autopilot, &ack(MavResult::MAV_RESULT_DENIED), 20_100_000, 20_100);
        assert!(hub.active().unwrap().pending_notices.iter().any(|(_, t)| t == "Vehicle reboot failed."));
        hub.vehicles.get_mut(&1).unwrap().start_guided(&json!({ "action": "reboot" }), 21_000).unwrap();
        hub.on_frame(origin(4), &autopilot, &ack(MavResult::MAV_RESULT_ACCEPTED), 21_100_000, 21_100);
        assert_eq!(crate::corelinks::pending_close(), Some((21_100, Some(vec![4]))), "Vehicle::_rebootCommandResultHandler closes the vehicle's own links");
        hub.vehicles.get_mut(&1).unwrap().start_guided(&json!({ "action": "mavlinkCommand", "command": 246, "params": [0.0, 1.0] }), 22_000).unwrap();
        hub.on_frame(origin(4), &autopilot, &ack(MavResult::MAV_RESULT_ACCEPTED), 22_100_000, 22_100);
        assert_eq!(crate::corelinks::pending_close(), Some((21_100, Some(vec![4]))), "a companion reboot sent some other way is not the vehicle rebooting");
    }

    #[test]
    fn an_airframe_change_reboots_only_after_both_writes_are_acknowledged() {
        use mavlink::dialects::ardupilotmega::MavCmd;
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        hub.on_frame(origin(4), &autopilot, &param_value("SYS_AUTOSTART", 3, 1, 4001.0), 10_000_000, 10_000);
        hub.on_frame(origin(4), &autopilot, &param_value("SYS_AUTOCONFIG", 3, 2, 0.0), 10_000_000, 10_000);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        let sent = vehicle.start_guided(&json!({ "action": "changeAutostart", "autostartId": 4001 }), 11_000).unwrap();
        let sets: Vec<(String, f32)> = sent.iter().filter_map(|b| match decode(b) { MavMessage::PARAM_SET(p) => Some((p.param_id.to_str().unwrap().to_string(), p.param_value)), _ => None }).collect();
        assert_eq!(sets, [("SYS_AUTOSTART".to_string(), 4001.0), ("SYS_AUTOCONFIG".to_string(), 1.0)], "both are written even though SYS_AUTOSTART already holds 4001, as forceSetRawValue does");
        let reboots = |bytes: &[Vec<u8>]| bytes.iter().any(|b| matches!(decode(b), MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN && c.param1 == 1.0));
        assert!(!reboots(&vehicle.pump_with(11_500, None, 0)), "no reboot while the writes are unacknowledged");
        hub.on_frame(origin(4), &autopilot, &param_value("SYS_AUTOSTART", 3, 1, 4001.0), 12_000_000, 12_000);
        hub.on_frame(origin(4), &autopilot, &param_value("SYS_AUTOCONFIG", 3, 2, 1.0), 12_000_000, 12_000);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        assert!(!reboots(&vehicle.pump_with(12_700, None, 0)), "the acks start the 800 ms wait");
        assert!(reboots(&vehicle.pump_with(12_800, None, 0)));
        assert!(vehicle.airframe_reboot.is_none());
    }

    #[test]
    fn a_write_waits_for_the_parameter_list_and_refuses_values_the_type_cannot_hold() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &autopilot, &copter_heartbeat(5, false), 0, 0);
        assert_eq!(hub.parameter_request(None, &json!({ "name": "RTL_ALT", "value": 1.0 }), 10).unwrap_err(), "Parameters are still loading.");
        assert_eq!(ParamValue::from_f64(1, 300.0), None);
        assert_eq!(ParamValue::from_f64(1, -1.0), None);
        assert_eq!(ParamValue::from_f64(6, 1.5), None);
        assert_eq!(ParamValue::from_f64(6, f64::NAN), None);
        assert_eq!(ParamValue::from_f64(9, 1e40), None);
        assert_eq!(ParamValue::from_f64(3, 65535.0), Some(ParamValue::U16(65535)));
        assert_eq!(ParamValue::from_f64(9, 1.5), Some(ParamValue::F32(1.5)));
    }

    #[test]
    fn a_replayed_vehicle_never_sends() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        assert!(hub.on_frame(Origin { link: 4, replay: true, v2: true }, &autopilot, &copter_heartbeat(5, false), 0, 0).is_empty());
        assert!(hub.tick(10_000).is_empty());
        assert_eq!(hub.snapshot()["vehicle"]["heartbeats"], 1);
    }

    #[test]
    fn a_silent_vehicle_gets_its_parameter_list_requested_again_after_the_initial_timeout() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &autopilot, &copter_heartbeat(5, false), 0, 0);
        let refused = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &refused, 100_000, 100);
        hub.on_frame(origin(4), &autopilot, &refused, 150_000, 150);
        let opened = hub.on_frame(origin(4), &autopilot, &refused, 200_000, 200);
        let listed = refuse_pack(&mut hub, &autopilot, &opened, 200);
        assert!(matches!(decode(&listed[0].1), MavMessage::PARAM_REQUEST_LIST(_)));
        assert!(hub.tick(5_199).is_empty());
        let again = hub.tick(5_201);
        assert!(matches!(decode(&again[0].1), MavMessage::PARAM_REQUEST_LIST(_)), "the list is asked for again after five seconds");
    }

    #[test]
    #[allow(deprecated)]
    fn a_px4_vehicle_asks_for_its_protocol_version_and_collects_its_standard_modes() {
        use mavlink::dialects::ardupilotmega::{AUTOPILOT_VERSION_DATA, AVAILABLE_MODES_DATA, HEARTBEAT_DATA, MavAutopilot, MavModeFlag, MavStandardMode, MavType, PROTOCOL_VERSION_DATA};
        let autopilot = MavHeader { system_id: 2, component_id: 1, sequence: 0 };
        let mut px4 = HEARTBEAT_DATA::default();
        px4.mavtype = MavType::MAV_TYPE_QUADROTOR;
        px4.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        px4.base_mode = MavModeFlag::from_bits_retain(0x01);
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &autopilot, &MavMessage::HEARTBEAT(px4), 0, 0);
        let after_version = hub.on_frame(origin(4), &autopilot, &MavMessage::AUTOPILOT_VERSION(AUTOPILOT_VERSION_DATA::default()), 100_000, 100);
        assert_eq!(request_of(&after_version[0].1), (512, 300.0));
        let protocol = MavMessage::PROTOCOL_VERSION(PROTOCOL_VERSION_DATA { version: 200, min_version: 100, max_version: 200, ..Default::default() });
        let after_protocol = hub.on_frame(origin(4), &autopilot, &protocol, 200_000, 200);
        assert_eq!(request_of(&after_protocol[0].1), (512, 435.0));
        let mode = |index: u8, name: &str, custom: u32| {
            let mut name_bytes = [0u8; 35];
            name_bytes[..name.len()].copy_from_slice(name.as_bytes());
            MavMessage::AVAILABLE_MODES(AVAILABLE_MODES_DATA { custom_mode: custom, number_modes: 2, mode_index: index, standard_mode: MavStandardMode::MAV_STANDARD_MODE_NON_STANDARD, mode_name: name_bytes.into(), ..Default::default() })
        };
        let second = hub.on_frame(origin(4), &autopilot, &mode(1, "Manual", 65536), 300_000, 300);
        assert_eq!(request_of(&second[0].1), (512, 435.0), "the next mode is requested");
        let metadata = hub.on_frame(origin(4), &autopilot, &mode(2, "Position", 196608), 400_000, 400);
        assert_eq!(request_of(&metadata[0].1), (512, 397.0));
        let modes = hub.snapshot()["vehicle"]["flightModes"].clone();
        assert_eq!(modes.as_array().unwrap().len(), 2);
        assert_eq!(modes[1]["name"], "Position");
        assert_eq!(hub.snapshot()["vehicle"]["maxProtoVersion"], 200);
    }

    fn ftp_request(bytes: &[u8]) -> ftp::Request {
        match decode(bytes) {
            MavMessage::FILE_TRANSFER_PROTOCOL(f) => ftp::Request::decode(&f.payload).unwrap(),
            other => panic!("not an ftp frame: {other:?}"),
        }
    }

    fn ftp_reply(reply: ftp::Request) -> MavMessage {
        use mavlink::dialects::ardupilotmega::FILE_TRANSFER_PROTOCOL_DATA;
        MavMessage::FILE_TRANSFER_PROTOCOL(FILE_TRANSFER_PROTOCOL_DATA { target_network: 0, target_system: 255, target_component: 190, payload: reply.encode() })
    }

    fn serve(files: &BTreeMap<String, Vec<u8>>, request: &ftp::Request) -> ftp::Request {
        let ack = |req_opcode: u8, offset: u32, data: Vec<u8>, burst_complete: bool| ftp::Request { seq: request.seq + 1, session: 9, opcode: ftp::RSP_ACK, req_opcode, burst_complete, offset, data };
        let nak = |req_opcode: u8, code: u8| ftp::Request { seq: request.seq + 1, session: 9, opcode: ftp::RSP_NAK, req_opcode, burst_complete: false, offset: 0, data: vec![code] };
        let open_path = String::from_utf8_lossy(&request.data).to_string();
        let file = files.values().next().cloned().unwrap_or_default();
        match request.opcode {
            ftp::CMD_OPEN_FILE_RO => files.get(&open_path).map(|f| ack(ftp::CMD_OPEN_FILE_RO, 0, (f.len() as u32).to_le_bytes().to_vec(), false)).unwrap_or_else(|| nak(ftp::CMD_OPEN_FILE_RO, 10)),
            ftp::CMD_BURST_READ_FILE | ftp::CMD_READ_FILE => {
                let at = request.offset as usize;
                match at >= file.len() {
                    true => nak(request.opcode, ftp::ERR_EOF),
                    false => ack(request.opcode, request.offset, file[at..].iter().copied().take(ftp::DATA_LEN).collect(), true),
                }
            }
            other => ack(other, 0, Vec::new(), false),
        }
    }

    #[test]
    fn an_ardupilot_loads_its_parameters_and_their_defaults_from_the_parameter_file() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &autopilot, &copter_heartbeat(5, false), 0, 0);
        let refused = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &refused, 1, 1);
        hub.on_frame(origin(4), &autopilot, &refused, 2, 2);
        let mut pack = vec![0x1C, 0x67, 2, 0, 2, 0];
        pack.extend([0x13, 6 << 4, b'R', b'T', b'L', b'_', b'A', b'L', b'T']);
        pack.extend(1500i32.to_le_bytes());
        pack.extend(1000i32.to_le_bytes());
        pack.extend([0x04, (4 << 4) | 4, b'S', b'P', b'E', b'E', b'D']);
        pack.extend(2.5f32.to_le_bytes());
        let files = BTreeMap::from([(params::PACK_URI.to_string(), pack)]);
        let mut pending = hub.on_frame(origin(4), &autopilot, &refused, 3, 3);
        (0..20).for_each(|i| {
            let Some((_, bytes)) = pending.first().cloned() else { return };
            if !matches!(decode(&bytes), MavMessage::FILE_TRANSFER_PROTOCOL(_)) {
                return;
            }
            pending = hub.on_frame(origin(4), &autopilot, &ftp_reply(serve(&files, &ftp_request(&bytes))), 10 + i, 10 + i);
        });
        let vehicle = hub.active().unwrap();
        assert!(vehicle.params.ready(), "the file makes the parameters ready without a PARAM_REQUEST_LIST");
        assert_eq!((vehicle.parameter(1, "RTL_SPEED"), vehicle.parameter_defaults.get("RTL_ALT")), (Some(ParamValue::F32(2.5)), Some(&ParamValue::I32(1000))));
        assert!(!pending.iter().any(|(_, b)| matches!(decode(b), MavMessage::PARAM_REQUEST_LIST(_))));
    }

    #[test]
    fn component_metadata_is_fetched_over_ftp_and_describes_the_parameters() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, COMPONENT_METADATA_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &autopilot, &copter_heartbeat(5, false), 0, 0);
        let refused = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &refused, 1, 1);
        let asked = hub.on_frame(origin(4), &autopilot, &refused, 2, 2);
        assert_eq!(request_of(&asked[0].1), (512, 397.0));
        let mut uri = [0u8; 100];
        let text = b"mftp://etc/extras/component_general.json";
        uri[..text.len()].copy_from_slice(text);
        let metadata = MavMessage::COMPONENT_METADATA(COMPONENT_METADATA_DATA { time_boot_ms: 0, file_crc: 7, uri: uri.into() });
        let general = br#"{"version":1,"metadataTypes":[{"type":1,"uri":"mftp://etc/extras/parameters.json.xz","fileCrc":99}]}"#.to_vec();
        let plain = br#"{"version":1,"parameters":[{"name":"RTL_ALT","type":"float","shortDesc":"Return altitude","units":"m","min":0,"max":8000},{"name":"CAM_{n}_MODE","type":"uint8","shortDesc":"Camera {n} mode"},{"name":"SERIAL1_PROTOCOL","type":"int32","shortDesc":"Serial 1 protocol","rebootRequired":true}]}"#;
        let mut parameters = Vec::new();
        lzma_rs::xz_compress(&mut std::io::Cursor::new(plain.as_slice()), &mut parameters).unwrap();
        let mut files = BTreeMap::new();
        files.insert("/etc/extras/component_general.json".to_string(), general);
        let mut pending = hub.on_frame(origin(4), &autopilot, &metadata, 3, 3);
        let mut parameter_file_opened = false;
        (0..40).for_each(|i| {
            let Some((_, bytes)) = pending.first().cloned() else { return };
            if !matches!(decode(&bytes), MavMessage::FILE_TRANSFER_PROTOCOL(_)) {
                return;
            }
            let request = ftp_request(&bytes);
            if request.opcode == ftp::CMD_OPEN_FILE_RO && String::from_utf8_lossy(&request.data).contains("parameters.json.xz") {
                parameter_file_opened = true;
                files = BTreeMap::from([("/etc/extras/parameters.json.xz".to_string(), parameters.clone())]);
            }
            let reply = serve(&files, &request);
            pending = hub.on_frame(origin(4), &autopilot, &ftp_reply(reply), 10 + i, 10 + i);
        });
        assert!(parameter_file_opened, "the general file named the parameter file, which was fetched next");
        assert!(matches!(decode(&pending[0].1), MavMessage::PARAM_REQUEST_LIST(_)), "after the metadata the connect sequence moves on to the parameters");
        let snapshot = hub.snapshot();
        assert_eq!(snapshot["vehicle"]["componentInformation"], json!({ "types": [1], "parameterMetadata": 3 }));
        let meta = hub.active().unwrap().parameter_meta("RTL_ALT", Some(ParamValue::F32(0.0))).unwrap();
        assert_eq!((meta["shortDescription"].as_str(), meta["units"].as_str(), meta["max"].as_f64()), (Some("Return altitude"), Some("m"), Some(8000.0)));
        assert_eq!(hub.active().unwrap().parameter_meta("CAM_2_MODE", Some(ParamValue::U8(0))).unwrap()["shortDescription"], "Camera 2 mode");
        assert!(hub.active().unwrap().parameter_meta("NOPE", None).is_none());

        let restart = hub.active().unwrap().parameter_meta("SERIAL1_PROTOCOL", Some(ParamValue::I32(0))).unwrap();
        assert_eq!(restart["rebootRequired"], true);
        assert_eq!(restart["vehicleRebootRequired"], true, "the two are served apart because a vehicle reboot and a ground station restart are different asks of the operator");
        assert_eq!(restart["applicationRestartRequired"], false);
        assert_eq!(restart["restartNotices"].as_array().map(Vec::len), Some(1));
        assert_eq!(meta["rebootRequired"], false, "a parameter that takes effect immediately must not ask for a restart, or the notice stops meaning anything");
        assert!(hub.active().unwrap().ftp_seq > 0, "the ftp sequence carries across downloads as it does in the Qt manager");
    }

    #[test]
    fn a_metadata_download_that_never_answers_times_out_and_the_sequence_moves_on() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, COMPONENT_METADATA_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &autopilot, &copter_heartbeat(5, false), 0, 0);
        let refused = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &refused, 1, 1);
        hub.on_frame(origin(4), &autopilot, &refused, 2, 2);
        let mut uri = [0u8; 100];
        uri[..18].copy_from_slice(b"mftp://etc/g.json ");
        let metadata = MavMessage::COMPONENT_METADATA(COMPONENT_METADATA_DATA { time_boot_ms: 0, file_crc: 7, uri: uri.into() });
        let opened = hub.on_frame(origin(4), &autopilot, &metadata, 3, 3);
        assert_eq!(ftp_request(&opened[0].1).opcode, ftp::CMD_OPEN_FILE_RO);
        assert!(hub.tick(1_000).is_empty(), "nothing happens before the one second ack timer");
        let timed_out = hub.tick(1_100);
        let listed: Vec<MavMessage> = refuse_pack(&mut hub, &autopilot, &timed_out, 1_100).into_iter().map(|(_, b)| decode(&b)).collect();
        assert!(matches!(listed.last(), Some(MavMessage::PARAM_REQUEST_LIST(_))), "an unanswered open fails the fetch and the parameters are requested anyway");
        assert!(hub.guided_snapshot(None)["guided"]["errors"].as_array().unwrap().iter().any(|e| e.as_str().unwrap().contains("Download failed")));
    }

    #[test]
    fn a_high_latency_link_refuses_plan_transfers_like_plan_master_controller() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        hub.vehicles.get_mut(&1).unwrap().commands.high_latency = true;
        assert_eq!(hub.mission_request(None, &json!({ "action": "load" }), 10_000), Err(HIGH_LATENCY_DOWNLOAD.to_string()));
        assert_eq!(hub.mission_request(None, &json!({ "plan": "rally", "action": "write", "points": [[47.0, 8.0, 40.0]] }), 10_000), Err(HIGH_LATENCY_UPLOAD.to_string()));
        assert_eq!(hub.write_mission(None, Vec::new(), 10_000), Err(HIGH_LATENCY_UPLOAD.to_string()));
    }

    #[test]
    #[allow(deprecated)]
    fn a_mission_write_serves_the_vehicle_requests_and_becomes_the_known_mission() {
        use mavlink::dialects::ardupilotmega::{MISSION_ACK_DATA, MISSION_REQUEST_INT_DATA, MavMissionResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let items = json!([
            { "frame": 0, "command": 16, "params": [0, 0, 0, 0, 47.0, 8.0, 0] },
            { "frame": 3, "command": 16, "params": [0, 0, 0, 0, 47.1, 8.1, 50] },
            { "frame": 2, "command": 177, "params": [1, 2, 0, 0, 0, 0, 0] }
        ]);
        let started = hub.mission_request(None, &json!({ "action": "write", "items": items }), 10_000).unwrap();
        assert!(matches!(decode(&started[0].1), MavMessage::MISSION_COUNT(c) if c.count == 3), "ArduPilot is sent the planned home as item zero, as Qt's PlanManager sends it");
        assert!(hub.mission_request(None, &json!({ "action": "load" }), 10_000).is_err(), "one transfer at a time");
        let request = |seq: u16| MavMessage::MISSION_REQUEST_INT(MISSION_REQUEST_INT_DATA { seq, target_system: 255, target_component: 190, ..Default::default() });
        let first = hub.on_frame(origin(4), &autopilot, &request(0), 10_100_000, 10_100);
        assert!(matches!(decode(&first[0].1), MavMessage::MISSION_ITEM_INT(i) if i.seq == 0 && i.current == 1 && i.x == 470000000));
        let second = hub.on_frame(origin(4), &autopilot, &request(1), 10_150_000, 10_150);
        assert!(matches!(decode(&second[0].1), MavMessage::MISSION_ITEM_INT(i) if i.seq == 1 && i.x == 471000000));
        let third = hub.on_frame(origin(4), &autopilot, &request(2), 10_200_000, 10_200);
        assert!(matches!(decode(&third[0].1), MavMessage::MISSION_ITEM_INT(i) if i.seq == 2 && i.param1 == 1.0), "with home sent, the jump target keeps its sequence");
        hub.on_frame(origin(4), &autopilot, &MavMessage::MISSION_ACK(MISSION_ACK_DATA { target_system: 255, target_component: 190, mavtype: MavMissionResult::MAV_MISSION_ACCEPTED, ..Default::default() }), 10_300_000, 10_300);
        let mission = hub.active().unwrap().mission_snapshot()["mission"].clone();
        assert_eq!((mission["inProgress"].as_bool(), mission["count"].as_u64(), mission["error"].is_null()), (Some(false), Some(3), true));
        assert!(hub.tick(12_000).is_empty());
        assert!(hub.mission_request(None, &json!({ "action": "write", "items": [{ "frame": 0 }] }), 12_000).is_err());
        assert!(hub.mission_request(None, &json!({ "action": "write", "items": [{ "frame": 0, "command": 16, "params": [0, "x", 0, 0, 47, 8, 50] }] }), 12_000).is_err(), "a non-numeric param is refused before anything is sent");
        assert!(hub.mission_request(None, &json!({ "action": "write", "items": [{ "frame": 0, "command": 65000, "params": [0, 0, 0, 0, 47, 8, 50] }] }), 12_000).is_err(), "a command the dialect cannot name is refused");
        assert!(hub.mission_request(None, &json!({ "action": "write", "items": [] }), 12_000).is_err());
        let again = hub.mission_request(None, &json!({ "action": "write", "items": [{ "frame": 0, "command": 16, "params": [0, 0, 0, 0, 47.0, 8.0, 0] }, { "frame": 3, "command": 16, "params": [0, 0, 0, 0, 47.2, 8.2, 60] }] }), 13_000).unwrap();
        assert!(matches!(decode(&again[0].1), MavMessage::MISSION_COUNT(c) if c.count == 2));
        use mavlink::dialects::ardupilotmega::MISSION_REQUEST_DATA;
        let plain = hub.on_frame(origin(4), &autopilot, &MavMessage::MISSION_REQUEST(MISSION_REQUEST_DATA { seq: 0, target_system: 0, target_component: 190, ..Default::default() }), 13_100_000, 13_100);
        assert!(matches!(decode(&plain[0].1), MavMessage::MISSION_ITEM_INT(i) if i.seq == 0), "the float request form and a broadcast target are served too");
    }

    #[test]
    fn a_fence_and_rally_write_use_their_own_plan_types_and_read_back_as_shapes() {
        use mavlink::dialects::ardupilotmega::{MISSION_ACK_DATA, MISSION_REQUEST_INT_DATA, MavMissionResult, MavMissionType};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let fence = json!({ "plan": "fence", "action": "write", "polygons": [{ "inclusion": true, "vertices": [[47.0, 8.0], [47.1, 8.0], [47.1, 8.1]] }], "circles": [{ "inclusion": false, "center": [47.05, 8.05], "radius": 90 }], "breachReturn": [47.02, 8.02, 30] });
        let started = hub.mission_request(None, &fence, 20_000).unwrap();
        assert!(matches!(decode(&started[0].1), MavMessage::MISSION_COUNT(c) if c.count == 5 && c.mission_type as u8 == 1));
        let request = |plan: u8, seq: u16| MavMessage::MISSION_REQUEST_INT(MISSION_REQUEST_INT_DATA { seq, target_system: 255, target_component: 190, mission_type: MavMissionType::from_u8(plan).unwrap() });
        let ack = |plan: u8| MavMessage::MISSION_ACK(MISSION_ACK_DATA { target_system: 255, target_component: 190, mavtype: MavMissionResult::MAV_MISSION_ACCEPTED, mission_type: MavMissionType::from_u8(plan).unwrap(), opaque_id: 0 });
        (0..5u16).for_each(|seq| {
            let served = hub.on_frame(origin(4), &autopilot, &request(1, seq), 20_100_000 + seq as u64 * 1000, 20_100 + seq as u64);
            assert!(matches!(decode(&served[0].1), MavMessage::MISSION_ITEM_INT(i) if i.seq == seq && i.mission_type as u8 == 1));
        });
        hub.on_frame(origin(4), &autopilot, &ack(1), 20_200_000, 20_200);
        let plans = hub.active().unwrap().mission_snapshot();
        assert_eq!(plans["fence"]["polygons"][0]["vertices"][2], json!([47.1, 8.1]));
        assert_eq!(plans["fence"]["circles"][0]["radius"], 90.0);
        assert_eq!(plans["fence"]["breachReturn"], json!([47.02, 8.02, 30.0]));
        let rally = hub.mission_request(None, &json!({ "plan": "rally", "action": "write", "points": [[47.3, 8.3, 40.0]] }), 21_000).unwrap();
        assert!(matches!(decode(&rally[0].1), MavMessage::MISSION_COUNT(c) if c.count == 1 && c.mission_type as u8 == 2));
        hub.on_frame(origin(4), &autopilot, &request(2, 0), 21_100_000, 21_100);
        hub.on_frame(origin(4), &autopilot, &ack(2), 21_200_000, 21_200);
        assert_eq!(hub.active().unwrap().mission_snapshot()["rally"]["points"], json!([[47.3, 8.3, 40.0]]));
        assert!(hub.mission_request(None, &json!({ "plan": "fence", "action": "write", "polygons": [{ "vertices": [[1, "x"]] }] }), 22_000).is_err());
        assert!(hub.mission_request(None, &json!({ "plan": "fence", "action": "write", "polygons": [{ "vertices": [[47.0, 8.0], [47.1, 8.0]] }] }), 22_000).is_err(), "two vertices are not a polygon");
        assert!(hub.mission_request(None, &json!({ "plan": "fence", "action": "write", "breachReturn": [47.0, 8.0] }), 22_000).is_err(), "a breach return point needs its altitude");
        assert!(hub.mission_request(None, &json!({ "plan": "walls", "action": "load" }), 22_000).is_err());
    }

    #[test]
    fn home_follows_the_ground_station_only_when_asked_and_once_the_vehicle_has_a_position() {
        use mavlink::dialects::ardupilotmega::{GLOBAL_POSITION_INT_DATA, MavCmd};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        assert!(hub.home_follows_gcs(true, (47.5, 8.5, 400.0), 40_000).is_empty(), "no vehicle position, nothing to compare against");
        let position = MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA { lat: 474000000, lon: 85000000, alt: 500_000, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &position, 41_000_000, 41_000);
        assert!(hub.home_follows_gcs(false, (47.5, 8.5, 400.0), 41_100).is_empty());
        let sent = hub.home_follows_gcs(true, (47.5, 8.5, 400.0), 41_200);
        assert!(sent.iter().any(|(_, b)| matches!(decode(b), MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_DO_SET_HOME && c.param5 == 47.5 && c.param7 == 400.0)), "{sent:?}");
    }

    struct NullBackend;
    impl crate::router::Backend for NullBackend {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_worsening_battery_is_announced_once_per_level_and_a_fence_breach_at_most_every_three_seconds() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        let spoken = || crate::speech::speech_view(&NullBackend, &[]).get("lines").and_then(Value::as_array).cloned().unwrap_or_default().iter().filter_map(|l| l["text"].as_str().map(str::to_string)).collect::<Vec<_>>();
        let count = |needle: &str| spoken().iter().filter(|t| t.contains(needle)).count();
        vehicle.announce_battery(8, BATTERY_LOW);
        assert_eq!(count("battery  level low"), 0, "Vehicle takes the first state a pack reports as already announced, so a pack plugged in low is not called out");
        vehicle.announce_battery(7, BATTERY_OK);
        vehicle.announce_battery(7, BATTERY_LOW);
        vehicle.announce_battery(7, BATTERY_LOW);
        assert_eq!(count("battery  level low"), 1, "the same level is said once");
        vehicle.announce_battery(7, BATTERY_CRITICAL);
        assert_eq!(count("battery  level is critical"), 1);
        vehicle.announce_battery(7, BATTERY_OK);
        vehicle.announce_battery(7, BATTERY_LOW);
        assert_eq!(count("battery  level low"), 2, "recovering to OK re-arms the warnings");
        let before = count("maximum altitude fence breached");
        vehicle.announce_fence(true, FENCE_BREACH_MAXALT, 10_000);
        vehicle.announce_fence(true, FENCE_BREACH_MAXALT, 11_000);
        vehicle.announce_fence(true, FENCE_BREACH_MAXALT, 14_000);
        assert_eq!(count("maximum altitude fence breached") - before, 2);
    }

    #[test]
    fn a_failed_transfer_is_announced_per_plan_as_vehicle_does() {
        assert_eq!(transfer_failed(plantransfer::PLAN_MISSION, "x"), "Mission transfer failed. Error: x");
        assert_eq!(transfer_failed(plantransfer::PLAN_FENCE, "x"), "GeoFence transfer failed. Error: x");
        assert_eq!(transfer_failed(plantransfer::PLAN_RALLY, "x"), "Rally Point transfer failed. Error: x");
    }

    #[test]
    fn status_texts_are_spoken_by_severity_or_a_leading_hash() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let vehicle = hub.vehicles.get_mut(&1).unwrap();
        let status = |severity: u8, text: &str| StatusText { component: 1, severity, text: text.into() };
        assert_eq!(vehicle.spoken_status(&status(4, "Low battery"), 0).as_deref(), Some("Low battery"), "warning is at or above notice");
        assert_eq!(vehicle.spoken_status(&status(6, "Waypoint 3 reached"), 0), None, "info is not read aloud");
        assert_eq!(vehicle.spoken_status(&status(6, "#Payload released"), 0).as_deref(), Some("Payload released"), "a leading hash asks for speech");
        vehicle.note_prearm("PreArm: RC not calibrated", 4, 1, 1_000);
        assert_eq!(vehicle.spoken_status(&status(2, "PreArm: RC not calibrated"), 5_000), None, "the same PreArm within ten seconds is not repeated");
        assert!(vehicle.spoken_status(&status(2, "PreArm: RC not calibrated"), 12_000).is_some());
        vehicle.note_prearm("Preflight Fail: Accel uncalibrated", 2, 1, 20_000);
        assert_eq!(vehicle.spoken_status(&status(2, "Preflight Fail: Accel uncalibrated"), 25_000), None, "Vehicle::_handleStatusText limits PX4 preflight repeats the same way");
    }

    #[test]
    fn an_arm_status_starts_the_remote_id_broadcast_and_silence_stops_it() {
        use mavlink::dialects::ardupilotmega::{MavOdidArmStatus, OPEN_DRONE_ID_ARM_STATUS_DATA};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        hub.vehicles.get_mut(&1).unwrap().streams_watched_ms = None;
        let settings = remoteid::Settings { region: remoteid::REGION_FAA, operator_id: "FIN87astrdge12k8".into(), operator_id_type: 0, operator_id_valid: false, send_operator_id: true, basic_id: "1234".into(), basic_id_type: 1, basic_id_ua_type: 2, send_basic_id: true, send_self_id: false, self_id_type: 0, self_id_free: "Survey".into(), self_id_emergency: "Emergency".into(), self_id_extended: "Extended".into(), location_type: remoteid::LOCATION_LIVE, classification_type: 0, latitude_fixed: 0.0, longitude_fixed: 0.0, altitude_fixed: 0.0, category_eu: 0, class_eu: 0 };
        hub.set_remote_inputs(settings, GcsFix { valid: true, latitude: 47.5, longitude: 8.5, altitude: 400.0, age_ms: 100 }, 30_000);
        let status = MavMessage::OPEN_DRONE_ID_ARM_STATUS(OPEN_DRONE_ID_ARM_STATUS_DATA { status: MavOdidArmStatus::MAV_ODID_ARM_STATUS_GOOD_TO_ARM, error: mavout::chars("") });
        assert!(hub.on_frame(origin(4), &MavHeader { system_id: 1, component_id: 236, sequence: 0 }, &status, (remoteid::EPOCH_2019_S + 60) * 1_000_000, 30_000).is_empty(), "the first broadcast waits one second, as the Qt timer does");
        let remote = hub.remote_snapshot(None)["remoteId"].clone();
        assert_eq!((remote["available"].as_bool(), remote["commsGood"].as_bool(), remote["armStatusGood"].as_bool(), remote["sending"].as_bool()), (Some(true), Some(true), Some(true), Some(true)));
        assert!(hub.tick_with(30_900, remoteid::EPOCH_2019_S + 60).is_empty());
        let first: Vec<MavMessage> = hub.tick_with(31_000, remoteid::EPOCH_2019_S + 61).into_iter().map(|(_, b)| decode(&b)).collect();
        assert!(matches!(first[0], MavMessage::OPEN_DRONE_ID_SYSTEM(ref m) if m.operator_latitude == 475000000 && m.target_system == 1 && m.target_component == 1 && m.timestamp == 61));
        assert!(matches!(first[1], MavMessage::OPEN_DRONE_ID_BASIC_ID(_)));
        assert!(matches!(first[2], MavMessage::OPEN_DRONE_ID_OPERATOR_ID(_)));
        assert!(hub.tick_with(31_900, remoteid::EPOCH_2019_S + 61).is_empty(), "one second between broadcasts");
        hub.remote_request(None, &json!({ "emergency": true })).unwrap();
        let emergency: Vec<MavMessage> = hub.tick_with(32_100, remoteid::EPOCH_2019_S + 62).into_iter().map(|(_, b)| decode(&b)).collect();
        assert!(emergency.iter().any(|m| matches!(m, MavMessage::OPEN_DRONE_ID_SELF_ID(d) if d.description_type as u32 == remoteid::SELF_ID_EMERGENCY)));
        hub.remote_request(None, &json!({ "emergency": false })).unwrap();
        hub.on_frame(origin(4), &MavHeader { system_id: 1, component_id: 236, sequence: 0 }, &status, (remoteid::EPOCH_2019_S + 62) * 1_000_000, 32_200);
        let cleared: Vec<MavMessage> = hub.tick_with(33_100, remoteid::EPOCH_2019_S + 63).into_iter().map(|(_, b)| decode(&b)).collect();
        assert!(cleared.iter().any(|m| matches!(m, MavMessage::OPEN_DRONE_ID_SELF_ID(d) if d.description_type as u32 == 0)), "once declared, the self id keeps going after the emergency is cleared");
        hub.on_frame(origin(4), &MavHeader { system_id: 1, component_id: 236, sequence: 0 }, &status, (remoteid::EPOCH_2019_S + 65) * 1_000_000, 35_500);
        let stale: Vec<MavMessage> = hub.tick_with(36_600, remoteid::EPOCH_2019_S + 66).into_iter().map(|(_, b)| decode(&b)).collect();
        assert!(matches!(stale[0], MavMessage::OPEN_DRONE_ID_SYSTEM(ref m) if m.operator_latitude == 0), "a fix older than five seconds is sent as unknown");
        assert_eq!(hub.remote_snapshot(None)["remoteId"]["gcsGpsGood"], false);
        assert!(hub.tick_with(38_800, remoteid::EPOCH_2019_S + 68).is_empty(), "silence for two and a half seconds stops the broadcast");
        assert_eq!(hub.remote_snapshot(None)["remoteId"]["commsGood"], false);
        assert!(hub.remote_request(None, &json!({})).is_err());
    }

    #[test]
    fn without_any_inputs_the_system_message_still_goes_out_with_an_unknown_position() {
        use mavlink::dialects::ardupilotmega::{MavOdidArmStatus, OPEN_DRONE_ID_ARM_STATUS_DATA};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &autopilot);
        let status = MavMessage::OPEN_DRONE_ID_ARM_STATUS(OPEN_DRONE_ID_ARM_STATUS_DATA { status: MavOdidArmStatus::MAV_ODID_ARM_STATUS_GOOD_TO_ARM, error: mavout::chars("") });
        hub.on_frame(origin(4), &autopilot, &status, 40_000_000, 40_000);
        let sent: Vec<MavMessage> = hub.tick_with(41_000, remoteid::EPOCH_2019_S).into_iter().map(|(_, b)| decode(&b)).collect();
        assert_eq!(sent.len(), 1);
        assert!(matches!(sent[0], MavMessage::OPEN_DRONE_ID_SYSTEM(ref m) if m.operator_latitude == 0 && m.operator_altitude_geo == -1000.0));
    }

    #[test]
    fn a_px4_log_streams_into_a_file_and_a_denied_start_removes_it() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, HEARTBEAT_DATA, LOGGING_DATA_ACKED_DATA, LOGGING_DATA_DATA, MavAutopilot, MavCmd, MavModeFlag, MavResult, MavType};
        let autopilot = MavHeader { system_id: 3, component_id: 1, sequence: 0 };
        let mut px4 = HEARTBEAT_DATA::default();
        px4.mavtype = MavType::MAV_TYPE_QUADROTOR;
        px4.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        px4.base_mode = MavModeFlag::from_bits_retain(0x01);
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &autopilot, &MavMessage::HEARTBEAT(px4.clone()), 0, 0);
        let dir = std::env::temp_dir().join(format!("groundstation-log-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let request = json!({ "action": "start", "path": dir.to_string_lossy(), "extension": "ulg" });
        let started = hub.log_request(Some(3), &request, 1_000).unwrap();
        assert!(matches!(decode(&started[0].1), MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_LOGGING_START));
        let file = hub.snapshot()["vehicle"]["log"]["file"].as_str().unwrap().to_string();
        assert!(file.starts_with(&dir.to_string_lossy().to_string()) && file.ends_with(".ulg") && file.contains("/003-"));
        assert!(hub.log_request(Some(3), &request, 1_100).is_err(), "one log at a time");
        let accepted = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_LOGGING_START, result: MavResult::MAV_RESULT_ACCEPTED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &accepted, 1_200_000, 1_200);
        let mut chunk = [0u8; 249];
        let header = [b'U', b'L', b'o', b'g', 0x01, 0x12, 0x35, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        chunk[..16].copy_from_slice(&header);
        chunk[16..21].copy_from_slice(&[2, 0, b'I', 7, 7]);
        let data = MavMessage::LOGGING_DATA(LOGGING_DATA_DATA { sequence: 0, target_system: 255, target_component: 190, length: 21, first_message_offset: 0, data: chunk });
        assert!(hub.on_frame(origin(4), &autopilot, &data, 1_300_000, 1_300).is_empty());
        let acked = MavMessage::LOGGING_DATA_ACKED(LOGGING_DATA_ACKED_DATA { sequence: 1, target_system: 255, target_component: 190, length: 5, first_message_offset: 0, data: { let mut c = [0u8; 249]; c[..5].copy_from_slice(&[2, 0, b'I', 8, 8]); c } });
        let replied = hub.on_frame(origin(4), &autopilot, &acked, 1_400_000, 1_400);
        assert!(matches!(decode(&replied[0].1), MavMessage::LOGGING_ACK(a) if a.sequence == 1 && a.target_system == 3));
        assert_eq!(hub.snapshot()["vehicle"]["log"]["bytes"], 26);
        let stopped = hub.log_request(Some(3), &json!({ "action": "stop" }), 1_500).unwrap();
        assert!(matches!(decode(&stopped[0].1), MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_LOGGING_STOP));
        assert_eq!(std::fs::metadata(&file).unwrap().len(), 26);
        assert_eq!(hub.snapshot()["vehicle"]["log"]["running"], false);
        assert_eq!(hub.snapshot()["vehicle"]["log"]["file"].as_str(), Some(file.as_str()), "the finished file stays known so a head can upload it");
        hub.log_request(Some(3), &json!({ "action": "start" }), 2_000).unwrap();
        let denied_file = hub.snapshot()["vehicle"]["log"]["file"].as_str().unwrap().to_string();
        let denied = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_LOGGING_START, result: MavResult::MAV_RESULT_DENIED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &denied, 2_100_000, 2_100);
        assert!(!std::path::Path::new(&denied_file).exists(), "a denied log is deleted as the Qt manager deletes it");
        assert_eq!(hub.snapshot()["vehicle"]["log"]["denied"], true);
        assert!(hub.log_request(Some(3), &json!({ "action": "start" }), 2_200).is_err(), "a denial sticks for the session");
        let mut copter = Hub::default();
        connect_copter(&mut copter, &MavHeader { system_id: 1, component_id: 1, sequence: 0 });
        assert!(copter.log_request(None, &json!({ "action": "start", "path": dir.to_string_lossy() }), 3_000).is_err(), "log streaming is a PX4 feature");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn auto_start_follows_the_arming_edge_on_px4() {
        use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavAutopilot, MavCmd, MavModeFlag, MavType};
        let autopilot = MavHeader { system_id: 3, component_id: 1, sequence: 0 };
        let heartbeat = |armed: bool| {
            let mut h = HEARTBEAT_DATA::default();
            h.mavtype = MavType::MAV_TYPE_QUADROTOR;
            h.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
            h.base_mode = MavModeFlag::from_bits_retain(if armed { 0x81 } else { 0x01 });
            MavMessage::HEARTBEAT(h)
        };
        let dir = std::env::temp_dir().join(format!("groundstation-log-auto-{}", std::process::id()));
        let mut hub = Hub::default();
        hub.log_request(None, &json!({ "autoStart": true }), 0).unwrap();
        hub.on_frame(origin(4), &MavHeader { system_id: 9, component_id: 1, sequence: 0 }, &heartbeat(false), 0, 0);
        hub.on_frame(origin(4), &MavHeader { system_id: 9, component_id: 1, sequence: 0 }, &heartbeat(true), 500_000, 500);
        assert!(hub.snapshot_of(Some(9))["vehicle"]["log"]["error"].as_str().unwrap().contains("path is required"), "auto start without a path says so instead of failing silently");
        hub.log_request(None, &json!({ "path": dir.to_string_lossy(), "extension": ".ulg" }), 600).unwrap();
        hub.on_frame(origin(4), &autopilot, &heartbeat(false), 0, 0);
        let armed: Vec<MavMessage> = hub.on_frame(origin(4), &autopilot, &heartbeat(true), 1_000_000, 1_000).into_iter().map(|(_, b)| decode(&b)).collect();
        assert!(armed.iter().any(|m| matches!(m, MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_LOGGING_START)), "arming starts the log when auto start is on");
        assert_eq!(hub.snapshot_of(Some(3))["vehicle"]["log"]["running"], true);
        let disarmed: Vec<MavMessage> = hub.on_frame(origin(4), &autopilot, &heartbeat(false), 2_000_000, 2_000).into_iter().map(|(_, b)| decode(&b)).collect();
        assert!(disarmed.iter().any(|m| matches!(m, MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_LOGGING_STOP)));
        assert_eq!(hub.snapshot_of(Some(3))["vehicle"]["log"]["running"], false);
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn a_px4_accel_calibration_runs_from_the_status_texts_through_the_view() {
        use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavAutopilot, MavCmd, MavType, STATUSTEXT_DATA};
        let autopilot = MavHeader { system_id: 3, component_id: 1, sequence: 0 };
        let mut px4 = HEARTBEAT_DATA::default();
        px4.mavtype = MavType::MAV_TYPE_QUADROTOR;
        px4.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &autopilot, &MavMessage::HEARTBEAT(px4), 0, 0);
        let started = hub.calibrate_request(None, &json!({ "action": "start", "type": "accelerometer" }), 1_000).unwrap();
        let MavMessage::COMMAND_LONG(command) = decode(&started[0].1) else { panic!() };
        assert_eq!((command.command, command.param5, command.target_system), (MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION, 1.0, 3));
        assert!(hub.calibrate_request(None, &json!({ "action": "start", "type": "compass" }), 1_100).is_err());
        let say = |hub: &mut Hub, text: &str| {
            let mut data = STATUSTEXT_DATA::default();
            data.text = crate::mavout::chars(text);
            hub.on_frame(origin(4), &autopilot, &MavMessage::STATUSTEXT(data), 0, 1_200)
        };
        say(&mut hub, "[cal] calibration started: 2 accel");
        say(&mut hub, "[cal] down orientation detected");
        let view = hub.calibration_snapshot(None);
        assert_eq!(view["calibration"]["running"], "accelerometer");
        assert_eq!(view["calibration"]["sides"][0]["stage"], "inProgress");
        assert!(view["calibration"]["showOrientations"].as_bool().unwrap());
        let cancelled = hub.calibrate_request(None, &json!({ "action": "cancel" }), 1_300).unwrap();
        let MavMessage::COMMAND_LONG(stop) = decode(&cancelled[0].1) else { panic!() };
        assert_eq!([stop.param1, stop.param2, stop.param3, stop.param4, stop.param5, stop.param6, stop.param7], [0.0; 7], "a stop is the calibration command with every parameter zero");
        say(&mut hub, "[cal] calibration cancelled");
        let after = hub.calibration_snapshot(None);
        assert_eq!((after["calibration"]["running"].as_str(), after["calibration"]["outcome"].as_str()), (None, Some("cancelled")));
        assert!(hub.calibrate_request(None, &json!({ "action": "next" }), 1_400).is_err(), "PX4 has no next step");
        assert!(hub.calibrate_request(None, &json!({ "action": "start", "type": "pressure" }), 1_500).is_err(), "pressure is an ArduPilot routine");
        assert!(hub.calibrate_request(Some(9), &json!({ "action": "start", "type": "gyro" }), 1_600).is_err(), "a vehicle the core never heard cannot be calibrated");
    }

    #[test]
    fn an_apm_accel_calibration_answers_the_vehicles_position_prompts() {
        use mavlink::dialects::ardupilotmega::{COMMAND_LONG_DATA, MavCmd};
        let apm = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        connect_copter(&mut hub, &apm);
        hub.calibrate_request(Some(1), &json!({ "action": "start", "type": "accelerometer" }), 2_000).unwrap();
        let prompt = MavMessage::COMMAND_LONG(COMMAND_LONG_DATA { command: MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS, param1: 1.0, ..Default::default() });
        assert!(hub.on_frame(origin(4), &apm, &prompt, 0, 2_100).is_empty());
        assert_eq!(hub.calibration_snapshot(Some(1))["calibration"]["sides"][0]["stage"], "inProgress");
        let acked = hub.calibrate_request(Some(1), &json!({ "action": "next" }), 2_200).unwrap();
        assert!(matches!(decode(&acked[0].1), MavMessage::COMMAND_ACK(a) if a.command == MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS), "next answers the prompt with the ack ArduPilot advances on, which ignores the command field");
        let done = MavMessage::COMMAND_LONG(COMMAND_LONG_DATA { command: MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS, param1: 16_777_215.0, ..Default::default() });
        hub.on_frame(origin(4), &apm, &done, 0, 2_300);
        assert_eq!(hub.calibration_snapshot(Some(1))["calibration"]["outcome"], "success");
    }
    #[test]
    fn a_relayed_adsb_contact_reaches_the_traffic_module() {
        use mavlink::dialects::ardupilotmega::{AdsbAltitudeType, AdsbEmitterType, AdsbFlags, ADSB_VEHICLE_DATA};
        *crate::adsb::lock() = crate::adsb::Traffic::default();
        let mut hub = Hub::default();
        let relayed = MavMessage::ADSB_VEHICLE(ADSB_VEHICLE_DATA {
            ICAO_address: 0xC0FFEE,
            lat: 474_000_000,
            lon: 85_000_000,
            altitude: 300_000,
            heading: 9_000,
            hor_velocity: 5_000,
            ver_velocity: 0,
            flags: AdsbFlags::ADSB_FLAGS_VALID_COORDS | AdsbFlags::ADSB_FLAGS_VALID_ALTITUDE,
            squawk: 0,
            altitude_type: AdsbAltitudeType::ADSB_ALTITUDE_TYPE_GEOMETRIC,
            callsign: mavlink::types::CharArray::from("RELAY  "),
            emitter_type: AdsbEmitterType::ADSB_EMITTER_TYPE_LIGHT,
            tslc: 1,
        });
        hub.on_frame(origin(0), &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &relayed, 1, 1);
        assert_eq!(crate::adsb::lock().count(), 1, "adsb::on_message had no caller anywhere, so a vehicle relaying traffic over MAVLink reached the module by no path at all - its SBS-1 feed opens its own socket, which is why the view still answered and the gap stayed invisible");
        *crate::adsb::lock() = crate::adsb::Traffic::default();
    }

    #[test]
    fn a_chunked_message_whose_tail_never_arrives_is_flushed_rather_than_held() {
        use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavAutopilot, MavType, STATUSTEXT_DATA};
        let autopilot = MavHeader { system_id: 5, component_id: 1, sequence: 0 };
        let mut px4 = HEARTBEAT_DATA::default();
        px4.mavtype = MavType::MAV_TYPE_QUADROTOR;
        px4.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        let mut hub = Hub::default();
        hub.on_frame(origin(4), &autopilot, &MavMessage::HEARTBEAT(px4), 0, 0);
        let mut chunk = STATUSTEXT_DATA::default();
        chunk.id = 7;
        chunk.chunk_seq = 0;
        chunk.text = crate::mavout::chars(&"a message whose tail the vehicle never sent u".chars().chain("pxxxx".chars()).collect::<String>());
        hub.on_frame(origin(4), &autopilot, &MavMessage::STATUSTEXT(chunk), 0, 1_000);
        assert_eq!(hub.snapshot()["vehicle"]["messages"].as_array().map(|m| m.len()), Some(0), "an unterminated chunk is held while its tail might still arrive");
        hub.tick(1_000 + CHUNKED_TEXT_TIMEOUT_MS - 1);
        assert_eq!(hub.snapshot()["vehicle"]["messages"].as_array().map(|m| m.len()), Some(0));
        hub.tick(1_000 + CHUNKED_TEXT_TIMEOUT_MS);
        let messages = hub.snapshot()["vehicle"]["messages"].as_array().cloned().unwrap_or_default();
        assert_eq!(messages.len(), 1, "after the timeout the operator sees what did arrive, as the Qt handler shows it");
        assert!(messages[0]["text"].as_str().unwrap().starts_with("a message whose tail"));
        hub.tick(9_000);
        assert_eq!(hub.snapshot()["vehicle"]["messages"].as_array().map(|m| m.len()), Some(1), "the flush happens once, not on every tick");
        let piece = |id: u16, seq: u8, text: &str| {
            let mut chunk = STATUSTEXT_DATA::default();
            chunk.id = id;
            chunk.chunk_seq = seq;
            chunk.text = crate::mavout::chars(text);
            MavMessage::STATUSTEXT(chunk)
        };
        hub.on_frame(origin(4), &autopilot, &piece(9, 0, &"the first half of a long message that fills the fie".chars().take(50).collect::<String>()), 0, 10_000);
        hub.on_frame(origin(4), &autopilot, &piece(9, 1, " and the second half"), 0, 10_100);
        let joined = hub.snapshot()["vehicle"]["messages"].as_array().cloned().unwrap_or_default();
        assert_eq!(joined.len(), 2, "the two chunks are one message, not two");
        assert!(joined[0]["text"].as_str().unwrap().ends_with(" and the second half"), "the chunk id and sequence reach the handler, so the halves are joined rather than shown apart");
    }
}
