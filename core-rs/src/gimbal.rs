use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::mavout::{GCS_COMPONENT, GCS_SYSTEM};

pub const MSG_GIMBAL_MANAGER_INFORMATION: u32 = 280;
pub const MSG_GIMBAL_MANAGER_STATUS: u32 = 281;
pub const MSG_GIMBAL_DEVICE_ATTITUDE_STATUS: u32 = 285;
pub const CMD_DO_GIMBAL_MANAGER_PITCHYAW: u16 = 1000;
pub const CMD_DO_GIMBAL_MANAGER_CONFIGURE: u16 = 1001;

pub const CAP_HAS_RETRACT: u32 = 1;
pub const CAP_HAS_YAW_LOCK: u32 = 1024;

pub const FLAG_RETRACT: u32 = 1;
pub const FLAG_NEUTRAL: u32 = 2;
pub const FLAG_ROLL_LOCK: u32 = 4;
pub const FLAG_PITCH_LOCK: u32 = 8;
pub const FLAG_YAW_LOCK: u32 = 16;
pub const FLAG_YAW_IN_VEHICLE_FRAME: u32 = 32;
pub const FLAG_YAW_IN_EARTH_FRAME: u32 = 64;

pub const ERROR_AT_ROLL_LIMIT: u32 = 1;
pub const ERROR_AT_PITCH_LIMIT: u32 = 2;
pub const ERROR_AT_YAW_LIMIT: u32 = 4;
pub const ERROR_ENCODER: u32 = 8;
pub const ERROR_POWER: u32 = 16;
pub const ERROR_MOTOR: u32 = 32;
pub const ERROR_SOFTWARE: u32 = 64;
pub const ERROR_COMMS: u32 = 128;
pub const ERROR_CALIBRATION_RUNNING: u32 = 256;
pub const ERROR_NO_MANAGER: u32 = 512;

pub const MANAGER_INFORMATION_RETRIES: u32 = 6;
pub const INFORMATION_RETRIES: u32 = 3;
pub const STATUS_RETRIES: u32 = 6;
pub const ATTITUDE_RETRIES: u32 = 3;
pub const STATUS_REQUEST_GAP_MS: u64 = 1000;
pub const SLOW_STATUS_INTERVAL_US: f64 = 5_000_000.0;
pub const DEFAULT_INTERVAL_US: f64 = 0.0;
pub const INFORMATION_REQUEST_TIMEOUT_MS: u64 = 3000;
pub const CONTROL_STALE_MS: u64 = 15_000;
pub const ATTITUDE_STALE_MS: u64 = 3000;
pub const HEADING_STALE_MS: u64 = 3000;
pub const NON_MAVLINK_DEVICE_IDS: u8 = 6;
pub const RELEASE_CONTROL: f64 = -3.0;
pub const LEAVE_UNCHANGED: f64 = -1.0;
pub const ANGLE_UNITS: &str = "degree";
pub const RATE_UNITS: &str = "degreePerSecond";

pub const REASON_NOT_READY: &str = "notReady";
pub const REASON_NO_ACTIVE_GIMBAL: &str = "noActiveGimbal";
pub const REASON_OTHERS_HAVE_CONTROL: &str = "othersHaveControl";
pub const REASON_ATTITUDE_UNKNOWN: &str = "attitudeUnknown";
pub const REASON_HEADING_UNKNOWN: &str = "headingUnknownWhileYawLocked";
pub const REASON_UNSUPPORTED: &str = "unsupported";

pub const DISCOVERY_IDLE: &str = "idle";
pub const DISCOVERY_SEARCHING: &str = "searching";
pub const DISCOVERY_FOUND: &str = "found";
pub const DISCOVERY_NONE: &str = "none";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PairId {
    pub manager_compid: u8,
    pub device_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ManagerInformation {
    pub compid: u8,
    pub device_id: u8,
    pub capability_flags: u32,
    pub limits_rad: Option<[f32; 6]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ManagerStatus {
    pub compid: u8,
    pub device_id: u8,
    pub primary_sysid: u8,
    pub primary_compid: u8,
    pub secondary_sysid: u8,
    pub secondary_compid: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DeviceAttitude {
    pub compid: u8,
    pub device_id: u8,
    pub flags: u32,
    pub q: [f32; 4],
    pub angular_velocity_rad_s: Option<[f32; 3]>,
    pub failure_flags: u32,
    pub delta_yaw_rad: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    Vehicle,
    Earth,
}

impl Frame {
    pub fn token(self) -> &'static str {
        match self {
            Frame::Vehicle => "vehicle",
            Frame::Earth => "earth",
        }
    }

    pub fn of(flags: u32) -> Frame {
        match (flags & FLAG_YAW_IN_VEHICLE_FRAME != 0, flags & FLAG_YAW_IN_EARTH_FRAME != 0, flags & FLAG_YAW_LOCK != 0) {
            (true, _, _) => Frame::Vehicle,
            (false, true, _) => Frame::Earth,
            (false, false, true) => Frame::Earth,
            (false, false, false) => Frame::Vehicle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attitude {
    pub roll: f32,
    pub pitch: f32,
    pub body_yaw: Option<f32>,
    pub absolute_yaw: Option<f32>,
    pub pitch_rate: Option<f32>,
    pub yaw_rate: Option<f32>,
    pub delta_yaw: Option<f32>,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Point { h_fov: f32, v_fov: f32 },
    Drag { slide_speed: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ownership {
    pub primary_sysid: u8,
    pub primary_compid: u8,
    pub secondary_sysid: u8,
    pub secondary_compid: u8,
    pub ours: bool,
    pub others: bool,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Out {
    RequestMessage { component: u8, message: u32 },
    MessageInterval { component: u8, message: u32, interval_us: f64 },
    Command { component: u8, command: u16, params: [f64; 7], show_error: bool },
    StartRateRepeat,
    StopRateRepeat,
    ActiveGimbal(PairId),
    GimbalComplete(PairId),
    AskToTakeControl(PairId),
    Refused { pair: Option<PairId>, reason: &'static str },
}

#[derive(Debug, PartialEq)]
pub struct Gimbal {
    pub attitude: Option<Attitude>,
    pub yaw_frame: Option<Frame>,
    pub commanded_pitch_rate: Option<f32>,
    pub commanded_yaw_rate: Option<f32>,
    pub yaw_lock: Option<bool>,
    pub retracted: Option<bool>,
    pub neutral: Option<bool>,
    pub capability_flags: Option<u32>,
    pub failure_flags: Option<u32>,
    pub limits: [Option<f32>; 6],
    pub complete: bool,
    ownership: Option<Ownership>,
    received_information: bool,
    received_status: bool,
    received_attitude: bool,
    information_retries: u32,
    status_retries: u32,
    attitude_retries: u32,
}

impl Default for Gimbal {
    fn default() -> Self {
        Gimbal {
            attitude: None,
            yaw_frame: None,
            commanded_pitch_rate: None,
            commanded_yaw_rate: None,
            yaw_lock: None,
            retracted: None,
            neutral: None,
            capability_flags: None,
            failure_flags: None,
            limits: [None; 6],
            complete: false,
            ownership: None,
            received_information: false,
            received_status: false,
            received_attitude: false,
            information_retries: INFORMATION_RETRIES,
            status_retries: STATUS_RETRIES,
            attitude_retries: ATTITUDE_RETRIES,
        }
    }
}

impl Gimbal {
    pub fn supports_retract(&self) -> Option<bool> {
        self.capability_flags.map(|flags| flags & CAP_HAS_RETRACT != 0)
    }

    pub fn supports_yaw_lock(&self) -> Option<bool> {
        self.capability_flags.map(|flags| flags & CAP_HAS_YAW_LOCK != 0)
    }

    pub fn have_control(&self, now_ms: u64) -> Option<bool> {
        self.fresh_ownership(now_ms).map(|owner| owner.ours)
    }

    pub fn others_have_control(&self) -> Option<bool> {
        self.ownership.map(|owner| owner.others)
    }

    pub fn attitude_age_ms(&self, now_ms: u64) -> Option<u64> {
        self.attitude.map(|a| now_ms.saturating_sub(a.at_ms))
    }

    pub fn attitude_stale(&self, now_ms: u64) -> bool {
        self.attitude_age_ms(now_ms).is_none_or(|age| age > ATTITUDE_STALE_MS)
    }

    fn fresh_ownership(&self, now_ms: u64) -> Option<Ownership> {
        self.ownership.filter(|owner| now_ms.saturating_sub(owner.at_ms) <= CONTROL_STALE_MS)
    }

    fn heard_everything(&self) -> bool {
        self.received_information && self.received_status && self.received_attitude
    }

    fn chasing(&self) -> bool {
        (!self.received_information && self.information_retries > 0) || (!self.received_status && self.status_retries > 0) || (!self.received_attitude && self.attitude_retries > 0)
    }

    fn missing(&self) -> Vec<&'static str> {
        [(self.received_information, "information"), (self.received_status, "status"), (self.received_attitude, "attitude")]
            .iter()
            .filter(|(heard, _)| !heard)
            .map(|(_, name)| *name)
            .collect()
    }

    fn failures(&self) -> Value {
        self.failure_flags
            .map(|flags| {
                let on = |bit: u32| flags & bit != 0;
                json!({
                    "atRollLimit": on(ERROR_AT_ROLL_LIMIT),
                    "atPitchLimit": on(ERROR_AT_PITCH_LIMIT),
                    "atYawLimit": on(ERROR_AT_YAW_LIMIT),
                    "encoderError": on(ERROR_ENCODER),
                    "powerError": on(ERROR_POWER),
                    "motorError": on(ERROR_MOTOR),
                    "softwareError": on(ERROR_SOFTWARE),
                    "commsError": on(ERROR_COMMS),
                    "calibrationRunning": on(ERROR_CALIBRATION_RUNNING),
                    "noManager": on(ERROR_NO_MANAGER),
                })
            })
            .unwrap_or(Value::Null)
    }

    fn snapshot(&self, pair: PairId, active: bool, control: Value, aim: Value, now_ms: u64) -> Value {
        let age_ms = self.attitude_age_ms(now_ms);
        let stale = self.attitude_stale(now_ms);
        let fresh = |mode: Option<bool>| mode.filter(|_| !stale);
        json!({
            "managerCompid": pair.manager_compid,
            "deviceId": pair.device_id,
            "active": active,
            "roll": self.attitude.map(|a| a.roll),
            "pitch": self.attitude.map(|a| a.pitch),
            "bodyYaw": self.attitude.and_then(|a| a.body_yaw),
            "absoluteYaw": self.attitude.and_then(|a| a.absolute_yaw),
            "attitudeAgeMs": age_ms,
            "attitudeStale": stale,
            "yawFrame": self.yaw_frame.map(Frame::token),
            "measuredPitchRate": self.attitude.and_then(|a| a.pitch_rate),
            "measuredYawRate": self.attitude.and_then(|a| a.yaw_rate),
            "commandedPitchRate": self.commanded_pitch_rate,
            "commandedYawRate": self.commanded_yaw_rate,
            "yawLock": fresh(self.yaw_lock),
            "retracted": fresh(self.retracted),
            "neutral": fresh(self.neutral),
            "haveControl": self.have_control(now_ms),
            "othersHaveControl": self.others_have_control(),
            "ownershipAgeMs": self.ownership.map(|owner| now_ms.saturating_sub(owner.at_ms)),
            "primarySysid": self.ownership.map(|owner| owner.primary_sysid),
            "primaryCompid": self.ownership.map(|owner| owner.primary_compid),
            "secondarySysid": self.ownership.map(|owner| owner.secondary_sysid),
            "secondaryCompid": self.ownership.map(|owner| owner.secondary_compid),
            "capabilityFlags": self.capability_flags,
            "supportsRetract": self.supports_retract(),
            "supportsYawLock": self.supports_yaw_lock(),
            "failureFlags": self.failure_flags,
            "failures": self.failures(),
            "limits": json!({
                "rollMin": self.limits[0],
                "rollMax": self.limits[1],
                "pitchMin": self.limits[2],
                "pitchMax": self.limits[3],
                "yawMin": self.limits[4],
                "yawMax": self.limits[5],
            }),
            "control": control,
            "aim": aim,
        })
    }
}

#[derive(Debug)]
struct Manager {
    retries: u32,
    received_information: bool,
}

impl Default for Manager {
    fn default() -> Self {
        Manager { retries: MANAGER_INFORMATION_RETRIES, received_information: false }
    }
}

#[derive(Debug)]
pub struct Gimbals {
    pub our_system: u8,
    pub our_component: u8,
    pub ready: bool,
    gimbals: BTreeMap<PairId, Gimbal>,
    managers: BTreeMap<u8, Manager>,
    active: Option<PairId>,
    heading: Option<(f32, u64)>,
    pending_information: Option<(u8, u64)>,
    last_status_request_ms: Option<u64>,
}

impl Default for Gimbals {
    fn default() -> Self {
        Gimbals {
            our_system: GCS_SYSTEM,
            our_component: GCS_COMPONENT,
            ready: false,
            gimbals: BTreeMap::new(),
            managers: BTreeMap::new(),
            active: None,
            heading: None,
            pending_information: None,
            last_status_request_ms: None,
        }
    }
}

pub fn wrap180(angle: f32) -> f32 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

pub fn euler_degrees(q: [f32; 4]) -> Option<(f32, f32, f32)> {
    let norm = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    (q.iter().all(|v| v.is_finite()) && norm > 0.1).then(|| {
        let [w, x, y, z] = q.map(|v| v / norm);
        (
            (2.0 * (w * x + y * z)).atan2(1.0 - 2.0 * (x * x + y * y)).to_degrees(),
            (2.0 * (w * y - z * x)).clamp(-1.0, 1.0).asin().to_degrees(),
            (2.0 * (w * z + x * y)).atan2(1.0 - 2.0 * (y * y + z * z)).to_degrees(),
        )
    })
}

fn offer(reason: Option<&'static str>) -> Value {
    json!({
        "offer": match reason { Some(_) => "blocked", None => "ready" },
        "reason": reason.unwrap_or(""),
    })
}

impl Gimbals {
    pub fn new(our_system: u8, our_component: u8) -> Gimbals {
        Gimbals { our_system, our_component, ..Default::default() }
    }

    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }

    pub fn set_heading(&mut self, heading: Option<f32>, now_ms: u64) {
        self.heading = heading.filter(|h| h.is_finite()).map(|h| (h, now_ms));
    }

    pub fn heading_at(&self, now_ms: u64) -> Option<f32> {
        self.heading.filter(|(_, at)| now_ms.saturating_sub(*at) <= HEADING_STALE_MS).map(|(h, _)| h)
    }

    pub fn heading_age_ms(&self, now_ms: u64) -> Option<u64> {
        self.heading.map(|(_, at)| now_ms.saturating_sub(at))
    }

    pub fn get(&self, pair: PairId) -> Option<&Gimbal> {
        self.gimbals.get(&pair)
    }

    pub fn active(&self) -> Option<PairId> {
        self.active
    }

    pub fn pairs(&self) -> Vec<PairId> {
        self.gimbals.keys().copied().collect()
    }

    pub fn set_active(&mut self, pair: PairId) -> Vec<Out> {
        match self.gimbals.get(&pair).is_some_and(|gimbal| gimbal.complete) && self.active != Some(pair) {
            true => {
                self.active = Some(pair);
                vec![Out::ActiveGimbal(pair)]
            }
            false => Vec::new(),
        }
    }

    pub fn on_heartbeat(&mut self, compid: u8, now_ms: u64) -> Vec<Out> {
        if !self.ready {
            return Vec::new();
        }
        let wanted = {
            let manager = self.managers.entry(compid).or_default();
            !manager.received_information && manager.retries > 0
        };
        if !wanted {
            return Vec::new();
        }
        let out = self.request_information(compid, now_ms);
        self.managers.entry(compid).and_modify(|m| m.retries = m.retries.saturating_sub(out.len() as u32));
        out
    }

    pub fn on_manager_information(&mut self, info: ManagerInformation, now_ms: u64) -> Vec<Out> {
        if !self.ready || info.device_id == 0 {
            return Vec::new();
        }
        let pair = PairId { manager_compid: info.compid, device_id: info.device_id };
        let limits = info.limits_rad.map(|rad| rad.map(|v| Some(v.to_degrees()).filter(|d| d.is_finite())));
        let gimbal = self.gimbals.entry(pair).or_default();
        gimbal.capability_flags = Some(info.capability_flags);
        gimbal.limits = limits.unwrap_or(gimbal.limits);
        gimbal.received_information = true;
        self.managers.entry(info.compid).or_default().received_information = true;
        self.pending_information = self.pending_information.filter(|(pending, _)| *pending != info.compid);
        self.check_complete(pair, now_ms)
    }

    pub fn on_manager_status(&mut self, status: ManagerStatus, now_ms: u64) -> Vec<Out> {
        if !self.ready || status.device_id == 0 {
            return Vec::new();
        }
        let pair = PairId { manager_compid: status.compid, device_id: status.device_id };
        let ours = (status.primary_sysid, status.primary_compid) == (self.our_system, self.our_component);
        let unowned = status.primary_sysid == 0 || status.primary_compid == 0;
        let gimbal = self.gimbals.entry(pair).or_default();
        gimbal.received_status = true;
        gimbal.ownership = Some(Ownership {
            primary_sysid: status.primary_sysid,
            primary_compid: status.primary_compid,
            secondary_sysid: status.secondary_sysid,
            secondary_compid: status.secondary_compid,
            ours,
            others: !ours && !unowned,
            at_ms: now_ms,
        });
        self.check_complete(pair, now_ms)
    }

    pub fn on_device_attitude_status(&mut self, report: DeviceAttitude, now_ms: u64) -> Vec<Out> {
        if !self.ready {
            return Vec::new();
        }
        let Some(pair) = self.attitude_pair(report.compid, report.device_id) else { return Vec::new() };
        let frame = Frame::of(report.flags);
        let delta_yaw = report.delta_yaw_rad.map(f32::to_degrees).filter(|d| d.is_finite());
        let offset = delta_yaw.or(self.heading_at(now_ms));
        let rate = |axis: usize| report.angular_velocity_rad_s.map(|w| w[axis].to_degrees()).filter(|r| r.is_finite());
        let attitude = euler_degrees(report.q).map(|(roll, pitch, yaw)| {
            let (body_yaw, absolute_yaw) = match frame {
                Frame::Vehicle => (Some(wrap180(yaw)), offset.map(|d| wrap180(yaw + d))),
                Frame::Earth => (offset.map(|d| wrap180(yaw - d)), Some(wrap180(yaw))),
            };
            Attitude { roll, pitch, body_yaw, absolute_yaw, pitch_rate: rate(1), yaw_rate: rate(2), delta_yaw, at_ms: now_ms }
        });
        let gimbal = self.gimbals.entry(pair).or_default();
        gimbal.retracted = Some(report.flags & FLAG_RETRACT != 0);
        gimbal.yaw_lock = Some(report.flags & FLAG_YAW_LOCK != 0);
        gimbal.neutral = Some(report.flags & FLAG_NEUTRAL != 0);
        gimbal.failure_flags = Some(report.failure_flags);
        gimbal.yaw_frame = Some(frame);
        gimbal.attitude = attitude.or(gimbal.attitude);
        gimbal.received_attitude = true;
        self.check_complete(pair, now_ms)
    }

    pub fn acquire_control(&self) -> Vec<Out> {
        let Some(pair) = self.active else { return vec![Out::Refused { pair: None, reason: REASON_NO_ACTIVE_GIMBAL }] };
        vec![Out::Command {
            component: pair.manager_compid,
            command: CMD_DO_GIMBAL_MANAGER_CONFIGURE,
            params: [self.our_system as f64, self.our_component as f64, LEAVE_UNCHANGED, LEAVE_UNCHANGED, f64::NAN, f64::NAN, pair.device_id as f64],
            show_error: true,
        }]
    }

    pub fn release_control(&self) -> Vec<Out> {
        let Some(pair) = self.active else { return vec![Out::Refused { pair: None, reason: REASON_NO_ACTIVE_GIMBAL }] };
        vec![Out::Command {
            component: pair.manager_compid,
            command: CMD_DO_GIMBAL_MANAGER_CONFIGURE,
            params: [RELEASE_CONTROL, RELEASE_CONTROL, LEAVE_UNCHANGED, LEAVE_UNCHANGED, f64::NAN, f64::NAN, pair.device_id as f64],
            show_error: true,
        }]
    }

    pub fn send_pitch_body_yaw(&mut self, pitch: f32, yaw: f32, show_error: bool, now_ms: u64) -> Vec<Out> {
        self.send_angles(FLAG_ROLL_LOCK | FLAG_PITCH_LOCK | FLAG_YAW_IN_VEHICLE_FRAME, pitch, wrap180(yaw), show_error, now_ms)
    }

    pub fn send_pitch_absolute_yaw(&mut self, pitch: f32, yaw: f32, show_error: bool, now_ms: u64) -> Vec<Out> {
        self.send_angles(FLAG_ROLL_LOCK | FLAG_PITCH_LOCK | FLAG_YAW_LOCK | FLAG_YAW_IN_EARTH_FRAME, pitch, wrap180(yaw), show_error, now_ms)
    }

    pub fn center(&mut self, now_ms: u64) -> Vec<Out> {
        self.send_pitch_body_yaw(0.0, 0.0, true, now_ms)
    }

    pub fn set_retract(&mut self, retract: bool, now_ms: u64) -> Vec<Out> {
        let flags = match retract {
            true => FLAG_RETRACT,
            false => 0,
        };
        match self.supported(Gimbal::supports_retract) {
            false => vec![Out::Refused { pair: self.active, reason: REASON_UNSUPPORTED }],
            true => self.hold_attitude(flags, now_ms),
        }
    }

    pub fn set_yaw_lock(&mut self, lock: bool, now_ms: u64) -> Vec<Out> {
        let locked = match lock {
            true => FLAG_YAW_LOCK,
            false => 0,
        };
        match self.supported(Gimbal::supports_yaw_lock) {
            false => vec![Out::Refused { pair: self.active, reason: REASON_UNSUPPORTED }],
            true => self.hold_attitude(FLAG_ROLL_LOCK | FLAG_PITCH_LOCK | locked, now_ms),
        }
    }

    pub fn set_rates(&mut self, pitch_rate: Option<f32>, yaw_rate: Option<f32>, now_ms: u64) -> Vec<Out> {
        let stored = self.active.and_then(|pair| self.gimbals.get_mut(&pair)).map(|gimbal| {
            gimbal.commanded_pitch_rate = pitch_rate.filter(|r| r.is_finite()).or(gimbal.commanded_pitch_rate);
            gimbal.commanded_yaw_rate = yaw_rate.filter(|r| r.is_finite()).or(gimbal.commanded_yaw_rate);
            (gimbal.commanded_pitch_rate.unwrap_or(0.0), gimbal.commanded_yaw_rate.unwrap_or(0.0), gimbal.yaw_lock.unwrap_or(false))
        });
        let Some((pitch, yaw, locked)) = stored else { return vec![Out::Refused { pair: self.active, reason: REASON_NO_ACTIVE_GIMBAL }] };
        let (target, control) = self.control(now_ms);
        let Some(pair) = target else { return control.into_iter().chain([Out::StopRateRepeat]).collect() };
        let lock = match locked {
            true => FLAG_YAW_LOCK,
            false => 0,
        };
        let repeat = match pitch != 0.0 || yaw != 0.0 {
            true => Out::StartRateRepeat,
            false => Out::StopRateRepeat,
        };
        control.into_iter().chain([self.rate_command(pair, FLAG_ROLL_LOCK | FLAG_PITCH_LOCK | lock, pitch, yaw), repeat]).collect()
    }

    pub fn on_screen_control(&mut self, pan_pct: f32, tilt_pct: f32, screen: Screen, now_ms: u64) -> Vec<Out> {
        let (pan_scale, tilt_scale) = match screen {
            Screen::Point { h_fov, v_fov } => (h_fov * 0.5, v_fov * 0.5),
            Screen::Drag { slide_speed } => (slide_speed * 0.1, slide_speed * 0.1),
        };
        let pointing = self
            .active
            .and_then(|pair| self.gimbals.get(&pair))
            .and_then(|gimbal| gimbal.attitude.map(|a| (a.body_yaw, a.pitch, gimbal.yaw_lock.unwrap_or(false), a.delta_yaw)));
        let Some((Some(body_yaw), pitch, locked, delta_yaw)) = pointing else {
            return vec![Out::Refused { pair: self.active, reason: REASON_ATTITUDE_UNKNOWN }];
        };
        let (pan, tilt) = (pan_pct * pan_scale + body_yaw, tilt_pct * tilt_scale + pitch);
        match (locked, delta_yaw.or(self.heading_at(now_ms))) {
            (true, Some(offset)) => self.send_pitch_absolute_yaw(tilt, pan + offset, false, now_ms),
            (true, None) => vec![Out::Refused { pair: self.active, reason: REASON_HEADING_UNKNOWN }],
            (false, _) => self.send_pitch_body_yaw(tilt, pan, false, now_ms),
        }
    }

    pub fn snapshot(&self, now_ms: u64) -> Value {
        let complete: Vec<(&PairId, &Gimbal)> = self.gimbals.iter().filter(|(_, gimbal)| gimbal.complete).collect();
        json!({
            "kind": "object",
            "class": "Gimbals",
            "ready": self.ready,
            "discovery": self.discovery(),
            "available": !complete.is_empty(),
            "count": complete.len(),
            "angleUnits": ANGLE_UNITS,
            "rateUnits": RATE_UNITS,
            "heading": self.heading_at(now_ms),
            "headingAgeMs": self.heading_age_ms(now_ms),
            "active": self.active.map(|pair| json!({ "managerCompid": pair.manager_compid, "deviceId": pair.device_id })),
            "pending": self.gimbals
                .iter()
                .filter(|(_, gimbal)| !gimbal.complete)
                .map(|(pair, gimbal)| json!({ "managerCompid": pair.manager_compid, "deviceId": pair.device_id, "missing": gimbal.missing() }))
                .collect::<Vec<_>>(),
            "gimbals": complete
                .iter()
                .map(|(pair, gimbal)| gimbal.snapshot(**pair, self.active == Some(**pair), offer(self.control_gate(**pair)), offer(self.aim_gate(**pair, now_ms)), now_ms))
                .collect::<Vec<_>>(),
        })
    }

    fn discovery(&self) -> &'static str {
        let searching = self.managers.is_empty()
            || self.managers.values().any(|manager| !manager.received_information && manager.retries > 0)
            || self.gimbals.values().any(|gimbal| !gimbal.complete && gimbal.chasing());
        match (self.ready, self.gimbals.values().any(|gimbal| gimbal.complete), searching) {
            (false, _, _) => DISCOVERY_IDLE,
            (true, true, _) => DISCOVERY_FOUND,
            (true, false, true) => DISCOVERY_SEARCHING,
            (true, false, false) => DISCOVERY_NONE,
        }
    }

    fn control_gate(&self, pair: PairId) -> Option<&'static str> {
        match (self.ready, self.gimbals.get(&pair)) {
            (false, _) => Some(REASON_NOT_READY),
            (true, None) => Some(REASON_NO_ACTIVE_GIMBAL),
            (true, Some(gimbal)) => gimbal.others_have_control().filter(|held| *held).map(|_| REASON_OTHERS_HAVE_CONTROL),
        }
    }

    fn aim_gate(&self, pair: PairId, now_ms: u64) -> Option<&'static str> {
        self.control_gate(pair).or_else(|| {
            let gimbal = self.gimbals.get(&pair)?;
            let pointing = gimbal.attitude.filter(|a| a.body_yaw.is_some());
            match (pointing, gimbal.yaw_lock.unwrap_or(false), pointing.and_then(|a| a.delta_yaw).or(self.heading_at(now_ms))) {
                (None, _, _) => Some(REASON_ATTITUDE_UNKNOWN),
                (_, true, None) => Some(REASON_HEADING_UNKNOWN),
                _ => None,
            }
        })
    }

    fn supported(&self, has: fn(&Gimbal) -> Option<bool>) -> bool {
        self.active.and_then(|pair| self.gimbals.get(&pair)).and_then(has) != Some(false)
    }

    fn attitude_pair(&self, compid: u8, device_id: u8) -> Option<PairId> {
        match device_id {
            0 => self.gimbals.keys().find(|pair| pair.device_id == compid).copied(),
            id if id <= NON_MAVLINK_DEVICE_IDS => Some(PairId { manager_compid: compid, device_id: id }),
            _ => None,
        }
    }

    fn control(&mut self, now_ms: u64) -> (Option<PairId>, Vec<Out>) {
        if !self.ready {
            return (None, vec![Out::Refused { pair: self.active, reason: REASON_NOT_READY }]);
        }
        let Some(pair) = self.active else { return (None, vec![Out::Refused { pair: None, reason: REASON_NO_ACTIVE_GIMBAL }]) };
        match self.gimbals.get(&pair).map(|gimbal| (gimbal.others_have_control(), gimbal.have_control(now_ms))) {
            None => (None, vec![Out::Refused { pair: Some(pair), reason: REASON_NO_ACTIVE_GIMBAL }]),
            Some((Some(true), _)) => (None, vec![Out::AskToTakeControl(pair)]),
            Some((_, Some(true))) => (Some(pair), Vec::new()),
            Some(_) => (Some(pair), self.acquire_control()),
        }
    }

    fn send_angles(&mut self, flags: u32, pitch: f32, yaw: f32, show_error: bool, now_ms: u64) -> Vec<Out> {
        let stop = self.stop_rates();
        let (target, control) = self.control(now_ms);
        let Some(pair) = target else { return control.into_iter().chain(stop).collect() };
        control.into_iter().chain(stop).chain([self.pitch_yaw(pair, flags, Some(pitch), Some(yaw), show_error)]).collect()
    }

    fn stop_rates(&mut self) -> Vec<Out> {
        if let Some(gimbal) = self.active.and_then(|pair| self.gimbals.get_mut(&pair)) {
            gimbal.commanded_pitch_rate = Some(0.0);
            gimbal.commanded_yaw_rate = Some(0.0);
        }
        vec![Out::StopRateRepeat]
    }

    fn hold_attitude(&mut self, flags: u32, now_ms: u64) -> Vec<Out> {
        let (target, control) = self.control(now_ms);
        let Some(pair) = target else { return control };
        let attitude = self.gimbals.get(&pair).and_then(|gimbal| gimbal.attitude);
        let yaw = attitude.and_then(|a| match Frame::of(flags) {
            Frame::Vehicle => a.body_yaw,
            Frame::Earth => a.absolute_yaw,
        });
        control.into_iter().chain([self.pitch_yaw(pair, flags, attitude.map(|a| a.pitch), yaw, true)]).collect()
    }

    fn pitch_yaw(&self, pair: PairId, flags: u32, pitch: Option<f32>, yaw: Option<f32>, show_error: bool) -> Out {
        Out::Command {
            component: pair.manager_compid,
            command: CMD_DO_GIMBAL_MANAGER_PITCHYAW,
            params: [pitch.map(f64::from).unwrap_or(f64::NAN), yaw.map(f64::from).unwrap_or(f64::NAN), f64::NAN, f64::NAN, flags as f64, 0.0, pair.device_id as f64],
            show_error,
        }
    }

    fn rate_command(&self, pair: PairId, flags: u32, pitch_rate: f32, yaw_rate: f32) -> Out {
        Out::Command {
            component: pair.manager_compid,
            command: CMD_DO_GIMBAL_MANAGER_PITCHYAW,
            params: [f64::NAN, f64::NAN, pitch_rate as f64, yaw_rate as f64, flags as f64, 0.0, pair.device_id as f64],
            show_error: false,
        }
    }

    fn request_information(&mut self, compid: u8, now_ms: u64) -> Vec<Out> {
        self.pending_information = self.pending_information.filter(|(_, due)| now_ms < *due);
        match self.pending_information {
            Some(_) => Vec::new(),
            None => {
                self.pending_information = Some((compid, now_ms.saturating_add(INFORMATION_REQUEST_TIMEOUT_MS)));
                vec![Out::RequestMessage { component: compid, message: MSG_GIMBAL_MANAGER_INFORMATION }]
            }
        }
    }

    fn check_complete(&mut self, pair: PairId, now_ms: u64) -> Vec<Out> {
        if self.gimbals.get(&pair).is_none_or(|gimbal| gimbal.complete) {
            return Vec::new();
        }
        let information = self.chase_information(pair, now_ms);
        let status = self.chase_status(pair, now_ms);
        let attitude = self.chase_attitude(pair);
        let complete = match self.gimbals.get(&pair).is_some_and(Gimbal::heard_everything) {
            true => self.complete(pair),
            false => Vec::new(),
        };
        information.into_iter().chain(status).chain(attitude).chain(complete).collect()
    }

    fn chase_information(&mut self, pair: PairId, now_ms: u64) -> Vec<Out> {
        if self.gimbals.get(&pair).is_none_or(|gimbal| gimbal.received_information || gimbal.information_retries == 0) {
            return Vec::new();
        }
        let out = self.request_information(pair.manager_compid, now_ms);
        self.gimbals.entry(pair).and_modify(|gimbal| gimbal.information_retries = gimbal.information_retries.saturating_sub(out.len() as u32));
        out
    }

    fn chase_status(&mut self, pair: PairId, now_ms: u64) -> Vec<Out> {
        let recently_asked = self.last_status_request_ms.is_some_and(|at| now_ms.saturating_sub(at) <= STATUS_REQUEST_GAP_MS);
        let retries = self.gimbals.get(&pair).filter(|gimbal| !gimbal.received_status).map(|gimbal| gimbal.status_retries).unwrap_or(0);
        if recently_asked || retries == 0 {
            return Vec::new();
        }
        self.last_status_request_ms = Some(now_ms);
        self.gimbals.entry(pair).and_modify(|gimbal| gimbal.status_retries -= 1);
        let interval_us = match retries > 2 {
            true => DEFAULT_INTERVAL_US,
            false => SLOW_STATUS_INTERVAL_US,
        };
        vec![Out::MessageInterval { component: pair.manager_compid, message: MSG_GIMBAL_MANAGER_STATUS, interval_us }]
    }

    fn chase_attitude(&mut self, pair: PairId) -> Vec<Out> {
        let wanted = self.gimbals.get(&pair).is_some_and(|gimbal| !gimbal.received_attitude && gimbal.attitude_retries > 0 && gimbal.received_information);
        if !wanted {
            return Vec::new();
        }
        self.gimbals.entry(pair).and_modify(|gimbal| gimbal.attitude_retries -= 1);
        let component = match pair.device_id <= NON_MAVLINK_DEVICE_IDS {
            true => pair.manager_compid,
            false => pair.device_id,
        };
        vec![Out::MessageInterval { component, message: MSG_GIMBAL_DEVICE_ATTITUDE_STATUS, interval_us: DEFAULT_INTERVAL_US }]
    }

    fn complete(&mut self, pair: PairId) -> Vec<Out> {
        self.gimbals.entry(pair).and_modify(|gimbal| gimbal.complete = true);
        let activated = self.active.is_none();
        self.active = self.active.or(Some(pair));
        activated.then_some(Out::ActiveGimbal(pair)).into_iter().chain([Out::GimbalComplete(pair)]).collect()
    }
}

pub static GIMBALS: LazyLock<Mutex<Gimbals>> = LazyLock::new(|| Mutex::new(Gimbals::default()));

pub fn lock() -> MutexGuard<'static, Gimbals> {
    GIMBALS.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn gimbal_view(_backend: &dyn crate::router::Backend, _args: &[String]) -> Value {
    lock().snapshot(crate::hub::now_ms())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUR_SYSTEM: u8 = 255;
    const OUR_COMPONENT: u8 = 190;
    const MANAGER: u8 = 1;
    const DEVICE: u8 = 154;

    fn pair() -> PairId {
        PairId { manager_compid: MANAGER, device_id: DEVICE }
    }

    fn info(compid: u8, device_id: u8, capability_flags: u32) -> ManagerInformation {
        ManagerInformation { compid, device_id, capability_flags, limits_rad: None }
    }

    fn status(compid: u8, device_id: u8, primary_sysid: u8, primary_compid: u8) -> ManagerStatus {
        ManagerStatus { compid, device_id, primary_sysid, primary_compid, ..Default::default() }
    }

    fn attitude(compid: u8, device_id: u8, flags: u32, q: [f32; 4]) -> DeviceAttitude {
        DeviceAttitude { compid, device_id, flags, q, ..Default::default() }
    }

    fn level() -> [f32; 4] {
        [1.0, 0.0, 0.0, 0.0]
    }

    fn yawed(degrees: f32) -> [f32; 4] {
        let half = degrees.to_radians() / 2.0;
        [half.cos(), 0.0, 0.0, half.sin()]
    }

    fn discovered_with(q: [f32; 4]) -> Gimbals {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        gimbals.on_manager_information(info(MANAGER, DEVICE, CAP_HAS_RETRACT | CAP_HAS_YAW_LOCK), 0);
        gimbals.on_manager_status(status(MANAGER, DEVICE, OUR_SYSTEM, OUR_COMPONENT), 2000);
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, q), 2000);
        gimbals
    }

    fn discovered() -> Gimbals {
        discovered_with(level())
    }

    fn angles(out: &[Out]) -> [f64; 7] {
        out.iter()
            .find_map(|sent| match sent {
                Out::Command { params, command: CMD_DO_GIMBAL_MANAGER_PITCHYAW, .. } => Some(*params),
                _ => None,
            })
            .expect("a pitch/yaw command")
    }

    fn refusal(out: &[Out]) -> Option<&'static str> {
        out.iter().find_map(|sent| match sent {
            Out::Refused { reason, .. } => Some(*reason),
            _ => None,
        })
    }

    #[test]
    fn the_wire_values_are_the_ones_the_protocol_defines_and_not_merely_the_ones_the_code_uses() {
        assert_eq!(
            (MSG_GIMBAL_MANAGER_INFORMATION, MSG_GIMBAL_MANAGER_STATUS, MSG_GIMBAL_DEVICE_ATTITUDE_STATUS),
            (280, 281, 285),
            "every other assertion in this file reads the flag through the same constant the code writes, so a transposed number would pass them all - these literals are the only thing standing between a typo and a vehicle sent nonsense"
        );
        assert_eq!((CMD_DO_GIMBAL_MANAGER_PITCHYAW, CMD_DO_GIMBAL_MANAGER_CONFIGURE), (1000, 1001));
        assert_eq!(
            (FLAG_RETRACT, FLAG_NEUTRAL, FLAG_ROLL_LOCK, FLAG_PITCH_LOCK, FLAG_YAW_LOCK, FLAG_YAW_IN_VEHICLE_FRAME, FLAG_YAW_IN_EARTH_FRAME),
            (1, 2, 4, 8, 16, 32, 64)
        );
        assert_eq!((CAP_HAS_RETRACT, CAP_HAS_YAW_LOCK), (1, 1024));
        assert_eq!(
            (
                ERROR_AT_ROLL_LIMIT,
                ERROR_AT_PITCH_LIMIT,
                ERROR_AT_YAW_LIMIT,
                ERROR_ENCODER,
                ERROR_POWER,
                ERROR_MOTOR,
                ERROR_SOFTWARE,
                ERROR_COMMS,
                ERROR_CALIBRATION_RUNNING,
                ERROR_NO_MANAGER
            ),
            (1, 2, 4, 8, 16, 32, 64, 128, 256, 512)
        );
        assert_eq!((RELEASE_CONTROL, LEAVE_UNCHANGED), (-3.0, -1.0));
        assert_eq!((ANGLE_UNITS, RATE_UNITS), ("degree", "degreePerSecond"), "the payload carries a unit identifier a head maps to its own locale, not an English abbreviation an operator reading Ukrainian has to be shown");
    }

    #[test]
    fn nothing_is_chased_until_the_initial_connection_has_completed() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        assert!(gimbals.on_heartbeat(MANAGER, 0).is_empty(), "asking a component for gimbal information during the initial connect is what the ready gate exists to stop");
        assert!(gimbals.on_manager_information(info(MANAGER, DEVICE, 0), 0).is_empty());
        assert!(gimbals.pairs().is_empty(), "and no gimbal is invented from a message that was never accepted");
        assert_eq!(gimbals.snapshot(0)["discovery"], DISCOVERY_IDLE, "a screen that has not started looking says so rather than showing the same emptiness as a search that failed");
        gimbals.set_ready(true);
        assert_eq!(gimbals.on_heartbeat(MANAGER, 0), vec![Out::RequestMessage { component: MANAGER, message: MSG_GIMBAL_MANAGER_INFORMATION }]);
        assert_eq!(gimbals.snapshot(0)["discovery"], DISCOVERY_SEARCHING);
    }

    #[test]
    fn discovery_says_whether_to_keep_waiting_and_what_is_still_missing() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        let spent: Vec<Value> = (0..MANAGER_INFORMATION_RETRIES + 1)
            .map(|attempt| {
                gimbals.on_heartbeat(MANAGER, attempt as u64 * INFORMATION_REQUEST_TIMEOUT_MS);
                gimbals.snapshot(attempt as u64 * INFORMATION_REQUEST_TIMEOUT_MS)["discovery"].clone()
            })
            .collect();
        assert_eq!(spent.first(), Some(&json!(DISCOVERY_SEARCHING)));
        assert_eq!(
            spent.last(),
            Some(&json!(DISCOVERY_NONE)),
            "once the retries are spent nothing will ever change this screen again, so it must stop claiming a search is in progress or the operator waits forever"
        );
        gimbals.on_manager_information(info(2, 155, 0), 0);
        let pending = gimbals.snapshot(0);
        assert_eq!(pending["discovery"], DISCOVERY_SEARCHING);
        assert_eq!(pending["pending"][0]["missing"], json!(["status", "attitude"]), "a manager that answered information and went quiet says which of the three messages is still outstanding");
        gimbals.on_manager_status(status(2, 155, OUR_SYSTEM, OUR_COMPONENT), 2000);
        gimbals.on_device_attitude_status(attitude(155, 0, FLAG_YAW_IN_VEHICLE_FRAME, level()), 2000);
        let found = gimbals.snapshot(2000);
        assert_eq!(found["discovery"], DISCOVERY_FOUND);
        assert!(found["pending"].as_array().is_some_and(|listed| listed.is_empty()));
    }

    #[test]
    fn only_one_information_request_is_in_flight_and_the_guard_clears_itself() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        assert_eq!(gimbals.on_heartbeat(MANAGER, 0).len(), 1);
        assert!(gimbals.on_heartbeat(2, 0).is_empty(), "a second manager waits its turn instead of colliding with the first request in the command queue");
        assert!(gimbals.on_heartbeat(3, INFORMATION_REQUEST_TIMEOUT_MS - 1).is_empty());
        assert_eq!(gimbals.on_heartbeat(3, INFORMATION_REQUEST_TIMEOUT_MS).len(), 1, "a reply that never arrives clears the guard, or the guard is a latch that ends discovery for good");
    }

    #[test]
    fn a_retry_is_only_spent_on_a_request_that_was_actually_sent() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        let sent: usize = (0..MANAGER_INFORMATION_RETRIES + 4).map(|attempt| gimbals.on_heartbeat(MANAGER, attempt as u64 * INFORMATION_REQUEST_TIMEOUT_MS).len()).sum();
        assert_eq!(sent, MANAGER_INFORMATION_RETRIES as usize, "the retries bound how many requests a silent manager gets, and then the asking stops");
        let mut blocked = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        blocked.set_ready(true);
        blocked.on_heartbeat(2, 0);
        (0..MANAGER_INFORMATION_RETRIES + 4).for_each(|_| {
            blocked.on_heartbeat(MANAGER, 0);
        });
        assert_eq!(
            blocked.on_heartbeat(MANAGER, INFORMATION_REQUEST_TIMEOUT_MS).len(),
            1,
            "a manager whose every attempt was suppressed still has its retries when the queue frees up, because a retry spent on nothing is a retry lost"
        );
    }

    #[test]
    fn a_gimbal_device_id_of_zero_names_no_gimbal() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        assert!(gimbals.on_manager_information(info(MANAGER, 0, CAP_HAS_RETRACT), 0).is_empty());
        assert!(gimbals.on_manager_status(status(MANAGER, 0, OUR_SYSTEM, OUR_COMPONENT), 0).is_empty());
        assert!(gimbals.pairs().is_empty(), "a manager reporting device 0 is reporting no device, not a device numbered zero");
        assert!(gimbals.on_device_attitude_status(attitude(MANAGER, 7, 0, level()), 0).is_empty(), "7 and up is not a device id this protocol can address");
        assert!(gimbals.on_device_attitude_status(attitude(DEVICE, 0, 0, level()), 0).is_empty(), "and an attitude from a device nobody has heard of is not a discovery");
        gimbals.on_manager_information(info(MANAGER, DEVICE, 0), 0);
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, level()), 0);
        assert!(gimbals.get(pair()).is_some_and(|gimbal| gimbal.attitude.is_some()), "device id 0 in an attitude means the sending component id is the device");
    }

    #[test]
    fn a_gimbal_is_complete_only_once_all_three_messages_have_arrived() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        let information = gimbals.on_manager_information(info(MANAGER, DEVICE, CAP_HAS_RETRACT), 0);
        assert!(information.contains(&Out::MessageInterval { component: MANAGER, message: MSG_GIMBAL_MANAGER_STATUS, interval_us: DEFAULT_INTERVAL_US }));
        assert!(
            information.contains(&Out::MessageInterval { component: DEVICE, message: MSG_GIMBAL_DEVICE_ATTITUDE_STATUS, interval_us: DEFAULT_INTERVAL_US }),
            "a device id above 6 is a real component, so its attitude is requested from the device itself"
        );
        assert!(!gimbals.get(pair()).unwrap().complete);
        assert!(gimbals.active().is_none(), "an incomplete gimbal is not something a head can be pointed at");
        gimbals.on_manager_status(status(MANAGER, DEVICE, OUR_SYSTEM, OUR_COMPONENT), 2000);
        let done = gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, level()), 2000);
        assert_eq!(done, vec![Out::ActiveGimbal(pair()), Out::GimbalComplete(pair())], "the first complete gimbal becomes the active one");
        assert!(gimbals.on_device_attitude_status(attitude(DEVICE, 0, 0, level()), 3000).is_empty(), "a complete gimbal is never chased again");
    }

    #[test]
    fn a_potential_gimbal_is_neither_listed_nor_selectable() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        gimbals.on_manager_status(status(MANAGER, 3, 0, 0), 0);
        let view = gimbals.snapshot(0);
        assert_eq!((view["count"].as_u64(), view["available"].as_bool()), (Some(0), Some(false)), "one stray status from any component must not put a gimbal in the indicator that a head counts");
        assert!(view["gimbals"].as_array().is_some_and(|listed| listed.is_empty()));
        let potential = PairId { manager_compid: MANAGER, device_id: 3 };
        assert!(
            gimbals.set_active(potential).is_empty() && gimbals.active().is_none(),
            "and the selector cannot activate it, or the next centre command is fired at a device whose existence was never confirmed"
        );
        assert_eq!(gimbals.center(0), vec![Out::Refused { pair: None, reason: REASON_NO_ACTIVE_GIMBAL }, Out::StopRateRepeat]);
    }

    #[test]
    fn a_non_mavlink_device_is_addressed_through_its_manager() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        let out = gimbals.on_manager_information(info(MANAGER, 3, 0), 0);
        assert!(
            out.contains(&Out::MessageInterval { component: MANAGER, message: MSG_GIMBAL_DEVICE_ATTITUDE_STATUS, interval_us: DEFAULT_INTERVAL_US }),
            "device ids 1 to 6 are not components, so only the manager can be asked for their attitude"
        );
    }

    #[test]
    fn the_status_request_slows_down_on_its_last_attempts_and_is_never_spammed() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        gimbals.on_manager_information(info(MANAGER, DEVICE, 0), 0);
        assert!(
            !gimbals.on_manager_information(info(MANAGER, DEVICE, 0), 500).iter().any(|out| matches!(out, Out::MessageInterval { message: MSG_GIMBAL_MANAGER_STATUS, .. })),
            "two status requests inside a second is the spam the gap exists to stop"
        );
        let intervals: Vec<f64> = (1..STATUS_RETRIES as u64)
            .flat_map(|attempt| gimbals.on_manager_information(info(MANAGER, DEVICE, 0), attempt * (STATUS_REQUEST_GAP_MS + 1)))
            .filter_map(|out| match out {
                Out::MessageInterval { message: MSG_GIMBAL_MANAGER_STATUS, interval_us, .. } => Some(interval_us),
                _ => None,
            })
            .collect();
        assert_eq!(
            intervals,
            vec![DEFAULT_INTERVAL_US, DEFAULT_INTERVAL_US, DEFAULT_INTERVAL_US, SLOW_STATUS_INTERVAL_US, SLOW_STATUS_INTERVAL_US],
            "the last two attempts ask for a fixed slow rate instead of the default one"
        );
        assert!(
            gimbals.on_manager_information(info(MANAGER, DEVICE, 0), 100_000).iter().all(|out| !matches!(out, Out::MessageInterval { message: MSG_GIMBAL_MANAGER_STATUS, .. })),
            "the retries run out and the asking stops"
        );
    }

    #[test]
    fn the_yaw_frame_falls_back_to_the_yaw_lock_flag_when_neither_frame_flag_is_set() {
        assert_eq!(Frame::of(FLAG_YAW_IN_VEHICLE_FRAME), Frame::Vehicle);
        assert_eq!(Frame::of(FLAG_YAW_IN_EARTH_FRAME), Frame::Earth);
        assert_eq!(Frame::of(FLAG_YAW_IN_VEHICLE_FRAME | FLAG_YAW_IN_EARTH_FRAME), Frame::Vehicle, "the vehicle frame flag settles a gimbal that sets both");
        assert_eq!(Frame::of(FLAG_YAW_LOCK), Frame::Earth, "an older gimbal sends neither frame flag, and then its yaw lock names the frame");
        assert_eq!(Frame::of(0), Frame::Vehicle);
    }

    #[test]
    fn both_yaw_readings_wrap_at_the_half_circle() {
        assert_eq!(wrap180(380.0), 20.0);
        assert_eq!(wrap180(-190.0), 170.0);
        assert_eq!(wrap180(180.0), -180.0, "a half turn is named once, not twice");
        assert_eq!(wrap180(-900.0), -180.0, "and a bearing wraps however many turns it is out by, not just one");
        let mut gimbals = discovered();
        gimbals.set_heading(Some(350.0), 3000);
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, yawed(30.0)), 3000);
        let reading = gimbals.get(pair()).unwrap().attitude.unwrap();
        assert!((reading.body_yaw.unwrap() - 30.0).abs() < 0.01);
        assert!((reading.absolute_yaw.unwrap() - 20.0).abs() < 0.01, "30 degrees of body yaw on a heading of 350 points at 20, not at 380");
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_EARTH_FRAME, yawed(-170.0)), 3000);
        let reading = gimbals.get(pair()).unwrap().attitude.unwrap();
        assert!((reading.absolute_yaw.unwrap() + 170.0).abs() < 0.01);
        assert!((reading.body_yaw.unwrap() + 160.0).abs() < 0.01, "-170 in the earth frame on a heading of 350 is -160 in the body frame, not -520");
    }

    #[test]
    fn the_delta_yaw_the_gimbal_sends_converts_the_frames_before_any_gcs_heading_does() {
        let mut gimbals = discovered();
        gimbals.on_device_attitude_status(
            DeviceAttitude { compid: DEVICE, device_id: 0, flags: FLAG_YAW_IN_VEHICLE_FRAME, q: yawed(30.0), delta_yaw_rad: Some(90.0_f32.to_radians()), ..Default::default() },
            3000,
        );
        let reading = gimbals.get(pair()).unwrap().attitude.unwrap();
        assert!(
            (reading.absolute_yaw.unwrap() - 120.0).abs() < 0.01,
            "the aircraft sent the body-to-earth offset in the same message, so an absolute yaw is available with no vehicle heading at all instead of being reported unknown"
        );
        gimbals.set_heading(Some(10.0), 4000);
        gimbals.on_device_attitude_status(
            DeviceAttitude { compid: DEVICE, device_id: 0, flags: FLAG_YAW_IN_VEHICLE_FRAME, q: yawed(30.0), delta_yaw_rad: Some(90.0_f32.to_radians()), ..Default::default() },
            4000,
        );
        let reading = gimbals.get(pair()).unwrap().attitude.unwrap();
        assert!((reading.absolute_yaw.unwrap() - 120.0).abs() < 0.01, "and the gimbal's own offset is the authoritative one, so it wins over a heading the GCS supplied");
        let mut locked = discovered();
        locked.on_device_attitude_status(
            DeviceAttitude {
                compid: DEVICE,
                device_id: 0,
                flags: FLAG_YAW_IN_VEHICLE_FRAME | FLAG_YAW_LOCK,
                q: yawed(10.0),
                delta_yaw_rad: Some(90.0_f32.to_radians()),
                ..Default::default()
            },
            3000,
        );
        assert!(
            (angles(&locked.on_screen_control(0.0, 0.0, Screen::Drag { slide_speed: 100.0 }, 3000))[1] - 100.0).abs() < 0.01,
            "a yaw-locked gimbal is draggable on delta yaw alone, rather than being a dead control until a heading turns up"
        );
    }

    #[test]
    fn a_yaw_the_core_cannot_convert_stays_absent() {
        let mut gimbals = discovered();
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, yawed(30.0)), 3000);
        let reading = gimbals.get(pair()).unwrap().attitude.unwrap();
        assert!(reading.body_yaw.is_some() && reading.absolute_yaw.is_none(), "without a vehicle heading the absolute yaw is unknown, and unknown is neither the body yaw nor zero");
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_EARTH_FRAME, yawed(30.0)), 3000);
        let reading = gimbals.get(pair()).unwrap().attitude.unwrap();
        assert!(reading.absolute_yaw.is_some() && reading.body_yaw.is_none());
    }

    #[test]
    fn an_attitude_that_was_never_measured_is_absent_rather_than_level() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        gimbals.on_manager_information(info(MANAGER, DEVICE, 0), 0);
        assert!(gimbals.get(pair()).unwrap().attitude.is_none(), "a gimbal that has not reported an attitude is not known to be pointing straight ahead");
        assert_eq!(euler_degrees([f32::NAN, 0.0, 0.0, 0.0]), None);
        assert_eq!(euler_degrees([0.0, 0.0, 0.0, 0.0]), None, "the protocol sends an all-zero or NaN quaternion when the attitude is unknown, and unknown is not level");
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, yawed(30.0)), 0);
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME | FLAG_RETRACT, [f32::NAN; 4]), 0);
        let gimbal = gimbals.get(pair()).unwrap();
        assert!(gimbal.attitude.is_some_and(|a| a.body_yaw.is_some_and(|yaw| (yaw - 30.0).abs() < 0.01)), "an unknown quaternion leaves the last known angles standing instead of erasing them to zero");
        assert_eq!(gimbal.retracted, Some(true), "and the flags in the same message are still read");
    }

    #[test]
    fn a_frozen_attitude_carries_its_age_and_stops_claiming_to_be_a_mode() {
        let mut gimbals = discovered();
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME | FLAG_YAW_LOCK, yawed(30.0)), 2000);
        let live = gimbals.snapshot(2500)["gimbals"][0].clone();
        assert_eq!((live["attitudeAgeMs"].as_u64(), live["attitudeStale"].as_bool(), live["yawLock"].as_bool()), (Some(500), Some(false), Some(true)));
        let frozen = gimbals.snapshot(2001 + ATTITUDE_STALE_MS)["gimbals"][0].clone();
        assert_eq!(
            frozen["attitudeStale"],
            true,
            "a gimbal that stopped reporting looks exactly like one reporting live, and the operator slews to a target on the strength of the number beside it"
        );
        assert_eq!(frozen["attitudeAgeMs"], 3001);
        assert!(frozen["pitch"].as_f64().is_some(), "the last known angles stay standing, but never without their age");
        assert!(
            frozen["yawLock"].is_null() && frozen["retracted"].is_null() && frozen["neutral"].is_null(),
            "past the window nobody knows what mode the gimbal is in, and a lock toggle drawn from a frozen flag unlocks the gimbal the operator meant to lock"
        );
        (0..4).for_each(|_| {
            gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, [f32::NAN; 4]), 20_000);
        });
        assert_eq!(
            gimbals.snapshot(20_000)["gimbals"][0]["attitudeStale"],
            true,
            "a gimbal that started sending NaN quaternions has stopped measuring, however fast its messages keep arriving"
        );
    }

    #[test]
    fn a_mode_nobody_has_reported_is_unknown_rather_than_off() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        gimbals.on_manager_information(info(MANAGER, DEVICE, CAP_HAS_YAW_LOCK), 0);
        let gimbal = gimbals.get(pair()).unwrap();
        assert_eq!(
            (gimbal.yaw_lock, gimbal.retracted, gimbal.neutral),
            (None, None, None),
            "before the first attitude message nothing is known about the mode, and claiming a gimbal is deployed and unlocked is a definite claim about hardware nobody has heard from"
        );
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME | FLAG_YAW_LOCK, level()), 0);
        let gimbal = gimbals.get(pair()).unwrap();
        assert_eq!((gimbal.yaw_lock, gimbal.retracted, gimbal.neutral), (Some(true), Some(false), Some(false)));
    }

    #[test]
    fn the_reason_a_gimbal_is_not_moving_is_published_rather_than_inferred() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        gimbals.on_manager_information(info(MANAGER, DEVICE, CAP_HAS_RETRACT), 0);
        assert!(gimbals.get(pair()).unwrap().failure_flags.is_none(), "no attitude message means no report of health, which is not the same as a clean bill of it");
        gimbals.on_manager_status(status(MANAGER, DEVICE, OUR_SYSTEM, OUR_COMPONENT), 2000);
        gimbals.on_device_attitude_status(
            DeviceAttitude {
                compid: DEVICE,
                device_id: 0,
                flags: FLAG_YAW_IN_VEHICLE_FRAME,
                q: level(),
                failure_flags: ERROR_AT_PITCH_LIMIT | ERROR_MOTOR | ERROR_CALIBRATION_RUNNING,
                ..Default::default()
            },
            2000,
        );
        let gimbal = gimbals.snapshot(2000)["gimbals"][0].clone();
        assert_eq!(gimbal["failureFlags"], (ERROR_AT_PITCH_LIMIT | ERROR_MOTOR | ERROR_CALIBRATION_RUNNING) as u64);
        assert_eq!(
            (gimbal["failures"]["atPitchLimit"].as_bool(), gimbal["failures"]["motorError"].as_bool(), gimbal["failures"]["calibrationRunning"].as_bool()),
            (Some(true), Some(true), Some(true)),
            "the aircraft already labelled why the commands are not moving the gimbal, and a screen that can only show an angle that will not change sends the operator after the radio link instead"
        );
        assert_eq!((gimbal["failures"]["atRollLimit"].as_bool(), gimbal["failures"]["powerError"].as_bool()), (Some(false), Some(false)));
        assert!(
            gimbals.snapshot(2000)["gimbals"][0]["failures"]["noManager"].as_bool() == Some(false),
            "and a head reads a named boolean rather than decoding a bitmask twice in two heads"
        );
    }

    #[test]
    fn the_hardware_travel_limits_come_from_the_message_that_carries_them() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        gimbals.on_manager_information(info(MANAGER, DEVICE, 0), 0);
        let unknown = gimbals.snapshot(0)["pending"][0].clone();
        assert_eq!(unknown["deviceId"], DEVICE);
        gimbals.on_manager_information(
            ManagerInformation {
                compid: MANAGER,
                device_id: DEVICE,
                capability_flags: 0,
                limits_rad: Some([f32::NAN, f32::NAN, (-120.0_f32).to_radians(), 30.0_f32.to_radians(), (-180.0_f32).to_radians(), 180.0_f32.to_radians()]),
            },
            0,
        );
        gimbals.on_manager_status(status(MANAGER, DEVICE, OUR_SYSTEM, OUR_COMPONENT), 2000);
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, level()), 2000);
        let limits = gimbals.snapshot(2000)["gimbals"][0]["limits"].clone();
        assert!(
            limits["pitchMin"].as_f64().is_some_and(|deg| (deg + 120.0).abs() < 0.01) && limits["pitchMax"].as_f64().is_some_and(|deg| (deg - 30.0).abs() < 0.01),
            "a pitch slider cannot be drawn without the travel, and every head that invents plus or minus 90 is wrong on a gimbal that pitches -120 to +30"
        );
        assert!((limits["yawMin"].as_f64().unwrap() + 180.0).abs() < 0.01 && (limits["yawMax"].as_f64().unwrap() - 180.0).abs() < 0.01);
        assert!(limits["rollMin"].is_null() && limits["rollMax"].is_null(), "an axis the manager did not report is unknown rather than a travel of zero");
    }

    #[test]
    fn a_capability_nobody_has_reported_is_unknown_rather_than_missing() {
        let mut gimbals = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gimbals.set_ready(true);
        gimbals.on_manager_status(status(MANAGER, DEVICE, OUR_SYSTEM, OUR_COMPONENT), 0);
        let gimbal = gimbals.get(pair()).unwrap();
        assert_eq!((gimbal.supports_retract(), gimbal.supports_yaw_lock()), (None, None), "the capability flags arrive with the manager information, and until they do the answer is not known");
        gimbals.on_manager_information(info(MANAGER, DEVICE, CAP_HAS_YAW_LOCK), 0);
        let gimbal = gimbals.get(pair()).unwrap();
        assert_eq!((gimbal.supports_retract(), gimbal.supports_yaw_lock()), (Some(false), Some(true)));
    }

    #[test]
    fn a_command_the_gimbal_cannot_honour_is_refused_with_a_reason() {
        let mut gimbals = discovered_with(level());
        gimbals.on_manager_information(info(MANAGER, DEVICE, CAP_HAS_YAW_LOCK), 2000);
        assert_eq!(
            gimbals.set_retract(true, 2000),
            vec![Out::Refused { pair: Some(pair()), reason: REASON_UNSUPPORTED }],
            "a gimbal that reported no retract capability is not sent a retract that will be silently dropped"
        );
        assert!(gimbals.set_yaw_lock(true, 2000).iter().any(|out| matches!(out, Out::Command { .. })), "the capability it did report is still commanded");
        let unknown = discovered_with([f32::NAN; 4]);
        assert_eq!(
            unknown.snapshot(2000)["gimbals"][0]["aim"],
            json!({ "offer": "blocked", "reason": REASON_ATTITUDE_UNKNOWN }),
            "the operator drags on the video and nothing moves, so the screen names the cause the core already knows instead of sending them after the video link"
        );
        assert_eq!(unknown.snapshot(2000)["gimbals"][0]["control"], json!({ "offer": "ready", "reason": "" }), "an unknown attitude blocks aiming relative to it, not every command there is");
        let mut locked = discovered();
        locked.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME | FLAG_YAW_LOCK, yawed(10.0)), 2000);
        assert_eq!(locked.snapshot(2000)["gimbals"][0]["aim"]["reason"], REASON_HEADING_UNKNOWN);
        assert_eq!(locked.on_screen_control(1.0, 0.0, Screen::Drag { slide_speed: 100.0 }, 2000), vec![Out::Refused { pair: Some(pair()), reason: REASON_HEADING_UNKNOWN }]);
        let mut taken = discovered();
        taken.on_manager_status(status(MANAGER, DEVICE, 42, 190), 3000);
        assert_eq!(taken.snapshot(3000)["gimbals"][0]["control"], json!({ "offer": "blocked", "reason": REASON_OTHERS_HAVE_CONTROL }));
        let mut connecting = discovered();
        connecting.set_ready(false);
        assert_eq!(connecting.center(3000), vec![Out::Refused { pair: Some(pair()), reason: REASON_NOT_READY }, Out::StopRateRepeat]);
        assert_eq!(connecting.snapshot(3000)["gimbals"][0]["control"]["reason"], REASON_NOT_READY);
    }

    #[test]
    fn control_ownership_comes_from_the_status_and_names_who_holds_it() {
        let mut gimbals = discovered();
        let owner = |gimbals: &Gimbals, now: u64| (gimbals.get(pair()).unwrap().have_control(now), gimbals.get(pair()).unwrap().others_have_control());
        assert_eq!(owner(&gimbals, 2000), (Some(true), Some(false)));
        gimbals.on_manager_status(ManagerStatus { compid: MANAGER, device_id: DEVICE, primary_sysid: 42, primary_compid: 190, secondary_sysid: 7, secondary_compid: 191 }, 3000);
        assert_eq!(owner(&gimbals, 3000), (Some(false), Some(true)));
        let held = gimbals.snapshot(3000)["gimbals"][0].clone();
        assert_eq!(
            (held["primarySysid"].as_u64(), held["primaryCompid"].as_u64()),
            (Some(42), Some(190)),
            "overriding the autopilot's own mount control and overriding the second operator on this aircraft are different decisions, so the holder is named rather than anonymous"
        );
        assert_eq!((held["secondarySysid"].as_u64(), held["secondaryCompid"].as_u64()), (Some(7), Some(191)));
        gimbals.on_manager_status(status(MANAGER, DEVICE, 0, 0), 4000);
        assert_eq!(owner(&gimbals, 4000), (Some(false), Some(false)), "nobody in control is not somebody else in control");
        assert_eq!(owner(&gimbals, 4000 + CONTROL_STALE_MS), (Some(false), Some(false)), "inside the window the last status still stands");
        assert_eq!(
            owner(&gimbals, 4001 + CONTROL_STALE_MS),
            (None, Some(false)),
            "our own hold expires with its evidence, because a flag that outlives it is a latch that lets a command through on nothing"
        );
        assert_eq!(gimbals.snapshot(4001 + CONTROL_STALE_MS)["gimbals"][0]["ownershipAgeMs"], 15_001, "and the age of the evidence is published beside the hold it supports");
        assert!(gimbals.get(pair()).unwrap().complete, "the gimbal itself does not go away just because its status went quiet");
    }

    #[test]
    fn a_command_asks_before_taking_a_gimbal_somebody_else_was_holding() {
        let mut gimbals = discovered();
        gimbals.on_manager_status(status(MANAGER, DEVICE, 42, 190), 3000);
        assert_eq!(gimbals.center(3000), vec![Out::AskToTakeControl(pair()), Out::StopRateRepeat], "someone else is pointing it, so the operator is asked and nothing is sent");
        assert_eq!(
            gimbals.center(100_000),
            vec![Out::AskToTakeControl(pair()), Out::StopRateRepeat],
            "the status drops out for fifteen seconds on three lost packets at the slow fallback rate, and silence is not the other station letting go - the first click must not steal their camera mid-flight"
        );
        gimbals.on_manager_status(status(MANAGER, DEVICE, 0, 0), 4000);
        let taken = gimbals.center(4000);
        assert!(matches!(taken.first(), Some(Out::Command { command: CMD_DO_GIMBAL_MANAGER_CONFIGURE, .. })), "a status that says nobody holds it is the evidence that releases the ask");
        assert!(taken.iter().any(|out| matches!(out, Out::Command { command: CMD_DO_GIMBAL_MANAGER_PITCHYAW, .. })), "and the command still goes out");
        let stale = gimbals.center(100_000);
        assert!(matches!(stale.first(), Some(Out::Command { command: CMD_DO_GIMBAL_MANAGER_CONFIGURE, .. })), "with our own hold expired control is asked for again rather than assumed");
        let nothing = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        assert_eq!(refusal(&nothing.acquire_control()), Some(REASON_NO_ACTIVE_GIMBAL), "with no active gimbal the core says why rather than doing nothing in silence");
        assert_eq!(refusal(&nothing.release_control()), Some(REASON_NO_ACTIVE_GIMBAL));
    }

    #[test]
    fn acquiring_names_us_and_releasing_leaves_the_secondary_alone() {
        let gimbals = discovered();
        let params = |out: &[Out]| match out.first() {
            Some(Out::Command { params, command: CMD_DO_GIMBAL_MANAGER_CONFIGURE, component: MANAGER, .. }) => *params,
            _ => panic!("a configure command"),
        };
        let acquire = params(&gimbals.acquire_control());
        assert_eq!((acquire[0], acquire[1]), (OUR_SYSTEM as f64, OUR_COMPONENT as f64), "taking control names this station as the primary");
        assert_eq!((acquire[2], acquire[3]), (LEAVE_UNCHANGED, LEAVE_UNCHANGED), "and says nothing about whoever holds secondary control");
        assert_eq!(acquire[6], DEVICE as f64);
        let release = params(&gimbals.release_control());
        assert_eq!((release[0], release[1], release[2]), (RELEASE_CONTROL, RELEASE_CONTROL, LEAVE_UNCHANGED));
    }

    #[test]
    fn an_angle_target_leaves_an_axis_the_core_does_not_know_unset() {
        let mut gimbals = discovered_with([f32::NAN; 4]);
        let params = angles(&gimbals.set_retract(true, 2000));
        assert_eq!(params[4], 1.0);
        assert!(params[0].is_nan() && params[1].is_nan(), "a retract must not invent a zero pitch and yaw out of an attitude that was never measured, and an unset axis is what NaN means here");
        gimbals.set_heading(Some(10.0), 2000);
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, yawed(30.0)), 2000);
        let params = angles(&gimbals.set_yaw_lock(true, 2000));
        assert_eq!(params[4], 28.0, "roll lock, pitch lock and yaw lock, asserted as the sum the vehicle receives rather than through the same names the code builds it from");
        assert!((params[1] - 40.0).abs() < 0.01, "locking the yaw holds the current angle in the earth frame those flags select, which is the body yaw plus the heading");
        let params = angles(&gimbals.set_yaw_lock(false, 2000));
        assert!((params[1] - 30.0).abs() < 0.01, "and unlocking holds the same direction expressed in the vehicle frame");
        assert_eq!((params[5], params[6]), (0.0, DEVICE as f64));
    }

    #[test]
    fn an_angle_command_wraps_its_yaw_and_stops_the_rate_repeat() {
        let mut gimbals = discovered();
        gimbals.set_rates(Some(10.0), Some(-5.0), 2000);
        let body = gimbals.send_pitch_body_yaw(-20.0, 190.0, false, 2000);
        assert!(body.contains(&Out::StopRateRepeat), "an angle target supersedes the rates, so whatever keeps resending them has to stop");
        assert_eq!((gimbals.get(pair()).unwrap().commanded_pitch_rate, gimbals.get(pair()).unwrap().commanded_yaw_rate), (Some(0.0), Some(0.0)));
        let params = angles(&body);
        assert_eq!((params[0], params[1]), (-20.0, -170.0), "190 degrees of body yaw is -170, and a bearing outside the circle never reaches the vehicle");
        assert_eq!(params[4], 44.0, "roll lock, pitch lock and yaw in the vehicle frame");
        assert!(gimbals.get(pair()).unwrap().attitude.is_some(), "and commanding an angle does not overwrite the measured attitude with the target");
        let params = angles(&gimbals.send_pitch_absolute_yaw(0.0, -200.0, true, 2000));
        assert_eq!(params[1], 160.0);
        assert_eq!(params[4], 92.0, "roll lock, pitch lock, yaw lock and yaw in the earth frame");
    }

    #[test]
    fn rates_go_out_as_the_acked_command_in_degrees_per_second() {
        let mut gimbals = discovered();
        let started = gimbals.set_rates(Some(30.0), None, 2000);
        let sent = match started.first() {
            Some(Out::Command { params, command: CMD_DO_GIMBAL_MANAGER_PITCHYAW, component: MANAGER, show_error: false }) => *params,
            _ => panic!("an acked pitch/yaw command, not a fire-and-forget stream message this GCS has never shipped a sender for"),
        };
        assert!(sent[0].is_nan() && sent[1].is_nan(), "a rate command leaves both angles unset, which is what NaN means in these two parameters");
        assert_eq!(
            (sent[2], sent[3], sent[4], sent[5], sent[6]),
            (30.0, 0.0, 12.0, 0.0, DEVICE as f64),
            "the rate rides in parameters three and four in degrees per second, under roll lock and pitch lock alone"
        );
        assert_eq!(started.last(), Some(&Out::StartRateRepeat));
        assert!(gimbals.set_rates(None, Some(-15.0), 3000).contains(&Out::StartRateRepeat), "one axis still moving keeps the repeat alive");
        assert!(gimbals.set_rates(Some(0.0), Some(0.0), 4000).contains(&Out::StopRateRepeat));
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME | FLAG_YAW_LOCK, level()), 5000);
        let locked = gimbals.set_rates(Some(5.0), None, 5000);
        assert!(
            matches!(locked.first(), Some(Out::Command { params, .. }) if params[4] == 28.0),
            "a rate carries roll lock, pitch lock and the yaw lock the gimbal is already in - and no frame flag, because a yaw locked to North and a yaw relative to the airframe are contradictory instructions and the gimbal honours the frame"
        );
    }

    #[test]
    fn a_stop_is_stored_even_when_the_command_cannot_go_out() {
        let mut gimbals = discovered();
        gimbals.set_rates(Some(30.0), Some(30.0), 2000);
        gimbals.on_manager_status(status(MANAGER, DEVICE, 42, 190), 2500);
        let released = gimbals.set_rates(Some(0.0), Some(0.0), 2500);
        assert_eq!(
            (gimbals.get(pair()).unwrap().commanded_pitch_rate, gimbals.get(pair()).unwrap().commanded_yaw_rate),
            (Some(0.0), Some(0.0)),
            "the operator let go of the stick while another station held the gimbal, and a zero dropped at the gate is a 30 deg/s slew waiting to be resent the moment control comes back"
        );
        assert!(
            released.contains(&Out::StopRateRepeat),
            "and whatever is resending the rate is stopped on the gated path too, or it keeps firing the take-control prompt at the operator forever"
        );
        gimbals.on_manager_status(status(MANAGER, DEVICE, OUR_SYSTEM, OUR_COMPONENT), 3000);
        let resumed = gimbals.set_rates(None, None, 3000);
        assert!(
            matches!(resumed.first(), Some(Out::Command { params, .. }) if params[2] == 0.0 && params[3] == 0.0),
            "with control back the stored rate is the stop the operator asked for, not the slew they abandoned"
        );
        assert!(resumed.contains(&Out::StopRateRepeat));
        let mut gone = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT);
        gone.set_ready(true);
        assert_eq!(refusal(&gone.set_rates(Some(0.0), None, 0)), Some(REASON_NO_ACTIVE_GIMBAL), "and with nothing to store into the core says so rather than returning silence");
    }

    #[test]
    fn the_commanded_rate_is_never_published_as_a_measurement() {
        let mut gimbals = discovered();
        let fresh = gimbals.snapshot(2000)["gimbals"][0].clone();
        assert!(
            fresh["commandedPitchRate"].is_null() && fresh["commandedYawRate"].is_null(),
            "a rate this station has never commanded is unknown, not a measured zero beside a pitch of -30"
        );
        assert!(fresh["measuredPitchRate"].is_null() && fresh["measuredYawRate"].is_null());
        gimbals.set_rates(Some(30.0), None, 2000);
        gimbals.on_device_attitude_status(
            DeviceAttitude {
                compid: DEVICE,
                device_id: 0,
                flags: FLAG_YAW_IN_VEHICLE_FRAME,
                q: level(),
                angular_velocity_rad_s: Some([0.0, 12.0_f32.to_radians(), (-40.0_f32).to_radians()]),
                ..Default::default()
            },
            2000,
        );
        let gimbal = gimbals.snapshot(2000)["gimbals"][0].clone();
        assert_eq!(gimbal["commandedPitchRate"], 30.0);
        assert!(
            gimbal["measuredPitchRate"].as_f64().is_some_and(|rate| (rate - 12.0).abs() < 0.01) && gimbal["measuredYawRate"].as_f64().is_some_and(|rate| (rate + 40.0).abs() < 0.01),
            "the gimbal is slewing at 40 deg/s of yaw while this station commanded none of it, and a screen showing our own command as telemetry draws a still camera that is moving"
        );
        assert!(gimbal.get("pitchRate").is_none() && gimbal.get("yawRate").is_none(), "an unqualified rate beside a measured pitch reads as telemetry, so the name says which one it is");
    }

    #[test]
    fn on_screen_control_moves_from_where_the_gimbal_is_pointing() {
        let mut gimbals = discovered();
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME, yawed(10.0)), 2000);
        let params = angles(&gimbals.on_screen_control(1.0, 0.5, Screen::Point { h_fov: 60.0, v_fov: 40.0 }, 2000));
        assert!((params[1] - 40.0).abs() < 0.01, "a click at the right edge is half a horizontal field of view away from where the gimbal already points");
        assert!((params[0] - 10.0).abs() < 0.01, "and a click halfway down is a quarter of the vertical field of view");
        let params = angles(&gimbals.on_screen_control(-1.0, 0.0, Screen::Drag { slide_speed: 100.0 }, 2000));
        assert!(params[1].abs() < 0.01, "a drag steps a tenth of the slide speed, from 10 degrees back to zero");
        let mut unknown = discovered_with([f32::NAN; 4]);
        assert_eq!(
            unknown.on_screen_control(1.0, 0.0, Screen::Drag { slide_speed: 100.0 }, 2000),
            vec![Out::Refused { pair: Some(pair()), reason: REASON_ATTITUDE_UNKNOWN }],
            "with no measured yaw there is no point to move relative to, and the refusal says that rather than returning an empty list the operator cannot see"
        );
        let mut locked = discovered();
        locked.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME | FLAG_YAW_LOCK, yawed(10.0)), 2000);
        assert_eq!(
            refusal(&locked.on_screen_control(1.0, 0.0, Screen::Drag { slide_speed: 100.0 }, 2000)),
            Some(REASON_HEADING_UNKNOWN),
            "a yaw-locked gimbal is aimed in the earth frame, which needs the vehicle heading, and here it is not known"
        );
        locked.set_heading(Some(90.0), 2000);
        locked.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME | FLAG_YAW_LOCK, yawed(10.0)), 2000);
        assert!((angles(&locked.on_screen_control(0.0, 0.0, Screen::Drag { slide_speed: 100.0 }, 2000))[1] - 100.0).abs() < 0.01);
    }

    #[test]
    fn a_heading_carries_its_age_and_stops_being_used_once_it_is_old() {
        let mut gimbals = discovered();
        gimbals.set_heading(Some(90.0), 2000);
        assert_eq!(gimbals.heading_at(2000 + HEADING_STALE_MS), Some(90.0));
        assert_eq!(gimbals.heading_at(2001 + HEADING_STALE_MS), None, "an absolute yaw computed from a heading of unknown age is a bearing nobody measured");
        let old = gimbals.snapshot(2001 + HEADING_STALE_MS);
        assert!(old["heading"].is_null() && old["headingAgeMs"] == 3001, "the age is published beside it so a head can say the bearing is old rather than silently dropping it");
    }

    #[test]
    fn the_active_gimbal_is_one_of_the_gimbals_and_changes_only_when_it_changes() {
        let mut gimbals = discovered();
        assert_eq!(gimbals.active(), Some(pair()));
        assert!(gimbals.set_active(pair()).is_empty(), "naming the gimbal that is already active tells a head nothing");
        assert!(gimbals.set_active(PairId { manager_compid: 9, device_id: 9 }).is_empty(), "and a gimbal nobody has heard from cannot become the active one");
        let second = PairId { manager_compid: 2, device_id: 155 };
        gimbals.on_manager_information(info(2, 155, 0), 5000);
        gimbals.on_manager_status(status(2, 155, OUR_SYSTEM, OUR_COMPONENT), 6000);
        let complete = gimbals.on_device_attitude_status(attitude(155, 0, FLAG_YAW_IN_VEHICLE_FRAME, level()), 6000);
        assert_eq!(complete, vec![Out::GimbalComplete(second)], "a second gimbal does not steal the active slot from the first");
        assert_eq!(gimbals.set_active(second), vec![Out::ActiveGimbal(second)]);
        assert_eq!(gimbals.pairs(), vec![pair(), second]);
    }

    #[test]
    fn the_snapshot_emits_numbers_and_unit_names_only() {
        let mut gimbals = discovered();
        gimbals.set_heading(Some(90.0), 2000);
        gimbals.on_device_attitude_status(attitude(DEVICE, 0, FLAG_YAW_IN_VEHICLE_FRAME | FLAG_NEUTRAL, yawed(45.0)), 2000);
        let view = gimbals.snapshot(2000);
        assert_eq!((view["class"].as_str(), view["available"].as_bool(), view["count"].as_u64()), (Some("Gimbals"), Some(true), Some(1)));
        assert_eq!((view["angleUnits"].as_str(), view["rateUnits"].as_str()), (Some(ANGLE_UNITS), Some(RATE_UNITS)), "the core names the unit and never renders the number, because a decimal separator belongs to the head");
        assert_eq!(view["active"]["deviceId"], DEVICE);
        let gimbal = &view["gimbals"][0];
        assert!(gimbal["bodyYaw"].as_f64().is_some_and(|yaw| (yaw - 45.0).abs() < 0.01) && gimbal["absoluteYaw"].as_f64().is_some_and(|yaw| (yaw - 135.0).abs() < 0.01));
        assert_eq!((gimbal["neutral"].as_bool(), gimbal["yawFrame"].as_str(), gimbal["active"].as_bool()), (Some(true), Some("vehicle"), Some(true)));
        assert_eq!((gimbal["supportsRetract"].as_bool(), gimbal["haveControl"].as_bool()), (Some(true), Some(true)));
        assert!(gimbals.snapshot(100_000)["gimbals"][0]["haveControl"].is_null(), "past the staleness window the snapshot says it does not know who is in control rather than guessing");
        let bare = Gimbals::new(OUR_SYSTEM, OUR_COMPONENT).snapshot(0);
        assert_eq!(bare["available"], false);
        assert!(bare["active"].is_null() && bare["heading"].is_null(), "nothing measured yet reads as absent, not as zero");
        assert!(bare["gimbals"].as_array().is_some_and(|listed| listed.is_empty()));
    }
}
