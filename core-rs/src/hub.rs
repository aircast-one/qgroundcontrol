use mavlink::{MavHeader, Message};
use mavlink::dialects::ardupilotmega::MavMessage;
use serde_json::{Value, json};
use std::collections::BTreeMap;
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
use crate::standardmodes::{self, AvailableMode, FlightMode, MSG_AVAILABLE_MODES, StandardModes};
use crate::transport::LinkId;
use crate::sensorfacts::{DistanceSensorFacts, EstimatorStatusFacts, LocalPositionFacts, TemperatureFacts, WindFacts};
use crate::statustext::{Handler, StatusText};
use crate::sysstatus::SysStatusSensors;
use crate::vehiclefacts::VehicleFacts;

pub const TYPE_GCS: u8 = 6;
pub const TYPE_ONBOARD_CONTROLLER: u8 = 18;
pub const TYPE_GIMBAL: u8 = 26;
pub const TYPE_ADSB: u8 = 27;
pub const COMP_AUTOPILOT1: u8 = 1;
pub const AUTOPILOT_INVALID: u8 = 8;
pub const ARMED_FLAG: u8 = 128;
pub const CUSTOM_MODE_FLAG: u8 = 1;
const MAX_MESSAGES: usize = 200;
pub const CONNECTION_LOST_US: u64 = 3_500_000;
pub const RESULT_UNSUPPORTED: u8 = 3;
pub const MINIMUM_TAKEOFF_ALTITUDE: f64 = 2.5;
pub const MAX_ERRORS: usize = 10;
pub const PROTO_MAVLINK2: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub link: LinkId,
    pub replay: bool,
    pub v2: bool,
}

#[derive(Debug)]
struct PlanSlot {
    transfer: Transfer,
    due: Option<u64>,
    progress: f64,
    error: Option<String>,
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
    pub batteries: Batteries,
    pub facts: VehicleFacts,
    pub wind: WindFacts,
    pub temperature: TemperatureFacts,
    pub distance: DistanceSensorFacts,
    pub local: LocalPositionFacts,
    pub estimator: EstimatorStatusFacts,
    pub sensors: SysStatusSensors,
    pub status_text: Handler,
    pub recent: Vec<StatusText>,
    pub by_name: BTreeMap<String, u64>,
    pub capabilities: u64,
    pub home_altitude: Option<f64>,
    pub home: Option<(f64, f64, f64)>,
    pub reposition_supported: Option<bool>,
    pub errors: Vec<String>,
    pub connection_lost: bool,
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
    metadata_types: BTreeMap<u8, Uris>,
    parameter_metadata: Option<ComponentParameters>,
    fetch: Option<Fetch>,
    ftp_due: Option<u64>,
    ftp_seq: u16,
    plans: [PlanSlot; 3],
}

#[derive(Debug)]
struct Fetch {
    kind: u8,
    uri: String,
    download: Download,
    started_ms: u64,
    progress: f64,
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
            batteries: Batteries::default(),
            facts: VehicleFacts::for_vehicle(id, component),
            wind: WindFacts::default(),
            temperature: TemperatureFacts::default(),
            distance: DistanceSensorFacts::default(),
            local: LocalPositionFacts::default(),
            estimator: EstimatorStatusFacts::default(),
            sensors: SysStatusSensors::default(),
            status_text: Handler::default(),
            recent: Vec::new(),
            by_name: BTreeMap::new(),
            capabilities: 0,
            home_altitude: None,
            home: None,
            reposition_supported: None,
            errors: Vec::new(),
            connection_lost: false,
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
            metadata_types: BTreeMap::new(),
            parameter_metadata: None,
            fetch: None,
            ftp_due: None,
            ftp_seq: 0,
            plans: [PLAN_MISSION, PLAN_FENCE, PLAN_RALLY].map(|kind| PlanSlot { transfer: Transfer::new(autopilot == crate::modes::AUTOPILOT_ARDUPILOT, kind), due: None, progress: 0.0, error: None }),
        }
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
                    self.plans[plan].due = None;
                    self.plans[plan].error = (!success).then_some(error.clone());
                    if !success {
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
        match request.get("action").and_then(Value::as_str).unwrap_or("") {
            "load" => Ok(self.load_plan(kind, now_ms)),
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
                    match ok {
                        false => {
                            self.note(format!("Component metadata download failed: {error}"));
                            self.step_done(connect::Step::ComponentInformation, now_ms)
                        }
                        true => self.metadata_received(fetch.kind, &fetch.uri, &bytes, now_ms),
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
            (_, Ok(text)) => {
                match crate::compinfo::parse(&text) {
                    Ok(parsed) => self.parameter_metadata = Some(parsed),
                    Err(reason) => self.note(format!("Parameter metadata could not be parsed: {reason}")),
                }
                self.step_done(connect::Step::ComponentInformation, now_ms)
            }
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
            "rebootRequired": meta.vehicle_reboot_required,
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

    fn connect_vehicle(&self) -> connect::Vehicle {
        let px4 = self.autopilot == crate::modes::AUTOPILOT_PX4;
        let apm = self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT;
        let proto = self.max_proto_version.unwrap_or(0);
        connect::Vehicle { px4, apm, fence_supported: self.capabilities & connect::CAP_MISSION_FENCE != 0 && proto >= PROTO_MAVLINK2, rally_supported: self.capabilities & connect::CAP_MISSION_RALLY != 0 && proto >= PROTO_MAVLINK2, max_proto_version: proto }
    }

    pub fn firmware(&self) -> Option<Firmware> {
        self.autopilot_version.as_ref().map(|v| connect::firmware_from(v, self.autopilot == crate::modes::AUTOPILOT_PX4))
    }

    pub fn parameter(&self, component: u8, name: &str) -> Option<ParamValue> {
        self.params.value(component, name)
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
        self.follow_connect(actions, now_ms)
    }

    fn step_done(&mut self, step: connect::Step, now_ms: u64) -> Vec<Vec<u8>> {
        if self.connect.current() != Some(step) {
            return Vec::new();
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
                Action::RefreshParameters => {
                    let actions = self.params.start();
                    self.follow_params(actions, now_ms)
                }
                Action::RequestComponentInformation => {
                    let outs = self.commands.request_message(MSG_COMPONENT_METADATA as u64, self.component, MSG_COMPONENT_METADATA, [0.0; 5], now_ms);
                    self.handle(outs, now_ms)
                }
                Action::LoadMission => self.load_plan(PLAN_MISSION, now_ms),
                Action::LoadGeoFence => self.load_plan(PLAN_FENCE, now_ms),
                Action::LoadRallyPoints => self.load_plan(PLAN_RALLY, now_ms),
                Action::FirstMissionLoadComplete => self.step_done(connect::Step::Mission, now_ms),
                Action::FirstGeoFenceLoadComplete => self.step_done(connect::Step::GeoFence, now_ms),
                Action::FirstRallyPointLoadComplete => self.step_done(connect::Step::RallyPoints, now_ms),
                Action::SetCapabilities(capabilities) => {
                    self.capabilities = capabilities;
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
                params::Action::Set { component, name, value } => self.encode(&Outbound::ParamSet { target: (id, component), name, bits: value.encode(), param_type: value.param_type() }).into_iter().collect(),
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
                    }
                    self.step_done(connect::Step::Parameters, now_ms)
                }
                params::Action::NoResponse => {
                    self.note("The vehicle did not respond to the parameter request.".to_string());
                    self.step_done(connect::Step::Parameters, now_ms)
                }
                params::Action::ReadFailed { component, name } => {
                    self.note(format!("Parameter read failed: {name} on component {component}"));
                    Vec::new()
                }
                params::Action::WriteFailed { component, name } => {
                    self.note(format!("Parameter write failed: {name} on component {component}"));
                    Vec::new()
                }
                params::Action::Added { .. } => Vec::new(),
            })
            .collect()
    }

    pub fn flight_mode(&self) -> String {
        crate::modes::name(self.autopilot, self.vehicle_type, self.base_mode, self.custom_mode)
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
            minimum_takeoff_altitude: MINIMUM_TAKEOFF_ALTITUDE,
            current_heading: Some(self.facts.heading),
        }
    }

    fn plan(&self, action: &Value) -> Plan {
        let state = self.planning_state();
        let number = |key: &str| action.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN);
        let flag = |key: &str| action.get(key).and_then(Value::as_bool).unwrap_or(false);
        match action.get("action").and_then(Value::as_str).unwrap_or("") {
            "takeoff" => guidedcmd::takeoff(&state, number("altitude")),
            "goto" => guidedcmd::goto(&state, number("latitude"), number("longitude"), action.get("loiterRadius").and_then(Value::as_f64).unwrap_or(0.0)),
            "changeAltitude" => guidedcmd::change_altitude(&state, number("delta"), flag("pause")),
            "pause" => guidedcmd::pause(&state),
            "rtl" => guidedcmd::rtl(&state, flag("smart")),
            "land" => guidedcmd::land(&state),
            "speed" => guidedcmd::change_speed(flag("ground"), number("metresPerSecond")),
            "arm" => Plan::Steps(vec![guidedcmd::arm(flag("arm"), flag("force"))]),
            "setMode" => guidedcmd::set_mode(&state, action.get("mode").and_then(Value::as_str).unwrap_or("")).map(Plan::Steps).unwrap_or_else(|| Plan::Refused("Unknown flight mode".to_string())),
            other => Plan::Refused(format!("Unknown guided action {other:?}")),
        }
    }

    pub fn start_guided(&mut self, action: &Value, now_ms: u64) -> Result<Vec<Vec<u8>>, String> {
        if self.guided.running() {
            return Err("A guided action is still running.".to_string());
        }
        match self.plan(action) {
            Plan::Refused(reason) => Err(reason),
            Plan::Steps(steps) => {
                self.errors.clear();
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
                Out::Send { command, command_int: false, params, .. } => self.encode(&Outbound::CommandLong { target, command, params }).into_iter().collect(),
                Out::Send { command, command_int: true, frame, params, x, y, .. } => self.encode(&Outbound::CommandInt { target, command, frame, params, x, y }).into_iter().collect(),
                Out::ShowError(text) => {
                    self.note(text);
                    Vec::new()
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
        let ticked = self.commands.tick(now_ms);
        let mut bytes = self.handle(ticked, now_ms);
        if self.initial_due.is_some_and(|due| now_ms >= due) {
            self.initial_due = None;
            let actions = self.params.on_initial_timeout();
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
        if self.ftp_due.is_some_and(|due| now_ms >= due) {
            self.ftp_due = None;
            let outs = self.fetch.as_mut().map(|f| f.download.on_timeout()).unwrap_or_default();
            bytes.extend(self.follow_ftp(outs, now_ms));
        }
        let slow = self.fetch.as_ref().is_some_and(|f| compmeta::too_slow(now_ms.saturating_sub(f.started_ms), f.progress));
        if slow {
            let outs = self.fetch.as_mut().map(|f| f.download.cancel()).unwrap_or_default();
            bytes.extend(self.follow_ftp(outs, now_ms));
            self.fetch = None;
            self.ftp_due = None;
            self.note("Component metadata download abandoned: too slow.".to_string());
            bytes.extend(self.step_done(connect::Step::ComponentInformation, now_ms));
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

    pub fn armed(&self) -> bool {
        self.base_mode & ARMED_FLAG != 0
    }

    #[allow(deprecated)]
    fn apply(&mut self, header: &MavHeader, message: &MavMessage, timestamp_us: u64, now_ms: u64) -> Vec<Vec<u8>> {
        self.messages += 1;
        *self.by_name.entry(message.message_name().to_string()).or_insert(0) += 1;
        let from = (header.system_id, header.component_id);
        match message {
            MavMessage::HEARTBEAT(h) if from == (self.id, self.component) => {
                self.heartbeats += 1;
                self.last_heartbeat_us = timestamp_us;
                self.connection_lost = false;
                self.base_mode = h.base_mode.bits();
                self.custom_mode = h.custom_mode;
                self.system_status = h.system_status as u8;
                self.vehicle_type = h.mavtype as u8;
            }
            MavMessage::COMMAND_ACK(a) => {
                let outs = self.commands.on_ack(header.component_id, a.command as u32 as u16, a.result as u8, now_ms);
                return self.handle(outs, now_ms);
            }
            MavMessage::AUTOPILOT_VERSION(v) => {
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
            MavMessage::FILE_TRANSFER_PROTOCOL(f) if matches!(f.target_system, 0 | mavout::GCS_SYSTEM) => {
                let Some(fetch) = self.fetch.as_mut().filter(|fetch| fetch.download.component == header.component_id) else { return Vec::new() };
                let outs = fetch.download.on_payload(&f.payload);
                return self.follow_ftp(outs, now_ms);
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
                let Some(value) = ParamValue::decode(p.param_type as u8, p.param_value) else { return Vec::new() };
                let actions = self.params.on_param_value(header.component_id, &name, p.param_count, p.param_index, value);
                return self.follow_params(actions, now_ms);
            }
            MavMessage::HOME_POSITION(h) => {
                self.home_altitude = Some(h.altitude as f64 / 1000.0);
                self.home = Some((h.latitude as f64 / 1e7, h.longitude as f64 / 1e7, h.altitude as f64 / 1000.0));
            }
            MavMessage::MISSION_COUNT(m) if matches!(m.target_system, 0 | mavout::GCS_SYSTEM) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let outs = self.plans[kind as usize].transfer.on_count(m.count);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::MISSION_ITEM_INT(m) if matches!(m.target_system, 0 | mavout::GCS_SYSTEM) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let scale = |v: i32| if m.frame as u8 == plantransfer::FRAME_MISSION { v as f64 } else { v as f64 * 1e-7 };
                let item = plantransfer::Item { seq: m.seq, frame: m.frame as u8, command: m.command as u32 as u16, current: m.current != 0, auto_continue: m.autocontinue != 0, params: [m.param1 as f64, m.param2 as f64, m.param3 as f64, m.param4 as f64, scale(m.x), scale(m.y), m.z as f64] };
                let outs = self.plans[kind as usize].transfer.on_item(item);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::MISSION_REQUEST_INT(m) if matches!(m.target_system, 0 | mavout::GCS_SYSTEM) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let outs = self.plans[kind as usize].transfer.on_request(m.seq);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::MISSION_REQUEST(m) if matches!(m.target_system, 0 | mavout::GCS_SYSTEM) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let outs = self.plans[kind as usize].transfer.on_request(m.seq);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::MISSION_ACK(m) if matches!(m.target_system, 0 | mavout::GCS_SYSTEM) && (m.mission_type as u8) < 3 => {
                let kind = m.mission_type as u8;
                let outs = self.plans[kind as usize].transfer.on_ack(m.mavtype as u8);
                return self.follow_plan(kind, outs, now_ms);
            }
            MavMessage::SYS_STATUS(s) => {
                self.sensors.update(s.onboard_control_sensors_present.bits(), s.onboard_control_sensors_enabled.bits(), s.onboard_control_sensors_health.bits());
            }
            MavMessage::STATUSTEXT(t) => {
                let end = t.text.iter().position(|b| *b == 0).unwrap_or(t.text.len());
                if let Some(status) = self.status_text.receive(header.component_id, t.severity as u8, 0, 0, &t.text[..end]) {
                    self.recent.push(status);
                    if self.recent.len() > MAX_MESSAGES {
                        self.recent.remove(0);
                    }
                }
            }
            _ => {}
        }
        self.gps.apply(message);
        self.batteries.apply(message);
        self.facts.apply(from, message);
        self.wind.apply(message);
        self.temperature.apply(message);
        self.distance.apply(message);
        self.local.apply(message);
        self.estimator.apply(message);
        Vec::new()
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
            "componentInformation": { "types": self.metadata_types.keys().collect::<Vec<_>>(), "parameterMetadata": self.parameter_metadata.as_ref().map(|p| p.named.len() + p.indexed.len()).unwrap_or(0) },
            "messages": self.recent.iter().rev().take(50).map(|m| json!({ "component": m.component, "severity": m.severity, "text": m.text })).collect::<Vec<_>>(),
            "byName": self.by_name,
        })
    }
}

#[derive(Debug, Default)]
pub struct Hub {
    vehicles: BTreeMap<u8, Vehicle>,
    active: Option<u8>,
}

pub static HUB: LazyLock<Mutex<Hub>> = LazyLock::new(|| Mutex::new(Hub::default()));

impl Hub {
    pub fn on_frame(&mut self, origin: Origin, header: &MavHeader, message: &MavMessage, timestamp_us: u64, now_ms: u64) -> Vec<(LinkId, Vec<u8>)> {
        let mut bytes = Vec::new();
        if let MavMessage::HEARTBEAT(h) = message {
            let (kind, autopilot) = (h.mavtype as u8, h.autopilot as u8);
            let excluded_type = matches!(kind, TYPE_GCS | TYPE_ONBOARD_CONTROLLER | TYPE_GIMBAL | TYPE_ADSB);
            if header.component_id == COMP_AUTOPILOT1 && !excluded_type && autopilot != AUTOPILOT_INVALID && header.system_id != 0 && !self.vehicles.contains_key(&header.system_id) {
                let mut vehicle = Vehicle::new(header.system_id, header.component_id, autopilot, kind, origin.link, origin.replay);
                if origin.v2 {
                    vehicle.max_proto_version = Some(PROTO_MAVLINK2);
                }
                bytes.extend(vehicle.begin_connect(now_ms));
                self.vehicles.insert(header.system_id, vehicle);
                self.active.get_or_insert(header.system_id);
            }
        }
        let Some(vehicle) = self.vehicles.get_mut(&header.system_id) else { return Vec::new() };
        if origin.v2 {
            vehicle.max_proto_version = Some(PROTO_MAVLINK2);
        }
        bytes.extend(vehicle.apply(header, message, timestamp_us, now_ms));
        let link = vehicle.link;
        bytes.extend(vehicle.pump(now_ms));
        match vehicle.replay {
            true => Vec::new(),
            false => bytes.into_iter().map(|bytes| (link, bytes)).collect(),
        }
    }

    pub fn link_closed(&mut self, link: LinkId) {
        let gone: Vec<u8> = self.vehicles.values().filter(|v| v.link == link).map(|v| v.id).collect();
        gone.iter().for_each(|id| self.remove(*id));
    }

    pub fn retain_links(&mut self, open: &[LinkId]) {
        let gone: Vec<u8> = self.vehicles.values().filter(|v| !open.contains(&v.link)).map(|v| v.id).collect();
        gone.iter().for_each(|id| self.remove(*id));
    }

    pub fn tick(&mut self, now_ms: u64) -> Vec<(LinkId, Vec<u8>)> {
        self.vehicles.values_mut().flat_map(|vehicle| {
            let (link, replay) = (vehicle.link, vehicle.replay);
            vehicle.pump(now_ms).into_iter().filter(move |_| !replay).map(move |bytes| (link, bytes))
        }).collect()
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

    pub fn mission_request(&mut self, id: Option<u8>, request: &Value, now_ms: u64) -> Result<Vec<(LinkId, Vec<u8>)>, String> {
        let chosen = id.or(self.active).ok_or_else(|| "No vehicle is connected through the core.".to_string())?;
        let vehicle = self.vehicles.get_mut(&chosen).ok_or_else(|| format!("Vehicle {chosen} is not connected through the core."))?;
        let link = vehicle.link;
        vehicle.mission_request(request, now_ms).map(|frames| frames.into_iter().map(|bytes| (link, bytes)).collect())
    }

    pub fn guided_snapshot(&self, id: Option<u8>) -> Value {
        let chosen = id.and_then(|id| self.vehicles.get(&id)).or_else(|| self.active());
        json!({
            "kind": "object",
            "class": "CoreGuided",
            "available": chosen.is_some(),
            "vehicleId": chosen.map(|v| v.id),
            "guided": chosen.map(Vehicle::guided_snapshot).unwrap_or(Value::Null),
        })
    }

    pub fn active(&self) -> Option<&Vehicle> {
        self.active.and_then(|id| self.vehicles.get(&id))
    }

    pub fn expire(&mut self, now_us: u64) -> Vec<u8> {
        self.vehicles.values_mut().for_each(|v| v.connection_lost = now_us.saturating_sub(v.last_heartbeat_us) > CONNECTION_LOST_US);
        self.vehicles.values().filter(|v| v.connection_lost).map(|v| v.id).collect()
    }

    pub fn remove(&mut self, id: u8) {
        self.vehicles.remove(&id);
        if self.active == Some(id) {
            self.active = self.vehicles.keys().next().copied();
        }
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
            "vehicleIds": self.vehicles.keys().collect::<Vec<_>>(),
            "vehicle": chosen.map(Vehicle::snapshot).unwrap_or(Value::Null),
        })
    }
}

pub fn now_us() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_micros() as u64).unwrap_or(0)
}

static STARTED: LazyLock<std::time::Instant> = LazyLock::new(std::time::Instant::now);

pub fn now_ms() -> u64 {
    STARTED.elapsed().as_millis() as u64
}

pub fn lock() -> MutexGuard<'static, Hub> {
    HUB.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn core_vehicle_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let mut hub = lock();
    hub.expire(now_us());
    hub.snapshot_of(args.first().and_then(|a| a.trim().parse().ok()))
}

pub fn core_guided_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    lock().guided_snapshot(args.first().and_then(|a| a.trim().parse().ok()))
}

pub fn core_mission_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let hub = lock();
    let vehicle = args.first().and_then(|a| a.trim().parse().ok()).and_then(|id| hub.vehicles.get(&id)).or_else(|| hub.active());
    json!({
        "kind": "object",
        "class": "CoreMission",
        "available": vehicle.is_some(),
        "vehicleId": vehicle.map(|v| v.id),
        "plans": vehicle.map(Vehicle::mission_snapshot).unwrap_or(Value::Null),
    })
}

pub fn core_parameters_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let hub = lock();
    let vehicle = args.first().and_then(|a| a.trim().parse().ok()).and_then(|id| hub.vehicles.get(&id)).or_else(|| hub.active());
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
        [id, name] => (id.trim().parse().ok(), name.trim()),
        [name] => (None, name.trim()),
        _ => (None, ""),
    };
    let hub = lock();
    let vehicle = id.and_then(|id| hub.vehicles.get(&id)).or_else(|| hub.active());
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
    fn the_sample_log_builds_one_vehicle_with_its_facts_and_counts() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
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
        assert!(hub.expire(5 + CONNECTION_LOST_US).is_empty());
        assert_eq!(hub.expire(6 + CONNECTION_LOST_US), vec![1]);
        assert_eq!((hub.snapshot()["available"].as_bool(), hub.snapshot()["vehicle"]["connectionLost"].as_bool()), (Some(true), Some(true)), "a silent vehicle is kept and flagged, as the Qt head keeps it until its link closes");
        hub.on_frame(origin(0), &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 7 + CONNECTION_LOST_US, 0);
        assert_eq!(hub.snapshot()["vehicle"]["connectionLost"], false);
        let mut two = Hub::default();
        two.on_frame(origin(0), &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 5, 0);
        two.on_frame(origin(0), &MavHeader { system_id: 7, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad), 6, 0);
        assert_eq!(two.snapshot()["vehicle"]["id"], 1);
        assert_eq!(two.snapshot_of(Some(7))["vehicle"]["id"], 7);
        assert_eq!(two.snapshot_of(Some(9))["available"], false);
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
        let armed = hub.on_frame(origin(4), &autopilot, &copter_heartbeat(4, false), 2_000_000, 2000);
        let MavMessage::COMMAND_LONG(arm) = decode(&armed[0].1) else { panic!() };
        assert_eq!((arm.command, arm.param1), (MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, 1.0));
        assert!(hub.tick(2_500).is_empty());
        let takeoff = hub.on_frame(origin(4), &autopilot, &copter_heartbeat(4, true), 3_000_000, 3000);
        let MavMessage::COMMAND_LONG(t) = decode(&takeoff[0].1) else { panic!() };
        assert_eq!((t.command, t.param7), (MavCmd::MAV_CMD_NAV_TAKEOFF, 10.0));
        assert_eq!(hub.guided_snapshot(Some(1))["guided"]["state"], "done");
        let denied = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_NAV_TAKEOFF, result: MavResult::MAV_RESULT_DENIED, ..Default::default() });
        hub.on_frame(origin(4), &autopilot, &denied, 3_100_000, 3100);
        assert_eq!(hub.guided_snapshot(None)["guided"]["errors"][0], "MAV_CMD 22 command denied");
        let goto = hub.guided(None, &json!({ "action": "goto", "latitude": 47.5, "longitude": 8.6 }), 3_200).unwrap();
        assert!(matches!(decode(&goto[0].1), MavMessage::COMMAND_INT(c) if c.command == MavCmd::MAV_CMD_DO_REPOSITION && c.x == 475000000));
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

    fn connect_copter(hub: &mut Hub, autopilot: &MavHeader) {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        hub.on_frame(origin(4), autopilot, &copter_heartbeat(5, false), 0, 0);
        let refused = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED, ..Default::default() });
        hub.on_frame(origin(4), autopilot, &refused, 1, 1);
        hub.on_frame(origin(4), autopilot, &refused, 2, 2);
        hub.on_frame(origin(4), autopilot, &refused, 3, 3);
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
        assert_eq!(first.len(), 1);
        assert_eq!(request_of(&first[0].1), (512, 148.0), "the first thing asked of a new vehicle is its autopilot version");
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
        assert!(matches!(decode(&after_metadata[0].1), MavMessage::PARAM_REQUEST_LIST(l) if l.target_system == 1 && l.target_component == 0), "metadata refused, parameters requested from every component");
        assert_eq!(hub.snapshot()["vehicle"]["connectStep"], "Parameters");
        assert!(hub.on_frame(origin(4), &autopilot, &param_value("RTL_ALT", 2, 0, 1500.0), 1_300_000, 1_300).is_empty());
        assert_eq!(hub.snapshot()["vehicle"]["parameters"]["ready"], false);
        let listed = hub.on_frame(origin(4), &autopilot, &param_value("WPNAV_SPEED", 2, 1, 250.0), 1_400_000, 1_400);
        assert!(matches!(decode(&listed[0].1), MavMessage::MISSION_REQUEST_LIST(_)), "with the parameters in, the mission is read from the vehicle");
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
        hub.on_frame(origin(4), &autopilot, &param_value("RTL_ALT", 2, 0, 2000.0), 11_100_000, 11_100);
        assert_eq!(hub.active().unwrap().parameter(1, "RTL_ALT").map(ParamValue::as_f64), Some(2000.0));
        let refreshed = hub.parameter_request(Some(1), &json!({ "name": "WPNAV_SPEED", "refresh": true }), 12_000).unwrap();
        assert!(matches!(decode(&refreshed[0].1), MavMessage::PARAM_REQUEST_READ(r) if r.param_index == -1 && r.param_id.to_str().unwrap() == "WPNAV_SPEED"));
        assert!(hub.parameter_request(Some(9), &json!({ "name": "X", "value": 1.0 }), 12_000).is_err());
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
        let listed = hub.on_frame(origin(4), &autopilot, &refused, 200_000, 200);
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
        let plain = br#"{"version":1,"parameters":[{"name":"RTL_ALT","type":"float","shortDesc":"Return altitude","units":"m","min":0,"max":8000},{"name":"CAM_{n}_MODE","type":"uint8","shortDesc":"Camera {n} mode"}]}"#;
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
        assert_eq!(snapshot["vehicle"]["componentInformation"], json!({ "types": [1], "parameterMetadata": 2 }));
        let meta = hub.active().unwrap().parameter_meta("RTL_ALT", Some(ParamValue::F32(0.0))).unwrap();
        assert_eq!((meta["shortDescription"].as_str(), meta["units"].as_str(), meta["max"].as_f64()), (Some("Return altitude"), Some("m"), Some(8000.0)));
        assert_eq!(hub.active().unwrap().parameter_meta("CAM_2_MODE", Some(ParamValue::U8(0))).unwrap()["shortDescription"], "Camera 2 mode");
        assert!(hub.active().unwrap().parameter_meta("NOPE", None).is_none());
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
        let frames: Vec<MavMessage> = hub.tick(1_100).into_iter().map(|(_, b)| decode(&b)).collect();
        assert!(matches!(frames.last(), Some(MavMessage::PARAM_REQUEST_LIST(_))), "an unanswered open fails the fetch and the parameters are requested anyway");
        assert!(hub.guided_snapshot(None)["guided"]["errors"].as_array().unwrap().iter().any(|e| e.as_str().unwrap().contains("Download failed")));
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
        assert!(matches!(decode(&started[0].1), MavMessage::MISSION_COUNT(c) if c.count == 2), "ArduPilot does not take the home item");
        assert!(hub.mission_request(None, &json!({ "action": "load" }), 10_000).is_err(), "one transfer at a time");
        let request = |seq: u16| MavMessage::MISSION_REQUEST_INT(MISSION_REQUEST_INT_DATA { seq, target_system: 255, target_component: 190, ..Default::default() });
        let first = hub.on_frame(origin(4), &autopilot, &request(0), 10_100_000, 10_100);
        assert!(matches!(decode(&first[0].1), MavMessage::MISSION_ITEM_INT(i) if i.seq == 0 && i.current == 1 && i.x == 471000000));
        let second = hub.on_frame(origin(4), &autopilot, &request(1), 10_200_000, 10_200);
        assert!(matches!(decode(&second[0].1), MavMessage::MISSION_ITEM_INT(i) if i.seq == 1 && i.param1 == 0.0), "the jump target follows the dropped home item");
        hub.on_frame(origin(4), &autopilot, &MavMessage::MISSION_ACK(MISSION_ACK_DATA { target_system: 255, target_component: 190, mavtype: MavMissionResult::MAV_MISSION_ACCEPTED, ..Default::default() }), 10_300_000, 10_300);
        let mission = hub.active().unwrap().mission_snapshot()["mission"].clone();
        assert_eq!((mission["inProgress"].as_bool(), mission["count"].as_u64(), mission["error"].is_null()), (Some(false), Some(2), true));
        assert!(hub.tick(12_000).is_empty());
        assert!(hub.mission_request(None, &json!({ "action": "write", "items": [{ "frame": 0 }] }), 12_000).is_err());
        assert!(hub.mission_request(None, &json!({ "action": "write", "items": [{ "frame": 0, "command": 16, "params": [0, "x", 0, 0, 47, 8, 50] }] }), 12_000).is_err(), "a non-numeric param is refused before anything is sent");
        assert!(hub.mission_request(None, &json!({ "action": "write", "items": [{ "frame": 0, "command": 65000, "params": [0, 0, 0, 0, 47, 8, 50] }] }), 12_000).is_err(), "a command the dialect cannot name is refused");
        assert!(hub.mission_request(None, &json!({ "action": "write", "items": [] }), 12_000).is_err());
        let again = hub.mission_request(None, &json!({ "action": "write", "items": [{ "frame": 0, "command": 16, "params": [0, 0, 0, 0, 47.0, 8.0, 0] }, { "frame": 3, "command": 16, "params": [0, 0, 0, 0, 47.2, 8.2, 60] }] }), 13_000).unwrap();
        assert!(matches!(decode(&again[0].1), MavMessage::MISSION_COUNT(c) if c.count == 1));
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
}
