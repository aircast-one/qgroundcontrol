use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;
use web_time::Instant;

use crate::modes::{COPTER_MODES, Mode, PLANE_MODES, PX4_MODES, ROVER_MODES, SUB_MODES};
use crate::read::{flag, object, value_number};
use crate::router::Backend;

const PX4_TAKEOVER: &str = "vehicle.parameterManager.getParameter(-1,COM_RC_OVERRIDE).rawValue";

pub const DEPS: &[&str] = &[
    "settings.appSettings.virtualJoystick",
    "settings.appSettings.virtualJoystickLeftHandedMode",
    "vehicles.activeVehicleAvailable",
    "vehicle.rover",
    "vehicle.sub",
    "vehicle.multiRotor",
    "vehicle.fixedWing",
    "vehicle.vtol",
    "vehicle.apmFirmware",
    "vehicle.px4Firmware",
    "vehicle.customMode",
    "vehicle.initialConnectComplete",
    PX4_TAKEOVER,
];

const COPTER_FLIES_ITSELF: [&str; 10] = [
    "Auto",
    "RTL",
    "Brake",
    "Throw",
    "Avoid ADSB",
    "Guided No GPS",
    "Smart RTL",
    "Follow",
    "AutoRotate",
    "AutoRTL",
];
const PLANE_FLIES_ITSELF: [&str; 8] = [
    "Auto",
    "RTL",
    "Takeoff",
    "Avoid ADSB",
    "Guided",
    "QuadPlane Land",
    "QuadPlane RTL",
    "Autoland",
];
const ROVER_FLIES_ITSELF: [&str; 7] = [
    "Hold",
    "Follow",
    "Dock",
    "Auto",
    "RTL",
    "Smart RTL",
    "Guided",
];
const SUB_FLIES_ITSELF: [&str; 3] = ["Auto", "Guided", "Surface"];
const COPTER_FREE_THROTTLE: [&str; 4] = ["Stabilize", "Acro", "Drift", "Flip"];
const PLANE_FREE_THROTTLE: [&str; 9] = [
    "Manual",
    "Stabilize",
    "Training",
    "Acro",
    "FBW A",
    "FBW B",
    "Cruise",
    "QuadPlane Stabilize",
    "QuadPlane Acro",
];
const PX4_FREE_THROTTLE: [&str; 4] = ["Manual", "Stabilized", "Acro", "Rattitude"];
const COPTER_RATE: [&str; 1] = ["Acro"];
const PLANE_RATE: [&str; 2] = ["Acro", "QuadPlane Acro"];
const PX4_RATE: [&str; 2] = ["Acro", "Rattitude"];
pub const RATE_EXPO_PCT: f64 = 70.0;
const PX4_AUTO: u32 = 4;
const PX4_OFFBOARD: u32 = 6;
const TAKEOVER_IN_AUTO: u32 = 1;
const TAKEOVER_IN_OFFBOARD: u32 = 2;

#[derive(Clone, Copy)]
enum Airframe {
    Copter,
    Plane,
    Rover,
    Sub,
    Px4,
    Other,
}

fn airframe(vehicle: &Value) -> Airframe {
    match (flag(vehicle, "px4Firmware"), flag(vehicle, "apmFirmware")) {
        (true, _) => Airframe::Px4,
        (_, true) if flag(vehicle, "multiRotor") => Airframe::Copter,
        (_, true) if flag(vehicle, "fixedWing") || flag(vehicle, "vtol") => Airframe::Plane,
        (_, true) if flag(vehicle, "rover") => Airframe::Rover,
        (_, true) if flag(vehicle, "sub") => Airframe::Sub,
        _ => Airframe::Other,
    }
}

fn px4_main(custom_mode: u32) -> u32 {
    (custom_mode >> 16) & 0xff
}

fn named(table: &[Mode], names: &[&str], custom_mode: u32) -> bool {
    table
        .iter()
        .any(|mode| mode.custom_mode == custom_mode && names.contains(&mode.name))
}

fn flown_by_sticks(airframe: Airframe, custom_mode: u32, takeover: u32) -> bool {
    match airframe {
        Airframe::Copter => !named(&COPTER_MODES, &COPTER_FLIES_ITSELF, custom_mode),
        Airframe::Plane => !named(&PLANE_MODES, &PLANE_FLIES_ITSELF, custom_mode),
        Airframe::Rover => !named(&ROVER_MODES, &ROVER_FLIES_ITSELF, custom_mode),
        Airframe::Sub => !named(&SUB_MODES, &SUB_FLIES_ITSELF, custom_mode),
        Airframe::Px4 => match px4_main(custom_mode) {
            PX4_AUTO => takeover & TAKEOVER_IN_AUTO != 0,
            PX4_OFFBOARD => takeover & TAKEOVER_IN_OFFBOARD != 0,
            _ => true,
        },
        Airframe::Other => true,
    }
}

fn throttle_free(airframe: Airframe, custom_mode: u32, driven: bool) -> bool {
    !driven
        && match airframe {
            Airframe::Copter => named(&COPTER_MODES, &COPTER_FREE_THROTTLE, custom_mode),
            Airframe::Plane => named(&PLANE_MODES, &PLANE_FREE_THROTTLE, custom_mode),
            Airframe::Px4 => PX4_MODES.iter().any(|mode| {
                PX4_FREE_THROTTLE.contains(&mode.name)
                    && px4_main(mode.custom_mode) == px4_main(custom_mode)
            }),
            Airframe::Rover | Airframe::Sub | Airframe::Other => false,
        }
}

fn rate_mode(airframe: Airframe, custom_mode: u32) -> bool {
    match airframe {
        Airframe::Copter => named(&COPTER_MODES, &COPTER_RATE, custom_mode),
        Airframe::Plane => named(&PLANE_MODES, &PLANE_RATE, custom_mode),
        Airframe::Px4 => PX4_MODES.iter().any(|mode| {
            PX4_RATE.contains(&mode.name) && px4_main(mode.custom_mode) == px4_main(custom_mode)
        }),
        Airframe::Rover | Airframe::Sub | Airframe::Other => false,
    }
}

fn softened(axis: f64, rate: bool) -> f64 {
    match rate {
        true => f64::from(crate::joystick::exponential(axis as f32, RATE_EXPO_PCT)),
        false => axis,
    }
}

pub const VEHICLE_FIELDS: &str =
    "rover,sub,multiRotor,fixedWing,vtol,apmFirmware,px4Firmware,customMode";

pub fn rate_mode_now(vehicle: &Value) -> bool {
    custom_mode(vehicle).is_some_and(|custom| rate_mode(airframe(vehicle), custom))
}

fn custom_mode(vehicle: &Value) -> Option<u32> {
    vehicle
        .get("customMode")
        .and_then(Value::as_u64)
        .and_then(|custom| u32::try_from(custom).ok())
}

pub const JOYSTICK_VALUE: &str = "vehicle.virtualTabletJoystickValue";
pub const SEND_PERIOD_MS: u64 = 40;
pub const TAKEOVER_DEFLECTION: f64 = 0.3;
pub const TAKEOVER_HOLD_MS: u64 = 300;
pub const TAKEOVER_RETRY_MS: u64 = 5000;
pub const YAW_EXPO_PCT: f64 = 20.0;
pub const YAW_DEADZONE: f64 = 0.15;
pub const ARM_GESTURE_DEFLECTION: f64 = 0.8;
pub const ARM_GESTURE_THROTTLE: f64 = 0.1;
pub const ARM_HOLD_MS: u64 = 1000;
const THROTTLE_CENTRE: f64 = 0.5;

pub fn arm_gesture(roll: f64, pitch: f64, yaw: f64, thrust: f64) -> bool {
    thrust <= ARM_GESTURE_THROTTLE
        && pitch <= -ARM_GESTURE_DEFLECTION
        && roll.abs() >= ARM_GESTURE_DEFLECTION
        && yaw.abs() >= ARM_GESTURE_DEFLECTION
        && roll.signum() != yaw.signum()
}

pub fn stick_deflected(roll: f64, pitch: f64, yaw: f64, thrust: f64, flying: bool) -> bool {
    [roll, pitch, yaw]
        .iter()
        .any(|axis| axis.abs() > TAKEOVER_DEFLECTION)
        || thrust - THROTTLE_CENTRE > TAKEOVER_DEFLECTION
        || (flying && THROTTLE_CENTRE - thrust > TAKEOVER_DEFLECTION)
}

fn setting(backend: &dyn Backend, name: &str) -> bool {
    backend
        .value(&format!("settings.appSettings.{name}"))
        .get("value")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn sending(backend: &dyn Backend) -> bool {
    let vehicles = backend.value_fields("vehicles", "activeVehicleAvailable");
    flag(&vehicles, "activeVehicleAvailable")
        && flag(
            &backend.value_fields("vehicle", "initialConnectComplete"),
            "initialConnectComplete",
        )
}

pub fn virtual_joystick_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = backend.value_fields("vehicle", VEHICLE_FIELDS);
    let connected = flag(
        &backend.value_fields("vehicles", "activeVehicleAvailable"),
        "activeVehicleAvailable",
    );
    let rover = connected && flag(&vehicle, "rover");
    let driven = rover || flag(&vehicle, "sub");
    let frame = airframe(&vehicle);
    let mode = connected.then(|| custom_mode(&vehicle)).flatten();
    let takeover = matches!(frame, Airframe::Px4)
        .then(|| value_number(&backend.value(PX4_TAKEOVER)))
        .flatten()
        .map_or(TAKEOVER_IN_AUTO, |bits| bits as u32);
    let left_handed = setting(backend, "virtualJoystickLeftHandedMode");
    let enabled = setting(backend, "virtualJoystick");
    let sending = sending(backend);
    json!({
        "kind": "object",
        "class": "VirtualJoystick",
        "enabled": enabled,
        "show": enabled && sending && mode.is_none_or(|custom| flown_by_sticks(frame, custom, takeover)),
        "sending": sending,
        "autoCenterThrottle": !mode.is_some_and(|custom| throttle_free(frame, custom, driven)),
        "softened": mode.is_some_and(|custom| rate_mode(frame, custom)),
        "leftHandedMode": left_handed,
        "leftPositiveOnly": connected && !rover && !left_handed,
        "rightPositiveOnly": connected && !rover && left_handed,
        "periodMs": SEND_PERIOD_MS,
    })
}

const REPORT_EVERY: Duration = Duration::from_secs(10);
const REMEMBERED: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StickSample {
    pub at: Instant,
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
    pub thrust: f64,
    pub touched: Option<Instant>,
}

impl StickSample {
    fn axes(&self) -> [f64; 4] {
        [self.roll, self.pitch, self.yaw, self.thrust]
    }
}

static SENT: Mutex<Vec<StickSample>> = Mutex::new(Vec::new());
static HELD: Mutex<Option<([f64; 4], Instant)>> = Mutex::new(None);

pub fn hold(axes: [f64; 4], for_ms: u64) {
    *HELD.lock().unwrap_or_else(PoisonError::into_inner) =
        Some((axes, Instant::now() + Duration::from_millis(for_ms)));
}

fn take_hold(touched: bool) -> Option<([f64; 4], Instant)> {
    let mut held = HELD.lock().unwrap_or_else(PoisonError::into_inner);
    match touched {
        true => held.take(),
        false => *held,
    }
}

fn steered(
    given: [f64; 4],
    held: Option<([f64; 4], Instant)>,
    now: Instant,
    touched: bool,
) -> [f64; 4] {
    held.filter(|(_, until)| !touched && now < *until)
        .map_or(given, |(axes, _)| axes)
}

pub fn remembered(kept: &[StickSample], sample: StickSample) -> Vec<StickSample> {
    kept.iter()
        .copied()
        .filter(|old| sample.at.saturating_duration_since(old.at) <= REMEMBERED)
        .chain(std::iter::once(sample))
        .collect()
}

pub fn sent_sticks() -> Vec<StickSample> {
    SENT.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

fn remember(sample: StickSample) {
    let moved = {
        let mut kept = SENT.lock().unwrap_or_else(PoisonError::into_inner);
        let moved = kept.last().is_none_or(|last| last.axes() != sample.axes());
        *kept = remembered(&kept, sample);
        moved
    };
    if moved {
        wake_view();
    }
}

fn wake_view() {
    #[cfg(all(feature = "synthetic", native_synthetic, not(test)))]
    crate::synthetic::host::wake();
}

fn past_deadzone(yaw: f64) -> f64 {
    ((yaw.abs() - YAW_DEADZONE).max(0.0) / (1.0 - YAW_DEADZONE)).copysign(yaw)
}

pub fn yaw_sent(yaw: f64) -> f64 {
    f64::from(crate::joystick::exponential(yaw as f32, YAW_EXPO_PCT))
}

pub fn copter_mode_now(vehicle: &Value) -> Option<&'static str> {
    let custom = custom_mode(vehicle)?;
    matches!(airframe(vehicle), Airframe::Copter)
        .then(|| {
            COPTER_MODES
                .iter()
                .find(|mode| mode.custom_mode == custom)
                .map(|mode| mode.name)
        })
        .flatten()
}
const PAUSED: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq)]
pub struct StickWindow {
    since: Instant,
    last: Instant,
    sends: u64,
    refused: BTreeMap<&'static str, u64>,
    touch_to_send_ms: Vec<u64>,
    handling_ms: Vec<u64>,
    gap_max: Duration,
    rate_mode: bool,
}

impl StickWindow {
    pub fn opened(now: Instant) -> Self {
        Self {
            since: now,
            last: now,
            sends: 0,
            refused: BTreeMap::new(),
            touch_to_send_ms: Vec::new(),
            handling_ms: Vec::new(),
            gap_max: Duration::ZERO,
            rate_mode: false,
        }
    }

    pub fn recorded(
        &self,
        now: Instant,
        touch_age_ms: Option<u64>,
        handling: Duration,
        refusal: Option<&'static str>,
        rate_mode: bool,
    ) -> Self {
        Self {
            since: self.since,
            last: now,
            sends: self.sends + 1,
            refused: self
                .refused
                .iter()
                .map(|(reason, count)| (*reason, *count))
                .chain(
                    refusal
                        .map(|reason| (reason, self.refused.get(reason).copied().unwrap_or(0) + 1)),
                )
                .collect(),
            touch_to_send_ms: self
                .touch_to_send_ms
                .iter()
                .copied()
                .chain(touch_age_ms)
                .collect(),
            handling_ms: self
                .handling_ms
                .iter()
                .copied()
                .chain(std::iter::once(
                    u64::try_from(handling.as_millis()).unwrap_or(u64::MAX),
                ))
                .collect(),
            gap_max: self.gap_max.max(now.saturating_duration_since(self.last)),
            rate_mode: self.rate_mode || rate_mode,
        }
    }

    pub fn closes(&self, now: Instant) -> bool {
        self.sends > 0
            && (now.saturating_duration_since(self.since) >= REPORT_EVERY
                || now.saturating_duration_since(self.last) >= PAUSED)
    }

    pub fn line(&self) -> String {
        format!(
            "event=sticks seconds={} sends={} refused={} refusals={} moves={} touch_to_send_p50_ms={} touch_to_send_max_ms={} handling_p50_ms={} handling_max_ms={} send_gap_max_ms={} rate_mode={}",
            self.last.saturating_duration_since(self.since).as_secs(),
            self.sends,
            self.refused.values().sum::<u64>(),
            match self.refused.is_empty() {
                true => "none".to_string(),
                false => self
                    .refused
                    .iter()
                    .map(|(reason, count)| format!("{reason}:{count}"))
                    .collect::<Vec<_>>()
                    .join(","),
            },
            self.touch_to_send_ms.len(),
            crate::stats::shown(crate::stats::median(&self.touch_to_send_ms)),
            crate::stats::shown(self.touch_to_send_ms.iter().copied().max()),
            crate::stats::shown(crate::stats::median(&self.handling_ms)),
            crate::stats::shown(self.handling_ms.iter().copied().max()),
            self.gap_max.as_millis(),
            self.rate_mode,
        )
    }
}

static STICKS: Mutex<Option<StickWindow>> = Mutex::new(None);

fn closed(held: Option<StickWindow>, now: Instant) -> Option<StickWindow> {
    match held {
        Some(open) if open.closes(now) => {
            log::info!(target: "sticks", "{}", open.line());
            None
        }
        kept => kept,
    }
}

fn note_send(
    entered: Instant,
    touch_age_ms: Option<u64>,
    refusal: Option<&'static str>,
    rate_mode: bool,
) {
    let now = Instant::now();
    let mut held = STICKS.lock().unwrap_or_else(PoisonError::into_inner);
    let window = closed(held.take(), now).unwrap_or_else(|| StickWindow::opened(now));
    *held = Some(window.recorded(
        now,
        touch_age_ms,
        now.saturating_duration_since(entered),
        refusal,
        rate_mode,
    ));
}

pub fn flush(now: Instant) {
    let mut held = STICKS.lock().unwrap_or_else(PoisonError::into_inner);
    *held = closed(held.take(), now);
}

fn refused(entered: Instant, reason: &'static str, said: &str, touch_age_ms: Option<u64>) -> Value {
    note_send(entered, touch_age_ms, Some(reason), false);
    json!({ "ok": false, "reason": said })
}

pub fn send(backend: &dyn Backend, args: &str) -> Value {
    let entered = Instant::now();
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let touch_age_ms = given.get(4).and_then(Value::as_u64);
    let axis = |i: usize| {
        given
            .get(i)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .map(|v| v.clamp(-1.0, 1.0))
    };
    let (Some(roll), Some(pitch), Some(yaw), Some(thrust)) = (axis(0), axis(1), axis(2), axis(3))
    else {
        return refused(
            entered,
            "bad_axes",
            "The virtual joystick sends roll, pitch, yaw and thrust, each from -1 to 1.",
            touch_age_ms,
        );
    };
    let touched = touch_age_ms.is_some();
    let [roll, pitch, yaw, thrust] = steered(
        [roll, pitch, yaw, thrust],
        take_hold(touched),
        entered,
        touched,
    );
    if !sending(backend) {
        return refused(
            entered,
            "not_connected",
            "No vehicle has finished connecting.",
            touch_age_ms,
        );
    }
    let vehicle = backend.value_fields("vehicle", VEHICLE_FIELDS);
    let rate = rate_mode_now(&vehicle);
    let (roll, pitch) = (softened(roll, rate), softened(pitch, rate));
    let yaw = past_deadzone(yaw);
    let thrust = match rate {
        true => THROTTLE_CENTRE + softened(2.0 * (thrust - THROTTLE_CENTRE), true) / 2.0,
        false => thrust,
    };
    let active = crate::hub::lock().active_id();
    if active.is_some_and(crate::joystickhost::drives) {
        return refused(
            entered,
            "joystick_active",
            "A joystick is enabled for this vehicle, so the on-screen sticks do not send.",
            touch_age_ms,
        );
    }
    remember(StickSample {
        at: entered,
        roll,
        pitch,
        yaw: yaw_sent(yaw),
        thrust,
        touched: touch_age_ms.and_then(|age| entered.checked_sub(Duration::from_millis(age))),
    });
    let action = json!({ "action": "virtualJoystick", "roll": roll, "pitch": pitch, "yaw": yaw, "thrust": thrust });
    let dispatched = crate::rcoverride::on_core(backend, action).unwrap_or_else(|| {
        flag(
            &object(&backend.invoke(
                JOYSTICK_VALUE,
                &json!([roll, pitch, yaw, thrust]).to_string(),
            )),
            "ok",
        )
    });
    note_send(
        entered,
        touch_age_ms,
        (!dispatched).then_some("dispatch_failed"),
        rate,
    );
    json!({ "ok": dispatched })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sent_sticks_are_remembered_for_two_seconds() {
        let start = Instant::now();
        let at = |ms: u64| StickSample {
            at: start + Duration::from_millis(ms),
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.1,
            thrust: 0.5,
            touched: None,
        };
        let kept = [0, 500, 1_500]
            .iter()
            .fold(Vec::new(), |kept, ms| remembered(&kept, at(*ms)));
        assert_eq!(
            remembered(&kept, at(2_400))
                .iter()
                .map(|sample| sample.at)
                .collect::<Vec<_>>(),
            [500, 1_500, 2_400].map(|ms| start + Duration::from_millis(ms)),
            "older than two seconds is dropped"
        );
        assert!(
            (yaw_sent(0.5) - 0.425).abs() < 1e-6,
            "the history holds the yaw the vehicle was sent, after the expo"
        );
    }

    #[test]
    fn a_stick_window_reports_touch_to_send_times_and_closes_on_a_pause() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let window = [
            (0, Some(12), None),
            (20, None, Some("not_connected")),
            (60, Some(30), Some("dispatch_failed")),
            (80, None, Some("not_connected")),
            (100, Some(8), None),
        ]
        .into_iter()
        .fold(StickWindow::opened(at(0)), |window, (ms, age, refusal)| {
            window.recorded(at(ms), age, Duration::from_millis(ms / 20), refusal, false)
        });
        assert_eq!(
            window.line(),
            "event=sticks seconds=0 sends=5 refused=3 refusals=dispatch_failed:1,not_connected:2 moves=3 touch_to_send_p50_ms=12 touch_to_send_max_ms=30 handling_p50_ms=3 handling_max_ms=5 send_gap_max_ms=40 rate_mode=false"
        );
        assert!(
            StickWindow::opened(at(0))
                .recorded(at(5), None, Duration::ZERO, None, true)
                .line()
                .contains("refused=0 refusals=none"),
            "a clean window says so"
        );
        assert!(!window.closes(at(500)), "still sending");
        assert!(
            window.closes(at(1_100)),
            "a second without a send ends the window, so a pause is not counted as a gap"
        );
        assert!(window.closes(at(10_000)));
        assert!(
            !StickWindow::opened(at(0)).closes(at(20_000)),
            "an empty window has nothing to report"
        );
        assert_eq!(
            closed(Some(window.clone()), at(1_100)),
            None,
            "the pump flushes a window once the sticks go quiet, so the last one of a flight is not lost"
        );
        assert_eq!(
            closed(Some(window.clone()), at(500)),
            Some(window),
            "a window still in use stays open"
        );
    }

    #[test]
    fn both_sticks_pulled_into_the_bottom_corners_is_the_arming_gesture() {
        assert!(arm_gesture(-1.0, -1.0, 1.0, 0.0), "inward, like a DJI");
        assert!(
            arm_gesture(1.0, -1.0, -1.0, 0.0),
            "outward, which is also inward with the throttle on the right stick"
        );
        assert!(
            !arm_gesture(1.0, -1.0, 1.0, 0.0),
            "both sticks pointing the same way"
        );
        assert!(!arm_gesture(-1.0, -1.0, 1.0, 0.5), "throttle not down");
        assert!(
            !arm_gesture(-1.0, 0.0, 1.0, 0.0),
            "the other stick not pulled down"
        );
        assert!(!arm_gesture(-0.5, -1.0, 1.0, 0.0), "short of the corner");
    }

    #[test]
    fn a_stick_pushed_well_off_centre_is_the_pilot_taking_over() {
        assert!(!stick_deflected(0.0, 0.0, 0.0, 0.5, true), "sticks at rest");
        assert!(
            !stick_deflected(0.2, -0.25, 0.1, 0.7, true),
            "small corrections and a brush of the throttle do not count"
        );
        assert!(stick_deflected(0.0, -0.6, 0.0, 0.5, true), "pitch forward");
        assert!(stick_deflected(0.0, 0.0, 0.5, 0.5, true), "a turn");
        assert!(stick_deflected(0.0, 0.0, 0.0, 0.95, false), "climbing");
        assert!(
            stick_deflected(0.0, 0.0, 0.0, 0.0, true),
            "pulling the throttle down to descend"
        );
        assert!(
            !stick_deflected(0.0, 0.0, 0.0, 0.0, false),
            "on the ground the throttle rests at the bottom"
        );
    }
    use crate::modes::px4;
    use std::cell::RefCell;

    struct Rig {
        kind: &'static str,
        left_handed: bool,
        connected: bool,
        custom: u32,
        takeover: Option<f64>,
        invoked: RefCell<Vec<String>>,
    }

    impl Backend for Rig {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.appSettings.virtualJoystick" => json!({ "kind": "fact", "value": true }),
                "settings.appSettings.virtualJoystickLeftHandedMode" => {
                    json!({ "kind": "fact", "value": self.left_handed })
                }
                PX4_TAKEOVER => self.takeover.map_or(
                    json!({ "kind": "null" }),
                    |bits| json!({ "kind": "value", "value": bits }),
                ),
                _ => json!({ "kind": "fact", "value": false }),
            }
            .to_string()
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            let is = |kind: &str| self.kind == kind;
            match path {
                "vehicles" => json!({ "activeVehicleAvailable": self.connected }),
                _ => json!({
                    "rover": is("rover"), "multiRotor": is("copter") || is("px4"), "fixedWing": is("plane"), "apmFirmware": !is("px4"), "px4Firmware": is("px4"),
                    "initialConnectComplete": self.connected, "customMode": self.custom, "flightMode": "SHOUTED BY THE VEHICLE",
                }),
            }
            .to_string()
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }
        fn invoke(&self, path: &str, args: &str) -> String {
            self.invoked.borrow_mut().push(format!("{path} {args}"));
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _paths: &[String]) {}
    }

    fn on(kind: &'static str, mode: &str) -> Rig {
        let table: &[Mode] = match kind {
            "plane" => &PLANE_MODES,
            "rover" => &ROVER_MODES,
            "px4" => &PX4_MODES,
            _ => &COPTER_MODES,
        };
        let custom = table
            .iter()
            .find(|entry| entry.name == mode)
            .map(|entry| entry.custom_mode)
            .expect("a mode of that firmware");
        Rig {
            kind,
            left_handed: false,
            connected: true,
            custom,
            takeover: None,
            invoked: RefCell::new(Vec::new()),
        }
    }

    fn rig(rover: bool, left_handed: bool, connected: bool) -> Rig {
        let (kind, mode) = if rover {
            ("rover", "Manual")
        } else {
            ("copter", "Loiter")
        };
        Rig {
            left_handed,
            connected,
            ..on(kind, mode)
        }
    }

    #[test]
    fn the_throttle_springs_back_where_the_autopilot_holds_height_and_stays_put_where_the_pilot_sets_it()
     {
        let throttle = |rig: Rig| virtual_joystick_view(&rig, &[])["autoCenterThrottle"].clone();
        assert_eq!(
            throttle(on("copter", "Loiter")),
            json!(true),
            "like a DJI, letting go holds height"
        );
        assert_eq!(throttle(on("copter", "Altitude Hold")), json!(true));
        assert_eq!(
            throttle(on("copter", "Stabilize")),
            json!(false),
            "the stick is the throttle, so it stays where the pilot leaves it"
        );
        assert_eq!(throttle(on("copter", "Acro")), json!(false));
        assert_eq!(throttle(on("plane", "FBW A")), json!(false));
        assert_eq!(throttle(on("px4", "Manual")), json!(false));
        assert_eq!(throttle(on("px4", "Position")), json!(true));
        assert_eq!(
            throttle(on("rover", "Manual")),
            json!(true),
            "a rover let go of stops"
        );
    }

    #[test]
    fn the_sticks_hide_while_the_autopilot_flies_itself_but_keep_sending_centred() {
        let shown = |rig: Rig| virtual_joystick_view(&rig, &[])["show"].clone();
        assert_eq!(shown(on("copter", "Loiter")), json!(true));
        assert_eq!(shown(on("copter", "Stabilize")), json!(true));
        assert_eq!(
            shown(on("copter", "RTL")),
            json!(false),
            "returning home ignores the sticks, so they are not offered"
        );
        assert_eq!(
            shown(on("copter", "Land")),
            json!(true),
            "sticks nudge a landing sideways"
        );
        assert_eq!(
            shown(on("copter", "Guided")),
            json!(true),
            "a copter in Guided hands over to Loiter when a stick moves"
        );
        assert_eq!(
            shown(on("plane", "Guided")),
            json!(false),
            "nothing takes over from Guided on other vehicles"
        );
        assert_eq!(shown(on("plane", "Auto")), json!(false));
        assert_eq!(
            virtual_joystick_view(&on("copter", "RTL"), &[])["enabled"],
            json!(true),
            "hidden sticks still stream centred, so the override link never drops mid-flight"
        );
        assert_eq!(
            shown(Rig {
                connected: false,
                ..on("copter", "Loiter")
            }),
            json!(false),
            "with no vehicle the sticks have nothing to fly"
        );
    }

    #[test]
    fn modes_are_told_apart_by_number_so_a_vehicle_naming_them_itself_changes_nothing() {
        let view = virtual_joystick_view(&on("copter", "Auto"), &[]);
        assert_eq!(
            (view["show"].clone(), view["autoCenterThrottle"].clone()),
            (json!(false), json!(true)),
            "ArduPilot 4.7 announces AUTO and STABILIZE in capitals"
        );
    }

    #[test]
    fn a_px4_pilot_takes_over_a_mission_by_moving_a_stick_unless_the_vehicle_forbids_it() {
        let shown = |rig: Rig| virtual_joystick_view(&rig, &[])["show"].clone();
        assert_eq!(
            shown(on("px4", "Mission")),
            json!(true),
            "COM_RC_OVERRIDE defaults to taking over in auto modes"
        );
        assert_eq!(shown(on("px4", "Hold")), json!(true));
        assert_eq!(
            shown(Rig {
                takeover: Some(0.0),
                ..on("px4", "Mission")
            }),
            json!(false)
        );
        assert_eq!(
            shown(on("px4", "Offboard")),
            json!(false),
            "offboard is only taken over when the vehicle allows it"
        );
        assert_eq!(
            shown(Rig {
                takeover: Some(3.0),
                ..on("px4", "Offboard")
            }),
            json!(true)
        );
        assert_eq!(px4(4, 4) >> 16 & 0xff, PX4_AUTO);
    }

    #[test]
    fn the_throttle_stick_runs_zero_to_one_except_on_a_rover() {
        let right_handed = virtual_joystick_view(&rig(false, false, true), &[]);
        assert_eq!(
            (
                right_handed["leftPositiveOnly"].as_bool(),
                right_handed["rightPositiveOnly"].as_bool()
            ),
            (Some(true), Some(false))
        );
        let left_handed = virtual_joystick_view(&rig(false, true, true), &[]);
        assert_eq!(
            (
                left_handed["leftPositiveOnly"].as_bool(),
                left_handed["rightPositiveOnly"].as_bool()
            ),
            (Some(false), Some(true))
        );
        let rover = virtual_joystick_view(&rig(true, false, true), &[]);
        assert_eq!(rover["leftPositiveOnly"], json!(false));
        assert_eq!(right_handed["show"], json!(true));
    }

    #[test]
    fn in_acro_a_small_thumb_move_asks_for_a_slow_rotation_and_full_stick_still_full_rate() {
        let acro = on("copter", "Acro");
        assert_eq!(send(&acro, "[0.3, -1.0, 0.0, 0.4]")["ok"], json!(true));
        let sent: Vec<f64> = serde_json::from_str(
            acro.invoked.borrow()[0]
                .split_once(' ')
                .expect("path and args")
                .1,
        )
        .expect("four axes");
        assert!(
            (sent[0] - 0.1089).abs() < 1e-3,
            "30% roll is about 11% rate, ~40 deg/s of ArduCopter's 360 ({})",
            sent[0]
        );
        assert_eq!(sent[1], -1.0, "full stick is still full rate");
        assert_eq!(sent[2], 0.0, "yaw keeps the expo every mode gets");
        assert!(
            (sent[3] - 0.4672).abs() < 1e-3,
            "10% below hover is ~3% below in thrust; a thumb twitch climbed or sank at 5 m/s ({})",
            sent[3]
        );
        let ends = on("copter", "Acro");
        send(&ends, "[0, 0, 0, 0]");
        send(&ends, "[0, 0, 0, 1]");
        send(&ends, "[0, 0, 0, 0.5]");
        let thrusts: Vec<f64> = ends
            .invoked
            .borrow()
            .iter()
            .map(|call| {
                serde_json::from_str::<Vec<f64>>(call.split_once(' ').expect("args").1)
                    .expect("axes")[3]
            })
            .collect();
        assert_eq!(
            thrusts,
            [0.0, 1.0, 0.5],
            "the bottom, top and hover points do not move"
        );
        assert_eq!(virtual_joystick_view(&acro, &[])["softened"], json!(true));
        let loiter = on("copter", "Loiter");
        send(&loiter, "[0.3, -1.0, 0.0, 0.4]");
        assert_eq!(
            loiter.invoked.borrow()[0],
            "vehicle.virtualTabletJoystickValue [0.3,-1.0,0.0,0.4]",
            "position modes keep the stick as it is"
        );
        assert_eq!(
            virtual_joystick_view(&on("copter", "Drift"), &[])["softened"],
            json!(false),
            "Drift levels itself"
        );
        assert_eq!(
            virtual_joystick_view(&on("px4", "Acro"), &[])["softened"],
            json!(true)
        );
        assert_eq!(
            virtual_joystick_view(&on("plane", "QuadPlane Acro"), &[])["softened"],
            json!(true)
        );
    }

    #[test]
    fn a_thumb_resting_on_the_yaw_stick_does_not_turn_the_aircraft_and_full_stick_is_still_full() {
        let sent = |yaw: f64| {
            let loiter = on("copter", "Loiter");
            send(&loiter, &format!("[0.0, 0.0, {yaw}, 0.5]"));
            let call = loiter.invoked.borrow()[0].clone();
            serde_json::from_str::<Vec<f64>>(call.split_once(' ').expect("path and args").1)
                .expect("four axes")[2]
        };
        assert_eq!(
            sent(0.12),
            0.0,
            "a quarter of the stray yaw sent while sliding sideways was this small"
        );
        assert_eq!(sent(-0.15), 0.0);
        assert!(
            (sent(0.575) - 0.5).abs() < 1e-9,
            "past the dead zone the stick rescales ({})",
            sent(0.575)
        );
        assert_eq!((sent(1.0), sent(-1.0)), (1.0, -1.0));
    }

    #[test]
    fn values_go_to_the_vehicle_only_once_it_has_connected() {
        let up = rig(false, false, true);
        assert_eq!(send(&up, "[0.1, 0.2, 1.0, 0.9]")["ok"], json!(true));
        assert_eq!(
            up.invoked.borrow().as_slice(),
            ["vehicle.virtualTabletJoystickValue [0.1,0.2,1.0,0.9]"]
        );
        let down = rig(false, false, false);
        assert_eq!(send(&down, "[0, 0, 0, 0]")["ok"], json!(false));
        assert!(down.invoked.borrow().is_empty());
        assert_eq!(send(&up, "[0, 0]")["ok"], json!(false));
    }

    #[test]
    fn a_held_stick_replaces_what_the_screen_streams_until_the_hold_runs_out() {
        let now = Instant::now();
        let later = |ms: u64| now + Duration::from_millis(ms);
        let parked = [0.0, 0.0, 0.0, 0.0];
        let climb = [0.0, 0.0, 0.0, 1.0];
        let held = Some((climb, later(500)));
        assert_eq!(
            steered(parked, held, now, false),
            climb,
            "on the ground the head streams a parked throttle 25 times a second, and it used to overwrite every held sample"
        );
        assert_eq!(steered(parked, held, later(500), false), parked);
        assert_eq!(steered(parked, None, now, false), parked);
        let thumb = [0.4, 0.0, 0.0, 0.5];
        assert_eq!(
            steered(thumb, held, now, true),
            thumb,
            "a finger on the screen always wins over a held stick"
        );
    }
}
