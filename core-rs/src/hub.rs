use mavlink::{MavHeader, Message};
use mavlink::dialects::ardupilotmega::MavMessage;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use crate::batteryfacts::Batteries;
use crate::gpsfacts::GpsFacts;
use crate::guidedcmd::{self, CMD_DO_REPOSITION, Plan, VehicleState};
use crate::guidedexec::{Emit, Executor, Observed};
use crate::mavcmd::{Command, Commands, Failure, Out, RESULT_ACCEPTED};
use crate::mavout::{self, Outbound};
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
    commands: Commands,
    guided: Executor,
}

impl Vehicle {
    fn new(id: u8, component: u8, autopilot: u8, vehicle_type: u8, link: LinkId) -> Vehicle {
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
            commands: Commands::for_firmware(autopilot == crate::modes::AUTOPILOT_PX4),
            guided: Executor::default(),
        }
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
                    self.handle(outs)
                }
                Emit::SetMode { base_mode, custom_mode } => self.encode(&Outbound::SetMode { system: self.id, base_mode, custom_mode }).into_iter().collect(),
                Emit::PositionTargetLocalNed { frame, type_mask, x, y, z } => self.encode(&Outbound::PositionTargetLocalNed { target, frame, type_mask, x, y, z }).into_iter().collect(),
                Emit::GuidedMissionItem { latitude, longitude, altitude_relative } => self.encode(&Outbound::GuidedMissionItem { target, latitude, longitude, altitude_relative }).into_iter().collect(),
            })
            .collect()
    }

    fn handle(&mut self, outs: Vec<Out>) -> Vec<Vec<u8>> {
        let target = (self.id, self.component);
        outs.into_iter()
            .filter_map(|out| match out {
                Out::Send { command, command_int: false, params, .. } => Some(Outbound::CommandLong { target, command, params }),
                Out::Send { command, command_int: true, frame, params, x, y, .. } => Some(Outbound::CommandInt { target, command, frame, params, x, y }),
                Out::ShowError(text) => {
                    self.note(text);
                    None
                }
                Out::Result { command: CMD_DO_REPOSITION, result, failure: Failure::ResultOnly, .. } => {
                    self.reposition_supported = match result {
                        RESULT_ACCEPTED => Some(true),
                        RESULT_UNSUPPORTED => Some(false),
                        _ => self.reposition_supported,
                    };
                    None
                }
                _ => None,
            })
            .collect::<Vec<Outbound>>()
            .iter()
            .filter_map(|send| self.encode(send))
            .collect()
    }

    pub fn pump(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        let ticked = self.commands.tick(now_ms);
        let mut bytes = self.handle(ticked);
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

    fn apply(&mut self, header: &MavHeader, message: &MavMessage, timestamp_us: u64, now_ms: u64) -> Vec<Out> {
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
            MavMessage::COMMAND_ACK(a) => return self.commands.on_ack(header.component_id, a.command as u32 as u16, a.result as u8, now_ms),
            MavMessage::AUTOPILOT_VERSION(v) => self.capabilities = v.capabilities.bits(),
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
        json!({
            "id": self.id,
            "component": self.component,
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
    pub fn on_frame(&mut self, link: LinkId, header: &MavHeader, message: &MavMessage, timestamp_us: u64, now_ms: u64) -> Vec<(LinkId, Vec<u8>)> {
        if let MavMessage::HEARTBEAT(h) = message {
            let (kind, autopilot) = (h.mavtype as u8, h.autopilot as u8);
            let excluded_type = matches!(kind, TYPE_GCS | TYPE_ONBOARD_CONTROLLER | TYPE_GIMBAL | TYPE_ADSB);
            if header.component_id == COMP_AUTOPILOT1 && !excluded_type && autopilot != AUTOPILOT_INVALID && header.system_id != 0 && !self.vehicles.contains_key(&header.system_id) {
                self.vehicles.insert(header.system_id, Vehicle::new(header.system_id, header.component_id, autopilot, kind, link));
                self.active.get_or_insert(header.system_id);
            }
        }
        let Some(vehicle) = self.vehicles.get_mut(&header.system_id) else { return Vec::new() };
        let acked = vehicle.apply(header, message, timestamp_us, now_ms);
        let link = vehicle.link;
        let mut bytes = vehicle.handle(acked);
        bytes.extend(vehicle.pump(now_ms));
        bytes.into_iter().map(|bytes| (link, bytes)).collect()
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
            let link = vehicle.link;
            vehicle.pump(now_ms).into_iter().map(move |bytes| (link, bytes))
        }).collect()
    }

    pub fn guided(&mut self, id: Option<u8>, action: &Value, now_ms: u64) -> Result<Vec<(LinkId, Vec<u8>)>, String> {
        let chosen = id.or(self.active).ok_or_else(|| "No vehicle is connected through the core.".to_string())?;
        let vehicle = self.vehicles.get_mut(&chosen).ok_or_else(|| format!("Vehicle {chosen} is not connected through the core."))?;
        let link = vehicle.link;
        vehicle.start_guided(action, now_ms).map(|frames| frames.into_iter().map(|bytes| (link, bytes)).collect())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_log_builds_one_vehicle_with_its_facts_and_counts() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
        let mut hub = Hub::default();
        let summary = crate::tlog::parse(&bytes);
        crate::tlog::for_each(&bytes, |ts, header, message| {
            hub.on_frame(0, header, message, ts, ts / 1000);
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
        hub.on_frame(0, &MavHeader { system_id: 255, component_id: 190, sequence: 0 }, &MavMessage::HEARTBEAT(gcs), 0, 0);
        assert_eq!(hub.snapshot()["available"], false);
        let mut companion = HEARTBEAT_DATA::default();
        companion.mavtype = MavType::MAV_TYPE_QUADROTOR;
        companion.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        hub.on_frame(0, &MavHeader { system_id: 1, component_id: 191, sequence: 0 }, &MavMessage::HEARTBEAT(companion.clone()), 1, 0);
        assert_eq!(hub.snapshot()["available"], false, "a heartbeat from a non-autopilot component makes no vehicle");
        companion.mavtype = MavType::MAV_TYPE_ONBOARD_CONTROLLER;
        hub.on_frame(0, &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(companion), 2, 0);
        assert_eq!(hub.snapshot()["available"], false, "an onboard controller type makes no vehicle");
        let mut quad = HEARTBEAT_DATA::default();
        quad.mavtype = MavType::MAV_TYPE_QUADROTOR;
        quad.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        quad.base_mode = mavlink::dialects::ardupilotmega::MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED;
        hub.on_frame(0, &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 5, 0);
        let snapshot = hub.snapshot();
        assert_eq!((snapshot["vehicle"]["armed"].as_bool(), snapshot["vehicle"]["autopilot"].as_u64(), snapshot["vehicle"]["heartbeats"].as_u64()), (Some(true), Some(12), Some(1)));
        assert!(hub.expire(5 + CONNECTION_LOST_US).is_empty());
        assert_eq!(hub.expire(6 + CONNECTION_LOST_US), vec![1]);
        assert_eq!(hub.snapshot()["available"], false);
        let mut two = Hub::default();
        two.on_frame(0, &MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 5, 0);
        two.on_frame(0, &MavHeader { system_id: 7, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad), 6, 0);
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
        assert!(hub.on_frame(4, &autopilot, &copter_heartbeat(5, false), 1_000_000, 1000).is_empty());
        let refused = hub.guided(None, &json!({ "action": "takeoff", "altitude": 10.0 }), 1_000).unwrap_err();
        assert_eq!(refused, "Unable to takeoff, vehicle position not known.");
        let position = MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA { lat: 474000000, lon: 85000000, alt: 500_000, relative_alt: 0, ..Default::default() });
        hub.on_frame(4, &autopilot, &position, 1_100_000, 1100);
        let started = hub.guided(None, &json!({ "action": "takeoff", "altitude": 10.0 }), 1_100).unwrap();
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].0, 4, "sent on the link the heartbeat came from");
        let MavMessage::COMMAND_LONG(set_mode) = decode(&started[0].1) else { panic!() };
        assert_eq!((set_mode.command, set_mode.param2, set_mode.target_system), (MavCmd::MAV_CMD_DO_SET_MODE, 4.0, 1));
        assert_eq!(hub.guided_snapshot(None)["guided"]["state"], "running");
        assert!(hub.guided(None, &json!({ "action": "land" }), 1_200).is_err(), "one action at a time");
        let ack = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_DO_SET_MODE, result: MavResult::MAV_RESULT_ACCEPTED });
        assert!(hub.on_frame(4, &autopilot, &ack, 1_300_000, 1300).is_empty());
        let armed = hub.on_frame(4, &autopilot, &copter_heartbeat(4, false), 2_000_000, 2000);
        let MavMessage::COMMAND_LONG(arm) = decode(&armed[0].1) else { panic!() };
        assert_eq!((arm.command, arm.param1), (MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, 1.0));
        assert!(hub.tick(2_500).is_empty());
        let takeoff = hub.on_frame(4, &autopilot, &copter_heartbeat(4, true), 3_000_000, 3000);
        let MavMessage::COMMAND_LONG(t) = decode(&takeoff[0].1) else { panic!() };
        assert_eq!((t.command, t.param7), (MavCmd::MAV_CMD_NAV_TAKEOFF, 10.0));
        assert_eq!(hub.guided_snapshot(Some(1))["guided"]["state"], "done");
        let denied = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_NAV_TAKEOFF, result: MavResult::MAV_RESULT_DENIED });
        hub.on_frame(4, &autopilot, &denied, 3_100_000, 3100);
        assert_eq!(hub.guided_snapshot(None)["guided"]["errors"][0], "MAV_CMD 22 command denied");
        let goto = hub.guided(None, &json!({ "action": "goto", "latitude": 47.5, "longitude": 8.6 }), 3_200).unwrap();
        assert!(matches!(decode(&goto[0].1), MavMessage::COMMAND_INT(c) if c.command == MavCmd::MAV_CMD_DO_REPOSITION && c.x == 475000000));
        assert!(matches!(decode(&goto[1].1), MavMessage::MISSION_ITEM(i) if i.current == 2));
        let unsupported = MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: MavCmd::MAV_CMD_DO_REPOSITION, result: MavResult::MAV_RESULT_UNSUPPORTED });
        hub.on_frame(4, &autopilot, &unsupported, 3_300_000, 3300);
        assert_eq!(hub.guided_snapshot(None)["guided"]["repositionSupported"], false);
        let rtl = hub.guided(None, &json!({ "action": "rtl" }), 3_400).unwrap();
        assert_eq!(rtl.len(), 1);
        hub.retain_links(&[4]);
        assert_eq!(hub.guided_snapshot(None)["available"], true);
        hub.link_closed(4);
        assert_eq!(hub.guided_snapshot(None)["available"], false, "a vehicle goes with its link, as it does in the Qt head");
        assert!(hub.guided(None, &json!({ "action": "land" }), 3_500).is_err());
        hub.on_frame(6, &autopilot, &copter_heartbeat(4, true), 4_000_000, 4_000);
        assert_eq!(hub.guided(None, &json!({ "action": "land" }), 4_100).unwrap()[0].0, 6, "a heartbeat on a new link makes a fresh vehicle bound to it");
    }
}
