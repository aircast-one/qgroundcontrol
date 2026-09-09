use mavlink::{MavHeader, Message};
use mavlink::dialects::ardupilotmega::MavMessage;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};

use crate::batteryfacts::Batteries;
use crate::gpsfacts::GpsFacts;
use crate::sensorfacts::{DistanceSensorFacts, EstimatorStatusFacts, LocalPositionFacts, TemperatureFacts, WindFacts};
use crate::statustext::{Handler, StatusText};
use crate::sysstatus::SysStatusSensors;
use crate::vehiclefacts::VehicleFacts;

pub const TYPE_GCS: u8 = 6;
pub const AUTOPILOT_INVALID: u8 = 8;
pub const ARMED_FLAG: u8 = 128;
pub const CUSTOM_MODE_FLAG: u8 = 1;
const MAX_MESSAGES: usize = 200;
pub const CONNECTION_LOST_US: u64 = 3_500_000;

#[derive(Debug)]
pub struct Vehicle {
    pub id: u8,
    pub component: u8,
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
}

impl Vehicle {
    fn new(id: u8, component: u8, autopilot: u8, vehicle_type: u8) -> Vehicle {
        Vehicle {
            id,
            component,
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
        }
    }

    pub fn armed(&self) -> bool {
        self.base_mode & ARMED_FLAG != 0
    }

    fn apply(&mut self, header: &MavHeader, message: &MavMessage, timestamp_us: u64) {
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
    }

    pub fn snapshot(&self) -> Value {
        json!({
            "id": self.id,
            "component": self.component,
            "autopilot": self.autopilot,
            "vehicleType": self.vehicle_type,
            "armed": self.armed(),
            "customMode": self.custom_mode,
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
    pub fn on_frame(&mut self, header: &MavHeader, message: &MavMessage, timestamp_us: u64) {
        if let MavMessage::HEARTBEAT(h) = message {
            let (kind, autopilot) = (h.mavtype as u8, h.autopilot as u8);
            if kind != TYPE_GCS && autopilot != AUTOPILOT_INVALID && header.system_id != 0 && !self.vehicles.contains_key(&header.system_id) {
                self.vehicles.insert(header.system_id, Vehicle::new(header.system_id, header.component_id, autopilot, kind));
                self.active.get_or_insert(header.system_id);
            }
        }
        if let Some(vehicle) = self.vehicles.get_mut(&header.system_id) {
            vehicle.apply(header, message, timestamp_us);
        }
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

pub fn core_vehicle_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    let mut hub = HUB.lock().unwrap();
    hub.expire(now_us());
    hub.snapshot_of(args.first().and_then(|a| a.trim().parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_log_builds_one_vehicle_with_its_facts_and_counts() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
        let mut hub = Hub::default();
        let summary = crate::tlog::parse(&bytes);
        crate::tlog::for_each(&bytes, |ts, header, message| hub.on_frame(header, message, ts));
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
        hub.on_frame(&MavHeader { system_id: 255, component_id: 190, sequence: 0 }, &MavMessage::HEARTBEAT(gcs), 0);
        assert_eq!(hub.snapshot()["available"], false);
        let mut quad = HEARTBEAT_DATA::default();
        quad.mavtype = MavType::MAV_TYPE_QUADROTOR;
        quad.autopilot = MavAutopilot::MAV_AUTOPILOT_PX4;
        quad.base_mode = mavlink::dialects::ardupilotmega::MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED;
        hub.on_frame(&MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 5);
        let snapshot = hub.snapshot();
        assert_eq!((snapshot["vehicle"]["armed"].as_bool(), snapshot["vehicle"]["autopilot"].as_u64(), snapshot["vehicle"]["heartbeats"].as_u64()), (Some(true), Some(12), Some(1)));
        assert!(hub.expire(5 + CONNECTION_LOST_US).is_empty());
        assert_eq!(hub.expire(6 + CONNECTION_LOST_US), vec![1]);
        assert_eq!(hub.snapshot()["available"], false);
        let mut two = Hub::default();
        two.on_frame(&MavHeader { system_id: 1, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad.clone()), 5);
        two.on_frame(&MavHeader { system_id: 7, component_id: 1, sequence: 0 }, &MavMessage::HEARTBEAT(quad), 6);
        assert_eq!(two.snapshot()["vehicle"]["id"], 1);
        assert_eq!(two.snapshot_of(Some(7))["vehicle"]["id"], 7);
        assert_eq!(two.snapshot_of(Some(9))["available"], false);
    }
}
