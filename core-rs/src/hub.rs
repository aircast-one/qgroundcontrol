use mavlink::{MavHeader, Message};
use mavlink::dialects::ardupilotmega::MavMessage;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use crate::batteryfacts::Batteries;
use crate::gpsfacts::GpsFacts;
use crate::connect::{self, Action, AutopilotVersion, Connect, Firmware, MSG_AUTOPILOT_VERSION, MSG_PROTOCOL_VERSION};
use crate::guidedcmd::{self, CMD_DO_REPOSITION, Plan, VehicleState};
use crate::guidedexec::{Emit, Executor, Observed};
use crate::mavcmd::{Command, Commands, Failure, Out, RESULT_ACCEPTED};
use crate::mavout::{self, Outbound};
use crate::params::{self, INITIAL_REQUEST_TIMEOUT_MS, ParamValue, Params, WAITING_TIMEOUT_MS};
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
    pub reposition_supported: Option<bool>,
    pub errors: Vec<String>,
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
            reposition_supported: None,
            errors: Vec::new(),
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
        }
    }

    fn connect_link(&self) -> connect::Link {
        connect::Link { present: true, high_latency: self.commands.high_latency, log_replay: self.replay }
    }

    fn connect_vehicle(&self) -> connect::Vehicle {
        let px4 = self.autopilot == crate::modes::AUTOPILOT_PX4;
        let apm = self.autopilot == crate::modes::AUTOPILOT_ARDUPILOT;
        connect::Vehicle { px4, apm, fence_supported: px4 || apm, rally_supported: px4 || apm, max_proto_version: self.max_proto_version.unwrap_or(0) }
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
                Action::RequestComponentInformation => self.step_done(connect::Step::ComponentInformation, now_ms),
                Action::LoadMission => self.step_done(connect::Step::Mission, now_ms),
                Action::LoadGeoFence => self.step_done(connect::Step::GeoFence, now_ms),
                Action::LoadRallyPoints => self.step_done(connect::Step::RallyPoints, now_ms),
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
            MavMessage::HOME_POSITION(h) => self.home_altitude = Some(h.altitude as f64 / 1000.0),
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
    pub fn on_frame(&mut self, link: LinkId, replay: bool, header: &MavHeader, message: &MavMessage, timestamp_us: u64, now_ms: u64) -> Vec<(LinkId, Vec<u8>)> {
        let mut bytes = Vec::new();
        if let MavMessage::HEARTBEAT(h) = message {
            let (kind, autopilot) = (h.mavtype as u8, h.autopilot as u8);
            let excluded_type = matches!(kind, TYPE_GCS | TYPE_ONBOARD_CONTROLLER | TYPE_GIMBAL | TYPE_ADSB);
            if header.component_id == COMP_AUTOPILOT1 && !excluded_type && autopilot != AUTOPILOT_INVALID && header.system_id != 0 && !self.vehicles.contains_key(&header.system_id) {
                let mut vehicle = Vehicle::new(header.system_id, header.component_id, autopilot, kind, link, replay);
                bytes.extend(vehicle.begin_connect(now_ms));
                self.vehicles.insert(header.system_id, vehicle);
                self.active.get_or_insert(header.system_id);
            }
        }
        let Some(vehicle) = self.vehicles.get_mut(&header.system_id) else { return Vec::new() };
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
        let gone: Vec<u8> = self.vehicles.values().filter(|v| now_us.saturating_sub(v.last_heartbeat_us) > CONNECTION_LOST_US).map(|v| v.id).collect();
        gone.iter().for_each(|id| self.remove(*id));
        gone
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
            hub.on_frame(0, false, header, message, ts, ts / 1000);
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
        hub.on_frame(0, false, &MavHeader { system_id: 255, component_id: 190, sequence: 0 }, &MavMessage::HEARTBEAT(gcs), 0, 0);
        assert_eq!(hub.snapshot()["available"], false);
        let mut companion = HEARTBEAT_DATA::default();
        companion.mavtype = MavType::MAV_TYPE_QUADROTOR;
        companion.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        hub.on_frame(0, false, &MavHeader { system_id: 1, component_id: 191, sequence: 0 }, &MavMessage::HEARTBEAT(companion.clone()), 1, 0);
        assert_eq!(hub.snapshot()["available"], false, "a heartbeat from a non-autopilot component makes no vehicle");
        companion.mavtype = MavType::MAV_TYPE_ONBOARD_CONTROLLER;
        hub.on_frame(0, false, &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(companion), 2, 0);
        assert_eq!(hub.snapshot()["available"], false, "an onboard controller type makes no vehicle");
        let mut quad = HEARTBEAT_DATA::default();
        quad.mavtype = MavType::MAV_TYPE_QUADROTOR;
        quad.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        quad.base_mode = mavlink::dialects::ardupilotmega::MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED;
        hub.on_frame(0, false, &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 5, 0);
        let snapshot = hub.snapshot();
        assert_eq!((snapshot["vehicle"]["armed"].as_bool(), snapshot["vehicle"]["autopilot"].as_u64(), snapshot["vehicle"]["heartbeats"].as_u64()), (Some(true), Some(12), Some(1)));
        assert!(hub.expire(5 + CONNECTION_LOST_US).is_empty());
        assert_eq!(hub.expire(6 + CONNECTION_LOST_US), vec![1]);
        assert_eq!(hub.snapshot()["available"], false);
        let mut two = Hub::default();
        two.on_frame(0, false, &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 5, 0);
        two.on_frame(0, false, &MavHeader { system_id: 7, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad), 6, 0);
        assert_eq!(two.snapshot()["vehicle"]["id"], 1);
        assert_eq!(two.snapshot_of(Some(7))["vehicle"]["id"], 7);
        assert_eq!(two.snapshot_of(Some(9))["available"], false);
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
        hub.on_frame(4, false, &autopilot, &position, 1_100_000, 1100);
        let started = hub.guided(None, &json!({ "action": "takeoff", "altitude": 10.0 }), 1_100).unwrap();
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].0, 4, "sent on the link the heartbeat came from");
        let MavMessage::COMMAND_LONG(set_mode) = decode(&started[0].1) else { panic!() };
        assert_eq!((set_mode.command, set_mode.param2, set_mode.target_system), (MavCmd::MAV_CMD_DO_SET_MODE, 4.0, 1));
        assert_eq!(hub.guided_snapshot(None)["guided"]["state"], "running");
        assert!(hub.guided(None, &json!({ "action": "land" }), 1_200).is_err(), "one action at a time");
        let ack = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_DO_SET_MODE, result: MavResult::MAV_RESULT_ACCEPTED });
        assert!(hub.on_frame(4, false, &autopilot, &ack, 1_300_000, 1300).is_empty());
        let armed = hub.on_frame(4, false, &autopilot, &copter_heartbeat(4, false), 2_000_000, 2000);
        let MavMessage::COMMAND_LONG(arm) = decode(&armed[0].1) else { panic!() };
        assert_eq!((arm.command, arm.param1), (MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, 1.0));
        assert!(hub.tick(2_500).is_empty());
        let takeoff = hub.on_frame(4, false, &autopilot, &copter_heartbeat(4, true), 3_000_000, 3000);
        let MavMessage::COMMAND_LONG(t) = decode(&takeoff[0].1) else { panic!() };
        assert_eq!((t.command, t.param7), (MavCmd::MAV_CMD_NAV_TAKEOFF, 10.0));
        assert_eq!(hub.guided_snapshot(Some(1))["guided"]["state"], "done");
        let denied = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_NAV_TAKEOFF, result: MavResult::MAV_RESULT_DENIED });
        hub.on_frame(4, false, &autopilot, &denied, 3_100_000, 3100);
        assert_eq!(hub.guided_snapshot(None)["guided"]["errors"][0], "MAV_CMD 22 command denied");
        let goto = hub.guided(None, &json!({ "action": "goto", "latitude": 47.5, "longitude": 8.6 }), 3_200).unwrap();
        assert!(matches!(decode(&goto[0].1), MavMessage::COMMAND_INT(c) if c.command == MavCmd::MAV_CMD_DO_REPOSITION && c.x == 475000000));
        assert!(matches!(decode(&goto[1].1), MavMessage::MISSION_ITEM(i) if i.current == 2));
        let unsupported = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_DO_REPOSITION, result: MavResult::MAV_RESULT_UNSUPPORTED });
        hub.on_frame(4, false, &autopilot, &unsupported, 3_300_000, 3300);
        assert_eq!(hub.guided_snapshot(None)["guided"]["repositionSupported"], false);
        let rtl = hub.guided(None, &json!({ "action": "rtl" }), 3_400).unwrap();
        assert_eq!(rtl.len(), 1);
        hub.retain_links(&[4]);
        assert_eq!(hub.guided_snapshot(None)["available"], true);
        hub.link_closed(4);
        assert_eq!(hub.guided_snapshot(None)["available"], false, "a vehicle goes with its link, as it does in the Qt head");
        assert!(hub.guided(None, &json!({ "action": "land" }), 3_500).is_err());
        hub.on_frame(6, false, &autopilot, &copter_heartbeat(4, true), 4_000_000, 4_000);
        assert_eq!(hub.guided(None, &json!({ "action": "land" }), 4_100).unwrap()[0].0, 6, "a heartbeat on a new link makes a fresh vehicle bound to it");
    }

    fn connect_copter(hub: &mut Hub, autopilot: &MavHeader) {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        hub.on_frame(4, false, autopilot, &copter_heartbeat(5, false), 0, 0);
        let refused = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED });
        hub.on_frame(4, false, autopilot, &refused, 1, 1);
        hub.on_frame(4, false, autopilot, &refused, 2, 2);
        hub.on_frame(4, false, autopilot, &param_value("RTL_ALT", 1, 0, 1500.0), 3, 3);
        assert_eq!(hub.snapshot()["vehicle"]["initialConnectComplete"], true);
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
        let first = hub.on_frame(4, false, &autopilot, &copter_heartbeat(5, false), 1_000_000, 1_000);
        assert_eq!(first.len(), 1);
        assert_eq!(request_of(&first[0].1), (512, 148.0), "the first thing asked of a new vehicle is its autopilot version");
        assert_eq!(hub.snapshot()["vehicle"]["connectStep"], "AutopilotVersion");
        let version = MavMessage::AUTOPILOT_VERSION(AUTOPILOT_VERSION_DATA { capabilities: MavProtocolCapability::from_bits_retain(8 | 4), flight_sw_version: 0x04050600, ..Default::default() });
        let after_version = hub.on_frame(4, false, &autopilot, &version, 1_100_000, 1_100);
        assert_eq!(request_of(&after_version[0].1), (512, 435.0), "ArduPilot skips the protocol version and goes to standard modes");
        assert_eq!(hub.snapshot()["vehicle"]["capabilities"], 12);
        assert_eq!(hub.snapshot()["vehicle"]["firmware"]["version"], "4.5.6 (0)");
        let unsupported = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED });
        let after_modes = hub.on_frame(4, false, &autopilot, &unsupported, 1_200_000, 1_200);
        assert!(matches!(decode(&after_modes[0].1), MavMessage::PARAM_REQUEST_LIST(l) if l.target_system == 1 && l.target_component == 0), "modes refused, parameters requested from every component");
        assert_eq!(hub.snapshot()["vehicle"]["connectStep"], "Parameters");
        assert!(hub.on_frame(4, false, &autopilot, &param_value("RTL_ALT", 2, 0, 1500.0), 1_300_000, 1_300).is_empty());
        assert_eq!(hub.snapshot()["vehicle"]["parameters"]["ready"], false);
        hub.on_frame(4, false, &autopilot, &param_value("WPNAV_SPEED", 2, 1, 250.0), 1_400_000, 1_400);
        let snapshot = hub.snapshot();
        assert_eq!(snapshot["vehicle"]["parameters"], json!({ "ready": true, "progress": 1.0, "count": 2 }));
        assert_eq!(snapshot["vehicle"]["initialConnectComplete"], true);
        assert_eq!(snapshot["vehicle"]["connectProgress"], 1.0);
        assert_eq!(hub.active().unwrap().parameter(1, "RTL_ALT").map(ParamValue::as_f64), Some(1500.0));
        assert!(hub.tick(10_000).is_empty(), "nothing is pending once connected");
        let written = hub.parameter_request(None, &json!({ "name": "RTL_ALT", "value": 2000.0 }), 11_000).unwrap();
        let MavMessage::PARAM_SET(set) = decode(&written[0].1) else { panic!() };
        assert_eq!((set.param_id.to_str().unwrap(), set.param_value, set.param_type as u8, set.target_component), ("RTL_ALT", 2000.0, 9, 1));
        assert_eq!(hub.parameter_request(None, &json!({ "name": "NEW_ONE", "value": 1.0 }), 11_000).unwrap_err(), "NEW_ONE is not a parameter of component 1.");
        assert!(hub.parameter_request(None, &json!({ "name": "RTL ALT", "value": 1.0 }), 11_000).is_err(), "a name with a space never goes on the wire");
        hub.on_frame(4, false, &autopilot, &param_value("RTL_ALT", 2, 0, 2000.0), 11_100_000, 11_100);
        assert_eq!(hub.active().unwrap().parameter(1, "RTL_ALT").map(ParamValue::as_f64), Some(2000.0));
        let refreshed = hub.parameter_request(Some(1), &json!({ "name": "WPNAV_SPEED", "refresh": true }), 12_000).unwrap();
        assert!(matches!(decode(&refreshed[0].1), MavMessage::PARAM_REQUEST_READ(r) if r.param_index == -1 && r.param_id.to_str().unwrap() == "WPNAV_SPEED"));
        assert!(hub.parameter_request(Some(9), &json!({ "name": "X", "value": 1.0 }), 12_000).is_err());
    }

    #[test]
    fn a_write_waits_for_the_parameter_list_and_refuses_values_the_type_cannot_hold() {
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        hub.on_frame(4, false, &autopilot, &copter_heartbeat(5, false), 0, 0);
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
        assert!(hub.on_frame(4, true, &autopilot, &copter_heartbeat(5, false), 0, 0).is_empty());
        assert!(hub.tick(10_000).is_empty());
        assert_eq!(hub.snapshot()["vehicle"]["heartbeats"], 1);
    }

    #[test]
    fn a_silent_vehicle_gets_its_parameter_list_requested_again_after_the_initial_timeout() {
        use mavlink::dialects::ardupilotmega::{COMMAND_ACK_DATA, MavCmd, MavResult};
        let autopilot = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut hub = Hub::default();
        hub.on_frame(4, false, &autopilot, &copter_heartbeat(5, false), 0, 0);
        let refused = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_REQUEST_MESSAGE, result: MavResult::MAV_RESULT_UNSUPPORTED });
        hub.on_frame(4, false, &autopilot, &refused, 100_000, 100);
        let listed = hub.on_frame(4, false, &autopilot, &refused, 200_000, 200);
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
        hub.on_frame(4, false, &autopilot, &MavMessage::HEARTBEAT(px4), 0, 0);
        let after_version = hub.on_frame(4, false, &autopilot, &MavMessage::AUTOPILOT_VERSION(AUTOPILOT_VERSION_DATA::default()), 100_000, 100);
        assert_eq!(request_of(&after_version[0].1), (512, 300.0));
        let protocol = MavMessage::PROTOCOL_VERSION(PROTOCOL_VERSION_DATA { version: 200, min_version: 100, max_version: 200, ..Default::default() });
        let after_protocol = hub.on_frame(4, false, &autopilot, &protocol, 200_000, 200);
        assert_eq!(request_of(&after_protocol[0].1), (512, 435.0));
        let mode = |index: u8, name: &str, custom: u32| {
            let mut name_bytes = [0u8; 35];
            name_bytes[..name.len()].copy_from_slice(name.as_bytes());
            MavMessage::AVAILABLE_MODES(AVAILABLE_MODES_DATA { custom_mode: custom, number_modes: 2, mode_index: index, standard_mode: MavStandardMode::MAV_STANDARD_MODE_NON_STANDARD, mode_name: name_bytes.into(), ..Default::default() })
        };
        let second = hub.on_frame(4, false, &autopilot, &mode(1, "Manual", 65536), 300_000, 300);
        assert_eq!(request_of(&second[0].1), (512, 435.0), "the next mode is requested");
        let listed = hub.on_frame(4, false, &autopilot, &mode(2, "Position", 196608), 400_000, 400);
        assert!(matches!(decode(&listed[0].1), MavMessage::PARAM_REQUEST_LIST(_)));
        let modes = hub.snapshot()["vehicle"]["flightModes"].clone();
        assert_eq!(modes.as_array().unwrap().len(), 2);
        assert_eq!(modes[1]["name"], "Position");
        assert_eq!(hub.snapshot()["vehicle"]["maxProtoVersion"], 200);
    }
}
