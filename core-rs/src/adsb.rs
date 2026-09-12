use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use mavlink::dialects::ardupilotmega::{AdsbAltitudeType, AdsbEmitterType, AdsbFlags, MavMessage};
use serde_json::{Value, json};

use crate::read::{flag, object, truthy, value_string};
use crate::router::Backend;
use crate::track::{azimuth_deg, distance_m};

pub const DEPS: &[&str] = &[
    "settings.adsbVehicleManagerSettings.adsbServerConnectEnabled.rawValue",
    "settings.adsbVehicleManagerSettings.adsbServerHostAddress.rawValue",
    "settings.adsbVehicleManagerSettings.adsbServerPort.rawValue",
    "vehicle.coordinate",
];

pub const EXPIRATION_MS: u64 = 120_000;
pub const PUBLISH_INTERVAL_MS: u64 = 1_000;
pub const STALE_MS: u64 = 3_000;
pub const MAX_SECONDS_SINCE_LAST_SEEN: u32 = 15;
pub const MOVED_METRES: f64 = 1.0;

const RETRY: Duration = Duration::from_secs(1);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

pub const FEET_TO_METRES: f64 = 0.3048;
pub const KNOTS_TO_METRES_PER_SECOND: f64 = 0.514444;
pub const FEET_PER_MINUTE_TO_METRES_PER_SECOND: f64 = 0.00508;
pub const CENTIMETRES_PER_METRE: f64 = 100.0;
pub const MILLIMETRES_PER_METRE: f64 = 1000.0;
pub const COORDINATE_SCALE: f64 = 1e7;
const NEAR_ZERO: f64 = 1e-12;

pub const IDENTIFICATION_AND_CATEGORY: u32 = 1;
pub const SURFACE_POSITION: u32 = 2;
pub const AIRBORNE_POSITION: u32 = 3;
pub const AIRBORNE_VELOCITY: u32 = 4;
pub const SURVEILLANCE_ALTITUDE: u32 = 5;
pub const SURVEILLANCE_ID: u32 = 6;

pub const FIELD_ICAO: usize = 4;
pub const FIELD_CALLSIGN: usize = 10;
pub const FIELD_ALTITUDE: usize = 11;
pub const FIELD_GROUND_SPEED: usize = 12;
pub const FIELD_TRACK: usize = 13;
pub const FIELD_LATITUDE: usize = 14;
pub const FIELD_LONGITUDE: usize = 15;
pub const FIELD_VERTICAL_RATE: usize = 16;
pub const FIELD_EMERGENCY: usize = 19;

pub const ALTITUDE_PRESSURE_QNH: &str = "pressureQnh";
pub const ALTITUDE_GEOMETRIC: &str = "geometric";

pub const CONNECT_FAILED: &str = "connectFailed";
pub const LINK_LOST: &str = "linkLost";

pub const EMERGENCIES: [(u16, &str); 3] = [(7500, "hijack"), (7600, "radioFailure"), (7700, "general")];

pub fn wrap_heading(degrees: f64) -> f64 {
    degrees.rem_euclid(360.0)
}

pub fn coordinate(latitude: f64, longitude: f64) -> Option<(f64, f64)> {
    ((-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude)).then_some((latitude, longitude))
}

pub fn emergency_token(squawk: u16) -> Option<&'static str> {
    EMERGENCIES.iter().find(|(code, _)| *code == squawk).map(|(_, token)| *token)
}

pub fn altitude_token(kind: AdsbAltitudeType) -> &'static str {
    match kind {
        AdsbAltitudeType::ADSB_ALTITUDE_TYPE_GEOMETRIC => ALTITUDE_GEOMETRIC,
        AdsbAltitudeType::ADSB_ALTITUDE_TYPE_PRESSURE_QNH => ALTITUDE_PRESSURE_QNH,
    }
}

pub fn emitter_token(kind: AdsbEmitterType) -> Option<&'static str> {
    match kind {
        AdsbEmitterType::ADSB_EMITTER_TYPE_NO_INFO => None,
        AdsbEmitterType::ADSB_EMITTER_TYPE_LIGHT => Some("light"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_SMALL => Some("small"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_LARGE => Some("large"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_HIGH_VORTEX_LARGE => Some("highVortexLarge"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_HEAVY => Some("heavy"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_HIGHLY_MANUV => Some("highlyManoeuvrable"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_ROTOCRAFT => Some("rotorcraft"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_GLIDER => Some("glider"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_LIGHTER_AIR => Some("lighterThanAir"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_PARACHUTE => Some("parachute"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_ULTRA_LIGHT => Some("ultraLight"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_UAV => Some("uav"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_SPACE => Some("space"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_EMERGENCY_SURFACE => Some("emergencySurface"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_SERVICE_SURFACE => Some("serviceSurface"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_POINT_OBSTACLE => Some("pointObstacle"),
        AdsbEmitterType::ADSB_EMITTER_TYPE_UNASSIGNED | AdsbEmitterType::ADSB_EMITTER_TYPE_UNASSIGNED2 | AdsbEmitterType::ADSB_EMITTER_TYPE_UNASSGINED3 => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Own {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_metres: Option<f64>,
}

impl Own {
    fn position(&self) -> (f64, f64) {
        (self.latitude, self.longitude)
    }
}

pub fn own(json: &str) -> Option<Own> {
    let coordinate = object(json);
    let number = |key: &str| coordinate.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
    let (latitude, longitude) = flag(&coordinate, "valid")
        .then(|| number("latitude").zip(number("longitude")))
        .flatten()
        .filter(|(latitude, longitude)| *latitude != 0.0 || *longitude != 0.0)?;
    Some(Own { latitude, longitude, altitude_metres: number("altitude") })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub host: String,
    pub port: u16,
}

impl Source {
    fn address(&self) -> Option<std::net::SocketAddr> {
        (self.host.as_str(), self.port).to_socket_addrs().ok()?.next()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub token: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub icao_address: u32,
    pub callsign: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude_metres: Option<f64>,
    pub altitude_type: Option<&'static str>,
    pub heading_degrees: Option<f64>,
    pub velocity_metres_per_second: Option<f64>,
    pub vertical_velocity_metres_per_second: Option<f64>,
    pub squawk: Option<u16>,
    pub alert: Option<bool>,
    pub emitter_type: Option<&'static str>,
    pub seconds_since_last_seen: Option<u32>,
    pub simulated: bool,
}

impl Report {
    pub fn positioned(&self) -> Option<(f64, f64)> {
        self.latitude.zip(self.longitude)
    }

    pub fn emergency(&self) -> Option<&'static str> {
        self.squawk.and_then(emergency_token)
    }

    fn merged(&self, next: &Report) -> Report {
        Report {
            icao_address: self.icao_address,
            callsign: next.callsign.clone().or_else(|| self.callsign.clone()),
            latitude: next.latitude.or(self.latitude),
            longitude: next.longitude.or(self.longitude),
            altitude_metres: next.altitude_metres.or(self.altitude_metres),
            altitude_type: next.altitude_type.or(self.altitude_type),
            heading_degrees: next.heading_degrees.or(self.heading_degrees),
            velocity_metres_per_second: next.velocity_metres_per_second.or(self.velocity_metres_per_second),
            vertical_velocity_metres_per_second: next.vertical_velocity_metres_per_second.or(self.vertical_velocity_metres_per_second),
            squawk: next.squawk.or(self.squawk),
            alert: next.alert.or(self.alert),
            emitter_type: next.emitter_type.or(self.emitter_type),
            seconds_since_last_seen: next.seconds_since_last_seen,
            simulated: next.simulated || self.simulated,
        }
    }
}

pub fn decode(message: &MavMessage) -> Option<Report> {
    let MavMessage::ADSB_VEHICLE(data) = message else { return None };
    if data.tslc as u32 > MAX_SECONDS_SINCE_LAST_SEEN {
        return None;
    }
    let has = |flag: AdsbFlags| data.flags.contains(flag);
    let position = has(AdsbFlags::ADSB_FLAGS_VALID_COORDS).then(|| coordinate(data.lat as f64 / COORDINATE_SCALE, data.lon as f64 / COORDINATE_SCALE)).flatten();
    let altitude = has(AdsbFlags::ADSB_FLAGS_VALID_ALTITUDE).then(|| data.altitude as f64 / MILLIMETRES_PER_METRE);
    let callsign = has(AdsbFlags::ADSB_FLAGS_VALID_CALLSIGN)
        .then(|| data.callsign.to_str().ok().map(|text| text.trim().to_string()))
        .flatten()
        .filter(|text| !text.is_empty());
    Some(Report {
        icao_address: data.ICAO_address,
        callsign,
        latitude: position.map(|(latitude, _)| latitude),
        longitude: position.map(|(_, longitude)| longitude),
        altitude_metres: altitude,
        altitude_type: altitude.map(|_| altitude_token(data.altitude_type)),
        heading_degrees: has(AdsbFlags::ADSB_FLAGS_VALID_HEADING).then(|| wrap_heading(data.heading as f64 / CENTIMETRES_PER_METRE)),
        velocity_metres_per_second: has(AdsbFlags::ADSB_FLAGS_VALID_VELOCITY).then(|| data.hor_velocity as f64 / CENTIMETRES_PER_METRE),
        vertical_velocity_metres_per_second: has(AdsbFlags::ADSB_FLAGS_VERTICAL_VELOCITY_VALID).then(|| data.ver_velocity as f64 / CENTIMETRES_PER_METRE),
        squawk: has(AdsbFlags::ADSB_FLAGS_VALID_SQUAWK).then_some(data.squawk),
        alert: None,
        emitter_type: emitter_token(data.emitter_type),
        seconds_since_last_seen: Some(data.tslc as u32),
        simulated: has(AdsbFlags::ADSB_FLAGS_SIMULATED),
    })
}

fn number(field: Option<&&str>) -> Option<f64> {
    field?.trim().parse().ok()
}

fn sbs1_callsign(icao_address: u32, values: &[&str]) -> Option<Report> {
    let callsign = values.get(FIELD_CALLSIGN)?.trim();
    (!callsign.is_empty()).then(|| Report { icao_address, callsign: Some(callsign.to_string()), ..Report::default() })
}

fn sbs1_position(icao_address: u32, values: &[&str]) -> Option<Report> {
    let emergency = number(values.get(FIELD_EMERGENCY))?;
    let stated = values.get(FIELD_ALTITUDE)?.trim();
    let geometric = stated.ends_with('H');
    let feet: i64 = stated.strip_suffix('H').unwrap_or(stated).parse().ok()?;
    let (latitude, longitude) = (number(values.get(FIELD_LATITUDE))?, number(values.get(FIELD_LONGITUDE))?);
    if latitude.abs() <= NEAR_ZERO && longitude.abs() <= NEAR_ZERO {
        return None;
    }
    let (latitude, longitude) = coordinate(latitude, longitude)?;
    Some(Report {
        icao_address,
        latitude: Some(latitude),
        longitude: Some(longitude),
        altitude_metres: Some(feet as f64 * FEET_TO_METRES),
        altitude_type: Some(if geometric { ALTITUDE_GEOMETRIC } else { ALTITUDE_PRESSURE_QNH }),
        alert: Some(emergency == 1.0),
        ..Report::default()
    })
}

fn sbs1_velocity(icao_address: u32, values: &[&str]) -> Option<Report> {
    let knots = number(values.get(FIELD_GROUND_SPEED))?;
    let heading = number(values.get(FIELD_TRACK))?;
    Some(Report {
        icao_address,
        heading_degrees: Some(wrap_heading(heading)),
        velocity_metres_per_second: Some(knots * KNOTS_TO_METRES_PER_SECOND),
        vertical_velocity_metres_per_second: number(values.get(FIELD_VERTICAL_RATE)).map(|feet_per_minute| feet_per_minute * FEET_PER_MINUTE_TO_METRES_PER_SECOND),
        ..Report::default()
    })
}

pub fn parse_sbs1(line: &str) -> Option<Report> {
    let text = line.trim_end_matches(['\r', '\n']);
    if text.chars().count() <= 4 || !text.starts_with("MSG") {
        return None;
    }
    let kind = text.chars().nth(4)?.to_digit(10)?;
    if kind == SURFACE_POSITION || kind > SURVEILLANCE_ID {
        return None;
    }
    let values: Vec<&str> = text.split(',').collect();
    let icao_address = u32::from_str_radix(values.get(FIELD_ICAO)?.trim(), 16).ok()?;
    match kind {
        IDENTIFICATION_AND_CATEGORY | SURVEILLANCE_ALTITUDE | SURVEILLANCE_ID => sbs1_callsign(icao_address, &values),
        AIRBORNE_POSITION => sbs1_position(icao_address, &values),
        AIRBORNE_VELOCITY => sbs1_velocity(icao_address, &values),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub report: Report,
    pub last_contact_ms: u64,
    pub last_published_ms: u64,
}

impl Contact {
    fn new(report: &Report, now_ms: u64) -> Option<Contact> {
        report.positioned()?;
        Some(Contact { report: report.clone(), last_contact_ms: now_ms, last_published_ms: now_ms })
    }

    fn update(&mut self, report: &Report, now_ms: u64) -> bool {
        self.last_contact_ms = now_ms;
        let previous = self.report.clone();
        let merged = previous.merged(report);
        let flown = report
            .heading_degrees
            .is_none()
            .then(|| previous.positioned().zip(merged.positioned()))
            .flatten()
            .filter(|(before, after)| distance_m(*before, *after) > MOVED_METRES)
            .map(|(before, after)| azimuth_deg(before, after));
        self.report = Report { heading_degrees: flown.or(merged.heading_degrees), ..merged };
        let alert_change = report.alert.is_some_and(|alert| Some(alert) != previous.alert);
        if !alert_change && now_ms.saturating_sub(self.last_published_ms) < PUBLISH_INTERVAL_MS {
            return false;
        }
        self.last_published_ms = now_ms;
        self.report != previous
    }

    pub fn expired(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.last_contact_ms) > EXPIRATION_MS
    }

    pub fn stale(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.last_contact_ms) > STALE_MS
    }

    pub fn range_metres(&self, own: Option<Own>) -> Option<f64> {
        own.zip(self.report.positioned()).map(|(own, position)| distance_m(own.position(), position))
    }

    pub fn json(&self, now_ms: u64, own: Option<Own>) -> Value {
        json!({
            "icaoAddress": self.report.icao_address,
            "callsign": self.report.callsign,
            "latitude": self.report.latitude,
            "longitude": self.report.longitude,
            "altitudeMetres": self.report.altitude_metres,
            "altitudeType": self.report.altitude_type,
            "headingDegrees": self.report.heading_degrees,
            "velocityMetresPerSecond": self.report.velocity_metres_per_second,
            "verticalVelocityMetresPerSecond": self.report.vertical_velocity_metres_per_second,
            "squawk": self.report.squawk,
            "emergency": self.report.emergency(),
            "alert": self.report.alert,
            "emitterType": self.report.emitter_type,
            "secondsSinceLastSeen": self.report.seconds_since_last_seen,
            "simulated": self.report.simulated,
            "ageMs": now_ms.saturating_sub(self.last_contact_ms),
            "stale": self.stale(now_ms),
            "distanceMetres": self.range_metres(own),
            "bearingDegrees": own.zip(self.report.positioned()).map(|(own, position)| azimuth_deg(own.position(), position)),
            "relativeAltitudeMetres": own.and_then(|own| own.altitude_metres).zip(self.report.altitude_metres).map(|(mine, theirs)| theirs - mine),
        })
    }
}

#[derive(Debug, Default)]
pub struct Traffic {
    contacts: BTreeMap<u32, Contact>,
    own: Option<Own>,
    enabled: bool,
    source: Option<Source>,
    generation: u64,
    connected: bool,
    failure: Option<Failure>,
    mavlink_ms: Option<u64>,
    revision: u64,
    updated_ms: Option<u64>,
    announced: u64,
    stale_announced: usize,
}

impl Traffic {
    pub fn observe(&mut self, own: Option<Own>) {
        self.own = own;
    }

    pub fn retarget(&mut self, enabled: bool, source: Option<Source>) -> bool {
        if (self.enabled, &self.source) == (enabled, &source) {
            return false;
        }
        self.enabled = enabled;
        self.source = source;
        self.generation += 1;
        self.connected = false;
        self.failure = None;
        true
    }

    pub fn attached(&mut self, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        self.connected = true;
        self.failure = None;
        true
    }

    pub fn failed(&mut self, generation: u64, token: &'static str, detail: String) -> bool {
        if generation != self.generation {
            return false;
        }
        self.connected = false;
        self.failure = Some(Failure { token, detail });
        true
    }

    pub fn receive(&mut self, report: &Report, now_ms: u64) -> bool {
        self.expire(now_ms);
        let changed = match self.contacts.get_mut(&report.icao_address) {
            Some(contact) => contact.update(report, now_ms),
            None => Contact::new(report, now_ms).is_some_and(|contact| self.contacts.insert(report.icao_address, contact).is_none()),
        };
        if changed {
            self.revision += 1;
            self.updated_ms = Some(now_ms);
        }
        changed
    }

    pub fn on_message(&mut self, message: &MavMessage, now_ms: u64) -> bool {
        let Some(report) = decode(message) else { return false };
        self.mavlink_ms = Some(now_ms);
        self.receive(&report, now_ms)
    }

    pub fn on_sbs1_line(&mut self, line: &str, now_ms: u64) -> bool {
        parse_sbs1(line).is_some_and(|report| self.receive(&report, now_ms))
    }

    pub fn expire(&mut self, now_ms: u64) -> Vec<u32> {
        let (alive, gone): (BTreeMap<u32, Contact>, BTreeMap<u32, Contact>) = std::mem::take(&mut self.contacts).into_iter().partition(|(_, contact)| !contact.expired(now_ms));
        self.contacts = alive;
        gone.into_keys().collect()
    }

    pub fn tick(&mut self, now_ms: u64) -> bool {
        let gone = !self.expire(now_ms).is_empty();
        let stale = self.contacts.values().filter(|contact| contact.stale(now_ms)).count();
        let due = gone || self.revision != self.announced || stale != self.stale_announced;
        self.announced = self.revision;
        self.stale_announced = stale;
        due
    }

    pub fn contact(&self, icao_address: u32) -> Option<&Contact> {
        self.contacts.get(&icao_address)
    }

    pub fn count(&self) -> usize {
        self.contacts.len()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn receiving(&self, now_ms: u64) -> bool {
        self.connected || self.mavlink_ms.is_some_and(|heard| now_ms.saturating_sub(heard) <= EXPIRATION_MS)
    }

    fn ranked(&self) -> Vec<&Contact> {
        self.contacts
            .values()
            .map(|contact| {
                let range = contact.range_metres(self.own);
                ((range.is_none(), range.map(f64::to_bits), contact.report.icao_address), contact)
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()
    }

    fn alerting(&self) -> Value {
        let stated: Vec<bool> = self.contacts.values().filter_map(|contact| contact.report.alert).collect();
        match (stated.iter().any(|alert| *alert), stated.is_empty()) {
            (true, _) => Value::Bool(true),
            (false, true) => Value::Null,
            (false, false) => Value::Bool(false),
        }
    }

    pub fn snapshot(&self, now_ms: u64) -> Value {
        json!({
            "kind": "object",
            "class": "AdsbTraffic",
            "enabled": self.enabled,
            "available": self.source.is_some(),
            "connected": self.connected,
            "receiving": self.receiving(now_ms),
            "source": self.source.as_ref().map(|source| json!({ "host": source.host, "port": source.port })),
            "error": self.failure.as_ref().map(|failure| json!({ "token": failure.token, "detail": failure.detail })),
            "mavlinkAgeMs": self.mavlink_ms.map(|heard| now_ms.saturating_sub(heard)),
            "ownPositionKnown": self.own.is_some(),
            "count": self.contacts.len(),
            "expirationMs": EXPIRATION_MS,
            "staleMs": STALE_MS,
            "publishIntervalMs": PUBLISH_INTERVAL_MS,
            "revision": self.revision,
            "updatedMs": self.updated_ms,
            "units": { "altitude": "m", "velocity": "m/s", "verticalVelocity": "m/s", "heading": "deg", "distance": "m", "bearing": "deg" },
            "alerting": self.alerting(),
            "alertUnknown": self.contacts.values().filter(|contact| contact.report.alert.is_none()).count(),
            "emergency": EMERGENCIES.iter().find(|(code, _)| self.contacts.values().any(|contact| contact.report.squawk == Some(*code))).map(|(_, token)| *token),
            "order": "nearestFirst",
            "contacts": self.ranked().iter().map(|contact| contact.json(now_ms, self.own)).collect::<Vec<_>>(),
        })
    }
}

pub static TRAFFIC: LazyLock<Mutex<Traffic>> = LazyLock::new(|| Mutex::new(Traffic::default()));
pub static ON_CHANGE: Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = Mutex::new(None);

pub fn lock() -> MutexGuard<'static, Traffic> {
    TRAFFIC.lock().unwrap_or_else(PoisonError::into_inner)
}

fn changed() {
    let hook = ON_CHANGE.lock().unwrap_or_else(PoisonError::into_inner).clone();
    if let Some(hook) = hook {
        hook();
    }
}

pub fn on_message(message: &MavMessage, now_ms: u64) -> bool {
    lock().on_message(message, now_ms)
}

fn follow(source: Source, generation: u64) {
    while lock().generation == generation {
        let opened = source.address().ok_or_else(|| format!("{} does not resolve", source.host)).and_then(|address| TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).map_err(|error| error.to_string()));
        match opened {
            Err(detail) => {
                if lock().failed(generation, CONNECT_FAILED, detail) {
                    changed();
                }
            }
            Ok(stream) => {
                if lock().attached(generation) {
                    changed();
                }
                BufReader::new(stream)
                    .lines()
                    .map_while(Result::ok)
                    .take_while(|_| lock().generation == generation)
                    .for_each(|line| {
                        if lock().on_sbs1_line(&line, crate::hub::now_ms()) {
                            changed();
                        }
                    });
                if lock().failed(generation, LINK_LOST, "the ADSB server closed the connection".to_string()) {
                    changed();
                }
            }
        }
        std::thread::sleep(RETRY);
    }
}

fn port_number(json: &str) -> Option<u16> {
    let value = object(json).get("value")?.clone();
    let number = value.as_f64().or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))?;
    (1.0..=65535.0).contains(&number).then_some(number as u16)
}

pub fn adsb_traffic_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let enabled = truthy(&object(&backend.get(DEPS[0])), "value");
    let host = value_string(&backend.get(DEPS[1])).trim().to_string();
    let source = port_number(&backend.get(DEPS[2])).filter(|_| !host.is_empty()).map(|port| Source { host, port });
    let now_ms = crate::hub::now_ms();
    let mut traffic = lock();
    traffic.observe(own(&backend.get(DEPS[3])));
    if traffic.retarget(enabled, source.clone())
        && enabled
        && let Some(source) = source
    {
        let generation = traffic.generation;
        std::thread::Builder::new().name("qgc-core-adsb".to_string()).spawn(move || follow(source, generation)).expect("adsb thread");
    }
    traffic.expire(now_ms);
    traffic.snapshot(now_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::ADSB_VEHICLE_DATA;
    use mavlink::types::CharArray;

    fn close(value: Option<f64>, expected: f64) -> bool {
        value.is_some_and(|value| (value - expected).abs() < 1e-9)
    }

    fn everything() -> AdsbFlags {
        AdsbFlags::ADSB_FLAGS_VALID_COORDS
            | AdsbFlags::ADSB_FLAGS_VALID_ALTITUDE
            | AdsbFlags::ADSB_FLAGS_VALID_HEADING
            | AdsbFlags::ADSB_FLAGS_VALID_VELOCITY
            | AdsbFlags::ADSB_FLAGS_VALID_CALLSIGN
            | AdsbFlags::ADSB_FLAGS_VALID_SQUAWK
            | AdsbFlags::ADSB_FLAGS_VERTICAL_VELOCITY_VALID
    }

    fn vehicle(flags: AdsbFlags, tslc: u8) -> MavMessage {
        MavMessage::ADSB_VEHICLE(ADSB_VEHICLE_DATA {
            ICAO_address: 0xABCDEF,
            lat: 475_000_000,
            lon: 85_000_000,
            altitude: 1_500_000,
            heading: 27_000,
            hor_velocity: 12_000,
            ver_velocity: -300,
            flags,
            squawk: 7700,
            altitude_type: AdsbAltitudeType::ADSB_ALTITUDE_TYPE_GEOMETRIC,
            callsign: CharArray::from("SWR123 "),
            emitter_type: AdsbEmitterType::ADSB_EMITTER_TYPE_LIGHT,
            tslc,
        })
    }

    fn located(icao_address: u32) -> Report {
        Report { icao_address, latitude: Some(47.5), longitude: Some(8.5), ..Report::default() }
    }

    fn sbs1(kind: u32, fields: &[(usize, &str)]) -> String {
        (0..22)
            .map(|at| match at {
                0 => "MSG".to_string(),
                1 => kind.to_string(),
                _ => fields.iter().find(|(index, _)| *index == at).map(|(_, text)| (*text).to_string()).unwrap_or_default(),
            })
            .collect::<Vec<String>>()
            .join(",")
    }

    fn position_line(fields: &[(usize, &str)]) -> String {
        let base = [(FIELD_ICAO, "ABCDEF"), (FIELD_ALTITUDE, "4000"), (FIELD_LATITUDE, "47.5"), (FIELD_LONGITUDE, "8.5"), (FIELD_EMERGENCY, "0")];
        let merged: Vec<(usize, &str)> = base.iter().filter(|(index, _)| !fields.iter().any(|(other, _)| other == index)).chain(fields.iter()).copied().collect();
        sbs1(AIRBORNE_POSITION, &merged)
    }

    #[test]
    fn every_reading_is_decoded_only_when_its_own_validity_bit_is_set() {
        let report = decode(&vehicle(everything(), 3)).unwrap();
        assert_eq!((report.icao_address, report.callsign.as_deref()), (0xABCDEF, Some("SWR123")));
        assert_eq!((report.latitude, report.longitude, report.altitude_metres), (Some(47.5), Some(8.5), Some(1500.0)));
        assert_eq!((report.heading_degrees, report.velocity_metres_per_second), (Some(270.0), Some(120.0)));
        assert_eq!(report.vertical_velocity_metres_per_second, Some(-3.0), "ver_velocity is centimetres per second, so a head told metres must be given metres");
        assert_eq!((report.squawk, report.altitude_type, report.emitter_type), (Some(7700), Some(ALTITUDE_GEOMETRIC), Some("light")));
        assert_eq!((report.seconds_since_last_seen, report.simulated), (Some(3), false));
        assert_eq!(report.alert, None, "ADSB_VEHICLE carries no alert bit, and an absent collision warning is unknown, never a granted all-clear");

        let bare = decode(&vehicle(AdsbFlags::empty(), 0)).unwrap();
        assert_eq!(bare, Report { icao_address: 0xABCDEF, seconds_since_last_seen: Some(0), emitter_type: Some("light"), ..Report::default() }, "a reading with no validity bit is absent, not zero");
        assert!(decode(&vehicle(everything(), MAX_SECONDS_SINCE_LAST_SEEN as u8 + 1)).is_none(), "a contact last seen longer ago than the message allows is not reported at all");
        assert!(decode(&vehicle(everything(), MAX_SECONDS_SINCE_LAST_SEEN as u8)).is_some(), "and the boundary itself still is");
        assert!(decode(&MavMessage::HEARTBEAT(Default::default())).is_none());
    }

    #[test]
    fn a_squawk_that_names_an_emergency_is_decoded_into_one() {
        assert_eq!(emergency_token(7500), Some("hijack"));
        assert_eq!(emergency_token(7600), Some("radioFailure"));
        assert_eq!(emergency_token(7700), Some("general"));
        assert_eq!(emergency_token(1200), None, "an ordinary VFR squawk is no emergency");
        assert_eq!(Report { squawk: None, ..located(1) }.emergency(), None, "and an unknown squawk is not a stated all-clear either");

        let mut traffic = Traffic::default();
        assert!(traffic.on_message(&vehicle(everything(), 1), 0), "the fixture squawks 7700");
        let view = traffic.snapshot(0);
        assert_eq!(view["contacts"][0]["emergency"], "general", "the head cannot be asked to carry the aviation table, and it is the only distress signal a MAVLink feed carries at all");
        assert_eq!(view["contacts"][0]["squawk"], 7700, "the raw code stays beside the token");
        assert_eq!(view["emergency"], "general", "and the summary says one is being squawked without the head walking the list");
        assert_eq!(Traffic::default().snapshot(0)["emergency"], Value::Null);
    }

    #[test]
    fn an_emitter_with_no_information_and_a_broken_coordinate_stay_absent() {
        assert_eq!(emitter_token(AdsbEmitterType::ADSB_EMITTER_TYPE_NO_INFO), None, "NO_INFO is the sender saying it does not know, so it is absent rather than a category");
        assert_eq!(emitter_token(AdsbEmitterType::ADSB_EMITTER_TYPE_UNASSIGNED), None);
        assert_eq!(altitude_token(AdsbAltitudeType::ADSB_ALTITUDE_TYPE_PRESSURE_QNH), ALTITUDE_PRESSURE_QNH);
        let unusable = MavMessage::ADSB_VEHICLE(ADSB_VEHICLE_DATA { lat: 1_000_000_000, ..match vehicle(everything(), 0) { MavMessage::ADSB_VEHICLE(d) => d, _ => unreachable!() } });
        let report = decode(&unusable).unwrap();
        assert_eq!((report.latitude, report.longitude), (None, None), "a latitude outside the sphere is no position, and half a position is none");
        assert_eq!(report.altitude_metres, Some(1500.0), "the other readings in the same message survive it");
    }

    #[test]
    fn a_coordinate_the_sender_could_not_have_meant_is_refused_rather_than_moved() {
        assert_eq!(coordinate(47.5, 8.5), Some((47.5, 8.5)));
        assert_eq!(coordinate(47.5, 180.0), Some((47.5, 180.0)), "the antimeridian is a real longitude");
        assert_eq!(coordinate(47.5, 200.0), None, "an ADSB longitude is an int32 at 1e7 and cannot overflow the globe, so 200 degrees is corrupt data - wrapping it would draw a phantom target up to a third of the way round the world from the truth");
        assert_eq!(coordinate(47.5, -200.0), None);
        assert_eq!(coordinate(91.0, 8.5), None);
        assert_eq!(coordinate(f64::NAN, 8.5), None);
        assert_eq!(coordinate(47.5, f64::INFINITY), None);
        assert_eq!(wrap_heading(-10.0), 350.0, "a heading is an angle and every angle names a real direction, so it wraps");
        assert_eq!(wrap_heading(370.0), 10.0);
    }

    #[test]
    fn sbs1_lines_decode_the_fields_their_message_type_carries() {
        let identity = parse_sbs1(&sbs1(IDENTIFICATION_AND_CATEGORY, &[(FIELD_ICAO, "ABCDEF"), (FIELD_CALLSIGN, " SWR123 ")])).unwrap();
        assert_eq!((identity.icao_address, identity.callsign.as_deref()), (0xABCDEF, Some("SWR123")));
        assert_eq!(identity.latitude, None, "an identification line says nothing about where the aircraft is");
        assert!(parse_sbs1(&sbs1(IDENTIFICATION_AND_CATEGORY, &[(FIELD_ICAO, "ABCDEF"), (FIELD_CALLSIGN, "   ")])).is_none(), "a blank callsign is no callsign");
        assert!(parse_sbs1(&sbs1(SURVEILLANCE_ALTITUDE, &[(FIELD_ICAO, "ABCDEF"), (FIELD_CALLSIGN, "SWR123")])).is_some());
        assert!(parse_sbs1(&sbs1(SURVEILLANCE_ID, &[(FIELD_ICAO, "ABCDEF"), (FIELD_CALLSIGN, "SWR123")])).is_some());

        let position = parse_sbs1(&position_line(&[(FIELD_ALTITUDE, "4000H"), (FIELD_EMERGENCY, "1")])).unwrap();
        assert_eq!((position.latitude, position.longitude), (Some(47.5), Some(8.5)));
        assert!(close(position.altitude_metres, 1219.2), "4000 ft is 1219.2 m and the core hands over metres; got {:?}", position.altitude_metres);
        assert_eq!((position.altitude_type, position.alert), (Some(ALTITUDE_GEOMETRIC), Some(true)), "the H suffix is the sender saying the altitude came from GNSS, not a barometer");
        assert_eq!(parse_sbs1(&position_line(&[])).unwrap().altitude_type, Some(ALTITUDE_PRESSURE_QNH));
        assert_eq!(parse_sbs1(&position_line(&[])).unwrap().alert, Some(false), "an emergency field that reads zero is a stated all-clear, unlike a missing one");
        assert!(parse_sbs1(&position_line(&[(FIELD_LATITUDE, "0.0"), (FIELD_LONGITUDE, "0.0")])).is_none(), "a null island fix is the sender having no position");
        assert!(parse_sbs1(&position_line(&[(FIELD_ALTITUDE, "")])).is_none(), "a position line without an altitude is refused whole, as the C++ link does");
        assert!(parse_sbs1(&position_line(&[(FIELD_EMERGENCY, "")])).is_none(), "and so is one without the emergency field");
        assert!(parse_sbs1("MSG,3,1,1,ABCDEF,1,,,,,,4000,,,47.5,8.5").is_none(), "a truncated line never indexes past its end");
        assert!(parse_sbs1(&position_line(&[(FIELD_LONGITUDE, "200.0")])).is_none(), "and a longitude off the globe is refused rather than wrapped onto it");

        let velocity = parse_sbs1(&sbs1(AIRBORNE_VELOCITY, &[(FIELD_ICAO, "ABCDEF"), (FIELD_GROUND_SPEED, "100.0"), (FIELD_TRACK, "-45.0"), (FIELD_VERTICAL_RATE, "64.0")])).unwrap();
        assert!(close(velocity.velocity_metres_per_second, 51.4444), "100 kt is 51.4444 m/s; got {:?}", velocity.velocity_metres_per_second);
        assert_eq!(velocity.heading_degrees, Some(315.0), "a negative track is the same bearing, wrapped");
        assert!(close(velocity.vertical_velocity_metres_per_second, 0.32512), "64 ft/min is 0.32512 m/s; got {:?}", velocity.vertical_velocity_metres_per_second);
        assert_eq!(
            parse_sbs1(&sbs1(AIRBORNE_VELOCITY, &[(FIELD_ICAO, "ABCDEF"), (FIELD_GROUND_SPEED, "100.0"), (FIELD_TRACK, "45.0")])).unwrap().vertical_velocity_metres_per_second,
            None,
            "an unreadable climb rate leaves the climb rate absent"
        );
        assert!(parse_sbs1(&sbs1(AIRBORNE_VELOCITY, &[(FIELD_ICAO, "ABCDEF"), (FIELD_TRACK, "45.0"), (FIELD_VERTICAL_RATE, "64")])).is_none(), "heading and speed arrive together or not at all");
    }

    #[test]
    fn the_alert_that_paints_the_collision_icon_is_the_emergency_field_not_the_squawk_change() {
        let squawk_changed = parse_sbs1(&position_line(&[(18, "1"), (FIELD_EMERGENCY, "0")])).unwrap();
        assert_eq!(
            squawk_changed.alert,
            Some(false),
            "SBS-1 field 19 means the squawk changed, which is an ordinary handoff; the flag this report feeds paints AlertAircraft.svg and bypasses the publish throttle, so reading it there would fire a red collision icon on every routine reassignment"
        );
        let emergency = parse_sbs1(&position_line(&[(18, "0"), (FIELD_EMERGENCY, "1")])).unwrap();
        assert_eq!(emergency.alert, Some(true), "field 20 is the emergency the C++ link read, and it is the one the icon means");
        assert_eq!(FIELD_EMERGENCY, 19, "zero based, that is the twentieth field");
    }

    #[test]
    fn a_contact_is_only_created_once_it_has_a_position() {
        let mut traffic = Traffic::default();
        let heard = Report { icao_address: 1, callsign: Some("SWR123".into()), ..Report::default() };
        assert!(!traffic.receive(&heard, 0), "a callsign alone cannot be drawn on a map, so it starts no contact");
        assert_eq!(traffic.count(), 0);
        assert!(traffic.receive(&located(1), 10), "the position it was waiting for creates it");
        assert_eq!(traffic.count(), 1);
        assert!(!traffic.receive(&located(1), 10), "and the same aircraft is one contact, not two");
        let contact = traffic.contact(1).unwrap();
        assert_eq!(contact.report.callsign, None, "the callsign heard before the contact existed was not retained, exactly as the C++ manager drops it");
    }

    #[test]
    fn a_throttled_report_is_still_merged_so_no_reading_is_silently_dropped() {
        let mut traffic = Traffic::default();
        assert!(traffic.receive(&located(1), 1_000));
        let moved = Report { latitude: Some(47.6), ..located(1) };
        assert!(!traffic.receive(&moved, 1_500), "a redraw inside the interval is skipped so a map is not repainted per message");
        assert_eq!(
            traffic.contact(1).unwrap().report.latitude,
            Some(47.6),
            "but the reading itself lands: SBS-1 interleaves position and velocity at 2 Hz each, so discarding whatever misses the slot freezes half the marker - a stale heading points conflicting traffic the wrong way"
        );
        let turning = Report { heading_degrees: Some(90.0), ..located(1) };
        assert!(!traffic.receive(&turning, 1_600));
        assert_eq!(traffic.contact(1).unwrap().report.heading_degrees, Some(90.0), "and so does the velocity line that arrived in the same window");
        assert!(traffic.receive(&Report { latitude: Some(47.7), ..located(1) }, 2_000), "the next report past the interval is published");
        assert!(!traffic.receive(&Report { latitude: Some(47.7), ..located(1) }, 3_000), "an update that changes nothing changes nothing");
    }

    #[test]
    fn the_keep_alive_is_refreshed_by_a_report_the_throttle_skipped() {
        let mut traffic = Traffic::default();
        assert!(traffic.receive(&located(1), 1_000));
        assert!(!traffic.receive(&Report { latitude: Some(47.6), ..located(1) }, 1_500), "throttled, and the only update this contact gets");
        assert!(traffic.expire(1_000 + EXPIRATION_MS).is_empty(), "the throttled report still refreshed the keep-alive, so the contact is not stale");
        assert_eq!(traffic.expire(1_500 + EXPIRATION_MS + 1), vec![1]);
        assert_eq!(traffic.count(), 0, "expiry removes the contact from the list and from the address map with it");
        assert!(!traffic.receive(&Report { callsign: Some("SWR123".into()), ..Report::default() }, 0), "an expired contact is gone, so a later callsign does not resurrect it");
    }

    #[test]
    fn expiry_runs_on_the_message_path_and_not_only_when_a_head_is_looking() {
        let mut traffic = Traffic::default();
        assert!(traffic.receive(&located(1), 1_000));
        let long_after = 1_000 + EXPIRATION_MS + 1;
        assert!(!traffic.receive(&Report { icao_address: 1, callsign: Some("SWR123".into()), ..Report::default() }, long_after), "a callsign line arriving ten minutes later must not merge into the dead contact");
        assert_eq!(traffic.count(), 0, "nothing polls the view while the traffic layer is hidden, so the message path is the only clock this list has");
        assert!(traffic.receive(&located(2), long_after));
        assert_eq!(traffic.count(), 1, "a live contact arriving at the same moment is kept");
    }

    #[test]
    fn a_contact_whose_last_fix_is_old_says_so_rather_than_leaving_each_head_to_guess() {
        let mut traffic = Traffic::default();
        assert!(traffic.receive(&located(1), 1_000));
        let fresh = traffic.snapshot(1_000 + STALE_MS);
        assert_eq!((fresh["contacts"][0]["stale"].clone(), fresh["contacts"][0]["ageMs"].clone()), (json!(false), json!(STALE_MS)));
        let old = traffic.snapshot(1_000 + STALE_MS + 1);
        assert_eq!(old["contacts"][0]["stale"], json!(true), "traffic arrives at about a hertz, so a fix seconds old is a ghost - at jet speed it is kilometres from where it is drawn");
        assert_eq!(old["staleMs"], json!(STALE_MS), "the threshold is named in the payload so two heads cannot pick two of them");
        assert_eq!(old["expirationMs"], json!(120_000), "two minutes, as the C++ manager kept them");
    }

    #[test]
    fn an_alert_transition_outranks_the_throttle_and_a_cleared_alert_is_published_too() {
        let mut traffic = Traffic::default();
        assert!(traffic.receive(&located(1), 1_000));
        let alerting = Report { alert: Some(true), ..located(1) };
        assert!(traffic.receive(&alerting, 1_100), "a collision warning must not wait out the throttle");
        assert_eq!(traffic.contact(1).unwrap().report.alert, Some(true));
        assert_eq!(traffic.snapshot(1_100)["alerting"], json!(true));
        assert!(!traffic.receive(&alerting, 1_200), "the same alert is not a transition");
        let clear = Report { alert: Some(false), ..located(1) };
        assert!(traffic.receive(&clear, 1_300), "and the alert clearing is a transition as well, or the warning latches on forever");
        assert_eq!(traffic.contact(1).unwrap().report.alert, Some(false));
        assert_eq!(traffic.snapshot(1_300)["alerting"], json!(false));
        assert!(!traffic.receive(&located(1), 1_400), "a report that says nothing about the alert leaves the last stated one alone");
        assert_eq!(traffic.contact(1).unwrap().report.alert, Some(false));
    }

    #[test]
    fn a_feed_that_states_no_alert_at_all_is_unknown_rather_than_all_clear() {
        let mut traffic = Traffic::default();
        assert!(traffic.on_message(&vehicle(everything(), 1), 0), "a MAVLink feed carries no alert bit whatsoever");
        let view = traffic.snapshot(0);
        assert_eq!(
            view["alerting"],
            Value::Null,
            "a threat banner bound to this must not read a green all-clear off a feed that never grants one; false is reserved for a feed that stated an alert and cleared it"
        );
        assert_eq!(view["alertUnknown"], json!(1), "and the head is told how many contacts it knows nothing about");
        assert!(traffic.receive(&Report { alert: Some(false), ..located(2) }, 2_000));
        let mixed = traffic.snapshot(2_000);
        assert_eq!((mixed["alerting"].clone(), mixed["alertUnknown"].clone()), (json!(false), json!(1)), "one contact stating all-clear is an answer for itself, not for the one still unknown");
        assert!(traffic.receive(&Report { alert: Some(true), ..located(3) }, 4_000));
        assert_eq!(traffic.snapshot(4_000)["alerting"], json!(true), "and any stated alert outranks the rest");
    }

    #[test]
    fn a_target_that_reports_no_heading_is_pointed_along_its_track() {
        let mut traffic = Traffic::default();
        let start = (47.5, 8.5);
        assert!(traffic.receive(&Report { icao_address: 1, latitude: Some(start.0), longitude: Some(start.1), ..Report::default() }, 0));
        assert_eq!(traffic.contact(1).unwrap().report.heading_degrees, None, "one fix is no direction of flight");
        let (north, east, _) = crate::geo::ned_to_geo(500.0, 0.0, 0.0, (start.0, start.1, 0.0));
        assert!(traffic.receive(&Report { icao_address: 1, latitude: Some(north), longitude: Some(east), ..Report::default() }, 2_000));
        assert!(
            close(traffic.contact(1).unwrap().report.heading_degrees, 0.0),
            "an SBS-1 position feed sends no track, and the marker paints due north for every such target unless the core derives one; got {:?}",
            traffic.contact(1).unwrap().report.heading_degrees
        );
        let (_, drifted, _) = crate::geo::ned_to_geo(0.0, 0.5, 0.0, (north, east, 0.0));
        assert!(traffic.receive(&Report { icao_address: 1, latitude: Some(north), longitude: Some(drifted), ..Report::default() }, 4_000));
        assert!(close(traffic.contact(1).unwrap().report.heading_degrees, 0.0), "half a metre is receiver noise, not a turn to the east");
        let stated = Report { icao_address: 1, latitude: Some(start.0), longitude: Some(start.1), heading_degrees: Some(123.0), ..Report::default() };
        assert!(traffic.receive(&stated, 6_000));
        assert_eq!(traffic.contact(1).unwrap().report.heading_degrees, Some(123.0), "a heading the sender states outranks one the core inferred");
    }

    #[test]
    fn a_later_report_fills_in_what_it_knows_and_leaves_the_rest_standing() {
        let mut traffic = Traffic::default();
        assert!(traffic.on_message(&vehicle(everything(), 1), 0));
        assert!(traffic.on_sbs1_line(&sbs1(IDENTIFICATION_AND_CATEGORY, &[(FIELD_ICAO, "ABCDEF"), (FIELD_CALLSIGN, "SWR999")]), 2_000));
        let contact = traffic.contact(0xABCDEF).unwrap();
        assert_eq!(contact.report.callsign.as_deref(), Some("SWR999"), "the newest value for a reading wins");
        assert_eq!(contact.report.squawk, Some(7700), "a reading the new report is silent about keeps its last known value");
        assert_eq!(contact.report.seconds_since_last_seen, None, "except the age of the last transponder hit, which an SBS-1 line cannot know - carrying the old tslc forward would report a contact as seconds old minutes after it went quiet");
        assert_eq!(traffic.snapshot(2_000)["contacts"][0]["ageMs"], json!(0), "ageMs is the one age the core stands behind");
    }

    #[test]
    fn nearby_is_measured_from_the_operators_own_aircraft_and_the_nearest_is_first() {
        let mut traffic = Traffic::default();
        let own_position = (47.5, 8.5);
        let far = crate::geo::ned_to_geo(10_000.0, 0.0, 0.0, (own_position.0, own_position.1, 0.0));
        let near = crate::geo::ned_to_geo(0.0, 1_000.0, 0.0, (own_position.0, own_position.1, 0.0));
        assert!(traffic.receive(&Report { icao_address: 0x000001, latitude: Some(far.0), longitude: Some(far.1), altitude_metres: Some(3_000.0), ..Report::default() }, 0));
        assert!(traffic.receive(&Report { icao_address: 0xFFFFFF, latitude: Some(near.0), longitude: Some(near.1), altitude_metres: Some(1_200.0), ..Report::default() }, 0));

        let blind = traffic.snapshot(0);
        assert_eq!(blind["ownPositionKnown"], json!(false));
        assert_eq!((blind["contacts"][0]["distanceMetres"].clone(), blind["contacts"][0]["bearingDegrees"].clone()), (Value::Null, Value::Null), "with no own position a range is unknown, never a range of zero");

        traffic.observe(Some(Own { latitude: own_position.0, longitude: own_position.1, altitude_metres: Some(1_000.0) }));
        let view = traffic.snapshot(0);
        assert_eq!(view["ownPositionKnown"], json!(true));
        assert_eq!(view["contacts"][0]["icaoAddress"], json!(0xFFFFFF), "the closest aircraft is first whatever its ICAO address sorts as, or the map and the traffic list disagree about which target is the threat");
        assert!(close(view["contacts"][0]["distanceMetres"].as_f64(), 1_000.0), "got {:?}", view["contacts"][0]["distanceMetres"]);
        assert!(close(view["contacts"][0]["bearingDegrees"].as_f64(), 90.0), "got {:?}", view["contacts"][0]["bearingDegrees"]);
        assert_eq!(view["contacts"][0]["relativeAltitudeMetres"], json!(200.0), "at my level is the other half of the question, and the sign says above");
        assert_eq!(view["contacts"][1]["icaoAddress"], json!(0x000001));
        assert_eq!(view["order"], "nearestFirst");

        traffic.observe(Some(Own { latitude: own_position.0, longitude: own_position.1, altitude_metres: None }));
        assert_eq!(traffic.snapshot(0)["contacts"][0]["relativeAltitudeMetres"], Value::Null, "without an altitude of my own the difference is unknown rather than the other aircraft's own altitude");
    }

    #[test]
    fn the_snapshot_hands_over_numbers_and_tokens_with_the_units_named_apart() {
        let mut traffic = Traffic::default();
        assert!(traffic.on_message(&vehicle(everything(), 4), 5_000));
        let view = traffic.snapshot(6_000);
        assert_eq!((view["class"].as_str(), view["count"].as_u64()), (Some("AdsbTraffic"), Some(1)));
        assert_eq!(view["units"], json!({ "altitude": "m", "velocity": "m/s", "verticalVelocity": "m/s", "heading": "deg", "distance": "m", "bearing": "deg" }), "the head formats the number, so the core names the unit and never writes the text");
        let contact = &view["contacts"][0];
        assert_eq!(contact["icaoAddress"], json!(0xABCDEF));
        assert_eq!((contact["altitudeMetres"].as_f64(), contact["velocityMetresPerSecond"].as_f64(), contact["headingDegrees"].as_f64()), (Some(1500.0), Some(120.0), Some(270.0)));
        assert_eq!(contact["ageMs"], json!(1_000));
        assert_eq!(contact["alert"], Value::Null, "unknown is null, so a head cannot read it as a cleared warning");
        assert_eq!((contact["altitudeType"].as_str(), contact["emitterType"].as_str(), contact["callsign"].as_str()), (Some(ALTITUDE_GEOMETRIC), Some("light"), Some("SWR123")));
        assert!(view["contacts"].as_array().unwrap().iter().all(|c| c.as_object().unwrap().values().all(|v| !v.is_string() || !v.as_str().unwrap().contains(' '))), "no field carries a formatted sentence");
        let empty = Traffic::default().snapshot(0);
        assert_eq!(empty["count"].as_u64(), Some(0));
        assert!(empty["contacts"].as_array().unwrap().is_empty());
    }

    #[test]
    fn an_empty_list_says_which_of_the_four_reasons_it_is_empty() {
        let mut traffic = Traffic::default();
        let off = traffic.snapshot(0);
        assert_eq!(
            (off["enabled"].clone(), off["available"].clone(), off["connected"].clone(), off["receiving"].clone(), off["count"].clone()),
            (json!(false), json!(false), json!(false), json!(false), json!(0)),
            "nothing configured and nothing heard: an operator reading count alone would call this a clear sky"
        );
        assert_eq!(off["source"], Value::Null);

        assert!(traffic.retarget(true, Some(Source { host: "adsb.local".into(), port: 30003 })));
        let waiting = traffic.snapshot(0);
        assert_eq!((waiting["enabled"].clone(), waiting["available"].clone(), waiting["connected"].clone()), (json!(true), json!(true), json!(false)), "switched on and configured, but the socket is not up yet");
        assert_eq!(waiting["source"], json!({ "host": "adsb.local", "port": 30003 }), "the operator is told which feed is being drawn from");
        assert!(!traffic.retarget(true, Some(Source { host: "adsb.local".into(), port: 30003 })), "the same setting does not restart the link");

        let generation = traffic.generation();
        assert!(traffic.failed(generation, CONNECT_FAILED, "connection refused".into()));
        assert_eq!(traffic.snapshot(0)["error"], json!({ "token": CONNECT_FAILED, "detail": "connection refused" }), "the token is what a head keys on and the detail is the untranslated reason, as the C++ link put in its app message");
        assert!(!traffic.failed(generation - 1, CONNECT_FAILED, "stale".into()), "a failure from a superseded link is dropped");

        assert!(traffic.attached(generation));
        let up = traffic.snapshot(0);
        assert_eq!((up["connected"].clone(), up["receiving"].clone(), up["error"].clone()), (json!(true), json!(true), Value::Null), "connected with an empty list is the one case that really is a clear sky");

        let mut mavlink_only = Traffic::default();
        assert!(mavlink_only.on_message(&vehicle(everything(), 1), 1_000));
        let heard = mavlink_only.snapshot(1_000);
        assert_eq!((heard["receiving"].clone(), heard["mavlinkAgeMs"].clone()), (json!(true), json!(0)), "a vehicle relaying ADSB is a live source with the server switched off");
        assert_eq!(mavlink_only.snapshot(1_000 + EXPIRATION_MS + 1)["receiving"], json!(false), "and once it has said nothing for longer than a contact lives, the list is unknown again rather than clear");
    }

    #[test]
    fn switching_the_server_off_does_not_blank_traffic_the_vehicle_is_still_relaying() {
        let mut traffic = Traffic::default();
        assert!(traffic.retarget(true, Some(Source { host: "adsb.local".into(), port: 30003 })));
        assert!(traffic.on_message(&vehicle(everything(), 1), 0));
        assert!(traffic.retarget(false, None));
        assert_eq!(traffic.count(), 1, "the C++ manager cleared every contact when the TCP link stopped, including the ones its own receiver sent - dropping live traffic because a setting was toggled is the blank this whole view exists to prevent");
        assert_eq!(traffic.snapshot(0)["connected"], json!(false), "the link state goes with the link");
    }

    #[test]
    fn a_head_is_told_when_something_moved_instead_of_diffing_the_whole_list() {
        let mut traffic = Traffic::default();
        assert!(!traffic.tick(0), "nothing has happened yet");
        assert!(traffic.receive(&located(1), 1_000));
        assert_eq!(traffic.snapshot(1_000)["updatedMs"], json!(1_000));
        assert_eq!(traffic.snapshot(1_000)["publishIntervalMs"], json!(PUBLISH_INTERVAL_MS), "the redraw cadence is decided in the core, so the head stops inventing a timer");
        assert!(traffic.tick(1_000), "the contact that arrived is worth announcing");
        assert!(!traffic.tick(1_000), "and only once");
        let revision = traffic.snapshot(1_000)["revision"].as_u64().unwrap();
        assert!(!traffic.receive(&Report { latitude: Some(47.6), ..located(1) }, 1_500), "throttled");
        assert_eq!(traffic.snapshot(1_500)["revision"].as_u64(), Some(revision), "a throttled merge is not a redraw");
        assert!(traffic.tick(1_500 + STALE_MS + 1), "a contact going stale changes what is drawn, so it is announced too");
        assert!(!traffic.tick(1_500 + STALE_MS + 2));
        assert!(traffic.tick(1_500 + EXPIRATION_MS + 1), "and so does one falling off the list");
        assert_eq!(traffic.count(), 0);
    }

    struct Settings {
        enabled: Value,
        host: Value,
        port: Value,
        coordinate: Value,
    }

    impl Backend for Settings {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.adsbVehicleManagerSettings.adsbServerConnectEnabled.rawValue" => json!({ "kind": "value", "value": self.enabled }).to_string(),
                "settings.adsbVehicleManagerSettings.adsbServerHostAddress.rawValue" => json!({ "kind": "value", "value": self.host }).to_string(),
                "settings.adsbVehicleManagerSettings.adsbServerPort.rawValue" => json!({ "kind": "value", "value": self.port }).to_string(),
                "vehicle.coordinate" => self.coordinate.to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            self.get(path)
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
    fn the_view_reads_the_server_the_operator_configured_and_the_position_it_is_measured_from() {
        let settings = Settings {
            enabled: json!(false),
            host: json!("adsb.example"),
            port: json!("30003"),
            coordinate: json!({ "kind": "coordinate", "valid": true, "latitude": 47.4, "longitude": 8.4, "altitude": 500.0 }),
        };
        let view = adsb_traffic_view(&settings, &[]);
        assert_eq!((view["enabled"].clone(), view["available"].clone()), (json!(false), json!(true)), "a host and port stand configured whether or not the operator has switched the feed on");
        assert_eq!(view["source"], json!({ "host": "adsb.example", "port": 30003 }), "the port fact is stored as text and still names a port");
        assert_eq!(view["ownPositionKnown"], json!(true));

        assert_eq!(DEPS.len(), 4, "every fact the view reads is watched, or it is never recomputed when the operator changes it");
        assert!(DEPS.iter().all(|dep| dep.starts_with("settings.adsbVehicleManagerSettings.") || *dep == "vehicle.coordinate"));

        let nowhere = adsb_traffic_view(&Settings { enabled: json!(true), host: json!("   "), port: json!(30003), coordinate: json!({ "kind": "null" }) }, &[]);
        assert_eq!((nowhere["enabled"].clone(), nowhere["available"].clone()), (json!(true), json!(false)), "switched on with no host is enabled and unusable, which is a different thing to say");
        assert_eq!(nowhere["ownPositionKnown"], json!(false));
        assert_eq!(port_number(&json!({ "value": "0" }).to_string()), None, "port zero is no port");
        assert_eq!(port_number(&json!({ "value": "70000" }).to_string()), None);
        assert_eq!(port_number(&json!({ "value": 30003 }).to_string()), Some(30003), "and a fact stored as a number still names one");
    }

    #[test]
    fn own_reads_the_coordinate_the_vehicle_publishes_and_refuses_the_one_it_does_not_have() {
        assert_eq!(own(&json!({ "kind": "coordinate", "valid": true, "latitude": 47.5, "longitude": 8.5, "altitude": 120.0 }).to_string()), Some(Own { latitude: 47.5, longitude: 8.5, altitude_metres: Some(120.0) }));
        assert_eq!(own(&json!({ "kind": "coordinate", "valid": false, "latitude": 47.5, "longitude": 8.5 }).to_string()), None);
        assert_eq!(own(&json!({ "kind": "coordinate", "valid": true, "latitude": 0.0, "longitude": 0.0 }).to_string()), None, "null island is what an autopilot publishes before it has a fix");
        assert_eq!(own(&json!({ "kind": "null" }).to_string()), None);
    }
}
