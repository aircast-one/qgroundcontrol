use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::joystick::{ACTION_NONE, ACTIONS, AdditionalAxes, AxisCalibration, ButtonEvent, FUNCTIONS, Input, Joystick, Out, Polling, Settings, Support, ThrottleMode};
use crate::settingsini::Setting;
use crate::stickcal::{Outcome as CalOutcome, StickCal};
use crate::mavout::Outbound;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable"];
pub const DEVICES: &str = "joystick.devices";
pub const INPUT: &str = "joystick.input";
pub const SELECT: &str = "joystick.select";
pub const ENABLE_JOYSTICK: &str = "joystick.enable";
pub const SET_SETTING: &str = "joystick.setting";
pub const CALIBRATION: &str = "joystick.calibration";
pub const BUTTON_ACTION: &str = "joystick.buttonAction";
pub const BUTTON_REPEAT: &str = "joystick.buttonRepeat";
const BUTTON_GROUP: &str = "JoystickButtonActionSettingsArray";
const AXIS_GROUP: &str = "JoystickAxisSettingsArray";
const STORED_TRANSMITTER_MODE: u8 = 2;
const SETTINGS_JSON: &str = include_str!("../../src/Settings/Joystick.SettingsGroup.json");
const ACTIVE_NAME: &str = "JoystickManager/activeJoystickName";
const ENABLED_VEHICLES: &str = "JoystickManager/joystickEnabledVehiclesIds";
const SETTINGS_PREFIX: &str = "JoystickSettingsV2";
const OPTIONAL_SETTINGS: [&str; 8] = ["enableManualControlPitchExtension", "enableManualControlRollExtension", "enableAdditionalAxis1", "enableAdditionalAxis2", "enableAdditionalAxis3", "enableAdditionalAxis4", "enableAdditionalAxis5", "enableAdditionalAxis6"];
const RC_OVERRIDE_TOTAL: usize = 18;
const RC_IGNORE_LOW: u16 = u16::MAX;
const RC_IGNORE_HIGH: u16 = 0;
const RC_LOW_CHANNELS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct Meta {
    pub name: String,
    pub kind: String,
    pub default: Value,
    pub label: String,
    pub units: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

pub static METADATA: LazyLock<Vec<Meta>> = LazyLock::new(|| {
    let parsed: Value = serde_json::from_str(SETTINGS_JSON).unwrap_or(Value::Null);
    parsed["QGC.MetaData.Facts"]
        .as_array()
        .map(|facts| {
            facts
                .iter()
                .map(|f| Meta {
                    name: f["name"].as_str().unwrap_or_default().to_string(),
                    kind: f["type"].as_str().unwrap_or_default().to_string(),
                    default: f["default"].clone(),
                    label: f["label"].as_str().or(f["shortDesc"].as_str()).unwrap_or_default().to_string(),
                    units: f["units"].as_str().unwrap_or_default().to_string(),
                    min: f["min"].as_f64(),
                    max: f["max"].as_f64(),
                })
                .collect()
        })
        .unwrap_or_default()
});

fn setting_key(joystick: &str, name: &str) -> String {
    format!("{SETTINGS_PREFIX}/{joystick}/{name}")
}

pub fn setting_value(joystick: &str, name: &str) -> Value {
    let meta = METADATA.iter().find(|m| m.name == name);
    let stored = crate::settingsstore::stored_text(&setting_key(joystick, name));
    match (meta, stored) {
        (Some(meta), Some(text)) => match meta.kind.as_str() {
            "bool" => json!(text == "true" || text == "1"),
            "double" => text.parse::<f64>().map_or(meta.default.clone(), |v| json!(v)),
            _ => text.parse::<i64>().map_or(meta.default.clone(), |v| json!(v)),
        },
        (Some(meta), None) => meta.default.clone(),
        (None, _) => Value::Null,
    }
}

fn flag(joystick: &str, name: &str) -> bool {
    setting_value(joystick, name).as_bool().unwrap_or(false)
}

fn number(joystick: &str, name: &str) -> f64 {
    setting_value(joystick, name).as_f64().unwrap_or(0.0)
}

pub fn settings_for(joystick: &str) -> Settings {
    Settings {
        calibrated: flag(joystick, "calibrated"),
        circle_correction: flag(joystick, "circleCorrection"),
        use_deadband: flag(joystick, "useDeadband"),
        negative_thrust: flag(joystick, "negativeThrust"),
        throttle_smoothing: flag(joystick, "throttleSmoothing"),
        throttle_mode: if flag(joystick, "throttleModeCenterZero") { ThrottleMode::CenterZero } else { ThrottleMode::DownZero },
        axis_frequency_hz: number(joystick, "axisFrequencyHz"),
        button_frequency_hz: number(joystick, "buttonFrequencyHz"),
        exponential_pct: number(joystick, "exponentialPct"),
        additional_axes: if number(joystick, "additionalAxesFunction") as i64 == 1 { AdditionalAxes::RcChannelsOverride } else { AdditionalAxes::ManualControl },
        optional_enabled: OPTIONAL_SETTINGS.map(|name| flag(joystick, name)),
    }
}

fn coerce(meta: &Meta, value: &Value) -> Option<String> {
    match meta.kind.as_str() {
        "bool" => value.as_bool().map(|b| b.to_string()),
        "double" => value.as_f64().filter(|v| meta.min.is_none_or(|m| *v >= m) && meta.max.is_none_or(|m| *v <= m)).map(|v| v.to_string()),
        _ => value.as_f64().filter(|v| v.fract() == 0.0 && meta.min.is_none_or(|m| *v >= m) && meta.max.is_none_or(|m| *v <= m)).map(|v| (v as i64).to_string()),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub name: String,
    pub axes: usize,
    pub buttons: usize,
    pub hats: usize,
}

#[derive(Default)]
struct Host {
    devices: Vec<Device>,
    joysticks: BTreeMap<String, Joystick>,
    calibration: Option<(String, StickCal)>,
}

fn axis_group(joystick: &str) -> String {
    format!("{SETTINGS_PREFIX}/{joystick}/{AXIS_GROUP}")
}

fn transmitter_mode(joystick: &str) -> u8 {
    setting_value(joystick, "transmitterMode").as_u64().and_then(|m| u8::try_from(m).ok()).filter(|m| (1..=4).contains(m)).unwrap_or(STORED_TRANSMITTER_MODE)
}

pub fn load_axes(joystick: &str, model: &mut Joystick, entries: &BTreeMap<String, Setting>) {
    let group = axis_group(joystick);
    let read = |axis: usize, key: &str| match entries.get(&format!("{group}/{axis}/{key}")) {
        Some(Setting::Text(text)) => text.parse::<i64>().ok().or_else(|| (text == "true").then_some(1)).or_else(|| (text == "false").then_some(0)),
        _ => None,
    };
    (0..model.axis_count()).for_each(|axis| {
        let function = read(axis, "function").and_then(|f| FUNCTIONS.get(f as usize)).map(|(f, _)| *f);
        if let (Some(function), Some(min), Some(max), Some(center)) = (function, read(axis, "min"), read(axis, "max"), read(axis, "center")) {
            let calibration = AxisCalibration { min: min as i32, max: max as i32, center: center as i32, deadband: read(axis, "deadband").unwrap_or(0) as i32, reversed: read(axis, "reversed").unwrap_or(0) != 0 };
            model.set_calibration(axis, calibration);
            model.set_axis_function(function, axis);
        }
    });
    model.set_transmitter_mode(transmitter_mode(joystick));
}

pub fn axis_entries(joystick: &str, model: &Joystick) -> BTreeMap<String, Setting> {
    let mut stored = model.clone();
    stored.set_transmitter_mode(STORED_TRANSMITTER_MODE);
    let group = axis_group(joystick);
    (0..stored.axis_count())
        .filter_map(|axis| Some((axis, stored.function_for(axis)?, stored.calibration(axis)?)))
        .flat_map(|(axis, function, c)| {
            let key = |name: &str| format!("{group}/{axis}/{name}");
            [
                (key("center"), Setting::Text(c.center.to_string())),
                (key("min"), Setting::Text(c.min.to_string())),
                (key("max"), Setting::Text(c.max.to_string())),
                (key("deadband"), Setting::Text(c.deadband.to_string())),
                (key("reversed"), Setting::Text(c.reversed.to_string())),
                (key("function"), Setting::Text(function.index().to_string())),
            ]
        })
        .collect()
}

fn button_group(joystick: &str) -> String {
    format!("{SETTINGS_PREFIX}/{joystick}/{BUTTON_GROUP}")
}

pub fn load_buttons(joystick: &str, model: &mut Joystick, entries: &BTreeMap<String, Setting>) {
    let group = button_group(joystick);
    (0..model.total_button_count()).for_each(|button| {
        if let Some(Setting::Text(action)) = entries.get(&format!("{group}/{button}/actionName")).filter(|a| !matches!(a, Setting::Text(t) if t.is_empty())) {
            model.set_button_action(button, Some(action));
            let repeat = matches!(entries.get(&format!("{group}/{button}/repeat")), Some(Setting::Text(t)) if t == "true");
            model.set_button_repeat(button, repeat, false);
        }
    });
}

pub fn button_entries(joystick: &str, model: &Joystick) -> BTreeMap<String, Setting> {
    let group = button_group(joystick);
    (0..model.total_button_count())
        .filter_map(|button| model.binding(button).map(|b| (button, b.clone())))
        .flat_map(|(button, binding)| [(format!("{group}/{button}/actionName"), Setting::Text(binding.action)), (format!("{group}/{button}/repeat"), Setting::Text(binding.repeat.to_string()))])
        .collect()
}

pub fn assignable_actions(flight_modes: &[String], assigned: &[String]) -> Vec<(String, bool)> {
    let fixed = |range: std::ops::Range<usize>| ACTIONS[range].iter().map(|(_, name, _, repeat)| (name.to_string(), *repeat)).collect::<Vec<_>>();
    let listed: Vec<(String, bool)> = std::iter::once((ACTION_NONE.to_string(), false))
        .chain(fixed(0..3))
        .chain(flight_modes.iter().map(|mode| (mode.clone(), false)))
        .chain(fixed(3..ACTIONS.len()))
        .collect();
    let extra: Vec<(String, bool)> = assigned.iter().filter(|a| !listed.iter().any(|(name, _)| name == *a)).map(|a| (a.clone(), false)).collect();
    listed.into_iter().chain(extra).collect()
}

fn settable_modes() -> Vec<String> {
    crate::hub::lock().active().map(|v| v.flight_modes.iter().filter(|m| m.can_be_set).map(|m| m.name.clone()).collect()).unwrap_or_default()
}

fn guided(id: u8, action: Value) {
    let sent = crate::hub::lock().guided(Some(id), &action, crate::hub::now_ms());
    match sent {
        Ok(frames) => frames.iter().for_each(|(link, bytes)| {
            crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
        }),
        Err(reason) => log::warn!("joystick action {action} was refused: {reason}"),
    }
}

pub fn execute(backend: &dyn Backend, id: u8, armed: bool, action: &str, event: ButtonEvent) {
    let modes = settable_modes();
    let recording = || crate::read::flag(&crate::video::camera_view(backend, &[]), "recording");
    let toggle_recording = || {
        crate::actions::run(backend, "camera.toggleRecording", "[]");
    };
    match (action, event) {
        ("Arm", ButtonEvent::Down) => guided(id, json!({ "action": "arm", "arm": true })),
        ("Disarm", ButtonEvent::Down) => guided(id, json!({ "action": "arm", "arm": false })),
        ("Toggle Arm", ButtonEvent::Down) => guided(id, json!({ "action": "arm", "arm": !armed })),
        ("VTOL: Fixed Wing", ButtonEvent::Down) => guided(id, json!({ "action": "vtolTransition", "forward": true })),
        ("VTOL: Multi-Rotor", ButtonEvent::Down) => guided(id, json!({ "action": "vtolTransition", "forward": false })),
        ("Trigger Camera", ButtonEvent::Down) => guided(id, json!({ "action": "triggerCamera" })),
        ("Start Recording Video", ButtonEvent::Down) if !recording() => toggle_recording(),
        ("Stop Recording Video", ButtonEvent::Down) if recording() => toggle_recording(),
        ("Toggle Recording Video", ButtonEvent::Down) => toggle_recording(),
        ("Gimbal Center", ButtonEvent::Down) => guided(id, json!({ "action": "gimbal", "op": "center" })),
        ("Gimbal Yaw Lock", ButtonEvent::Down) => guided(id, json!({ "action": "gimbal", "op": "yawLock", "lock": true })),
        ("Gimbal Yaw Follow", ButtonEvent::Down) => guided(id, json!({ "action": "gimbal", "op": "yawLock", "lock": false })),
        ("Emergency Stop", ButtonEvent::Down) => guided(id, json!({ "action": "emergencyStop" })),
        ("Gripper Grab", ButtonEvent::Down) => guided(id, json!({ "action": "gripper", "gripAction": 1 })),
        ("Gripper Release", ButtonEvent::Down) => guided(id, json!({ "action": "gripper", "gripAction": 0 })),
        ("Gripper Hold", ButtonEvent::Down) => guided(id, json!({ "action": "gripper", "gripAction": 2 })),
        ("Landing gear deploy", ButtonEvent::Down) => guided(id, json!({ "action": "landingGear", "retract": false })),
        ("Landing gear retract", ButtonEvent::Down) => guided(id, json!({ "action": "landingGear", "retract": true })),
        ("Motor Interlock enable", ButtonEvent::Down) => guided(id, json!({ "action": "motorInterlock", "enable": true })),
        ("Motor Interlock disable", ButtonEvent::Down) => guided(id, json!({ "action": "motorInterlock", "enable": false })),
        (mode, ButtonEvent::Down) if modes.iter().any(|m| m == mode) => guided(id, json!({ "action": "setMode", "mode": mode })),
        (other, _) => log::info!("joystick action {other} has no executor in the core yet"),
    }
}

fn save_buttons(joystick: &str, model: &Joystick) {
    crate::settingsstore::replace_group(&button_group(joystick), button_entries(joystick, model));
}

fn button_change(text: &str, repeat: bool) -> Value {
    let args = serde_json::from_str::<Vec<Value>>(text).unwrap_or_default();
    let Some(button) = args.first().and_then(Value::as_u64).map(|b| b as usize) else { return json!({ "ok": false, "reason": "A button index is required." }) };
    let mut host = host();
    let Some(active) = active_name(&host.devices) else { return json!({ "ok": false, "reason": "No joystick is connected." }) };
    let Some(model) = host.joysticks.get_mut(&active) else { return json!({ "ok": false, "reason": "No joystick is connected." }) };
    let changed = match repeat {
        false => {
            let action = args.get(1).and_then(Value::as_str).filter(|a| *a != ACTION_NONE);
            model.set_button_action(button, action)
        }
        true => model.set_button_repeat(button, args.get(1).and_then(Value::as_bool).unwrap_or(false), false),
    };
    if changed {
        save_buttons(&active, model);
    }
    json!({ "ok": changed, "reason": if changed { Value::Null } else { json!("That button cannot take this setting.") } })
}

fn apply_calibration(joystick: &str, model: &mut Joystick, channels: &[crate::stickcal::Channel]) {
    model.reset_calibration();
    channels.iter().enumerate().for_each(|(axis, channel)| {
        model.set_calibration(axis, StickCal::calibration(channel));
        if let Some(function) = channel.function {
            model.set_axis_function(function, axis);
        }
    });
    crate::settingsstore::replace_group(&axis_group(joystick), axis_entries(joystick, model));
    crate::settingsstore::written(&setting_key(joystick, "calibrated"), "true");
    crate::settingsstore::written(&setting_key(joystick, "transmitterMode"), &model.transmitter_mode().to_string());
}

static HOST: LazyLock<Mutex<Host>> = LazyLock::new(|| Mutex::new(Host::default()));

fn host() -> MutexGuard<'static, Host> {
    HOST.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn enabled_vehicles() -> Vec<String> {
    crate::settingsstore::stored_text(ENABLED_VEHICLES).unwrap_or_default().split(',').filter(|s| !s.is_empty()).map(str::to_string).collect()
}

pub fn active_name(devices: &[Device]) -> Option<String> {
    let stored = crate::settingsstore::stored_text(ACTIVE_NAME).unwrap_or_default();
    devices.iter().find(|d| d.name == stored).or_else(|| devices.first()).map(|d| d.name.clone())
}

fn active_vehicle() -> Option<(u8, u32, u8, bool)> {
    crate::hub::lock().active().map(|v| (v.id, v.link, v.autopilot, crate::hub::Vehicle::armed(v)))
}

fn support_for(autopilot: Option<u8>) -> Support {
    match autopilot {
        Some(crate::modes::AUTOPILOT_ARDUPILOT) => Support { throttle_mode_center_zero: false, negative_thrust: false, ..Support::default() },
        _ => Support::default(),
    }
}

fn polling(enabled_for_vehicle: bool, vehicle: bool, calibrated: bool) -> Polling {
    Polling { vehicle: vehicle && enabled_for_vehicle && calibrated, configuration: false }
}

fn send(vehicle: (u8, u32), outs: Vec<Out>) {
    let (id, link) = vehicle;
    outs.into_iter()
        .filter_map(|out| match out {
            Out::ManualControl { x, y, z, r, buttons, buttons2, enabled_extensions, extensions } => Some(Outbound::JoystickManualControl { target: id, x, y, z, r, buttons, buttons2, enabled_extensions, extensions }),
            Out::RcChannelsOverride { channels } => {
                let first = crate::joystick::RC_OVERRIDE_FIRST_CHANNEL - 1;
                let all: [u16; RC_OVERRIDE_TOTAL] = std::array::from_fn(|i| match i.checked_sub(first).and_then(|at| channels.get(at)) {
                    Some(value) => *value,
                    None if i < RC_LOW_CHANNELS => RC_IGNORE_LOW,
                    None => RC_IGNORE_HIGH,
                });
                Some(Outbound::RcOverride { target: (id, crate::hub::COMP_AUTOPILOT1), channels: all })
            }
            _ => None,
        })
        .filter_map(|outbound| crate::mavout::encode_next(&outbound))
        .for_each(|bytes| {
            crate::linkhost::write(&crate::linkhost::TRANSPORTS, link, &bytes);
        });
}

fn sync_polling(now_ms: u64) {
    let vehicle = active_vehicle();
    let enabled = enabled_vehicles();
    let mut host = host();
    let active = active_name(&host.devices);
    let names: Vec<String> = host.joysticks.keys().cloned().collect();
    let outs: Vec<Out> = names
        .iter()
        .flat_map(|name| {
            let configuring = host.calibration.as_ref().is_some_and(|(calibrating, _)| calibrating == name);
            let wanted = match (&active, vehicle) {
                _ if configuring => Polling { vehicle: false, configuration: true },
                (Some(active), Some((id, ..))) if active == name => polling(enabled.contains(&id.to_string()), true, settings_for(name).calibrated),
                _ => Polling::default(),
            };
            host.joysticks.get_mut(name).map(|j| j.set_polling(wanted, now_ms)).unwrap_or_default()
        })
        .collect();
    drop(host);
    if let Some((id, link, ..)) = vehicle {
        send((id, link), outs);
    }
}

fn devices(text: &str) -> Value {
    let parsed: Vec<Device> = serde_json::from_str::<Vec<Value>>(text)
        .ok()
        .and_then(|args| args.first().and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|d| {
            let count = |key: &str| d[key].as_u64().map(|n| n as usize);
            Some(Device { name: d["name"].as_str().filter(|n| !n.is_empty())?.to_string(), axes: count("axes")?, buttons: count("buttons")?, hats: count("hats").unwrap_or(0) })
        })
        .collect();
    {
        let mut host = host();
        host.joysticks.retain(|name, _| parsed.iter().any(|d| &d.name == name));
        let fresh: Vec<&Device> = parsed.iter().filter(|d| !host.joysticks.contains_key(&d.name)).collect();
        fresh.iter().for_each(|d| {
            let mut model = Joystick::new(d.axes, d.buttons, d.hats);
            let stored = crate::settingsstore::entries_under(&format!("{SETTINGS_PREFIX}/{}", d.name));
            load_axes(&d.name, &mut model, &stored);
            load_buttons(&d.name, &mut model, &stored);
            host.joysticks.insert(d.name.clone(), model);
        });
        if host.calibration.as_ref().is_some_and(|(name, _)| !parsed.iter().any(|d| &d.name == name)) {
            host.calibration = None;
        }
        host.devices = parsed;
    }
    sync_polling(crate::hub::now_ms());
    json!({ "ok": true })
}

fn input(backend: &dyn Backend, text: &str) -> Value {
    let args = serde_json::from_str::<Vec<Value>>(text).unwrap_or_default();
    let Some(name) = args.first().and_then(Value::as_str) else { return json!({ "ok": false, "reason": "joystick.input takes the joystick name" }) };
    let list = |at: usize| args.get(at).and_then(Value::as_array).cloned().unwrap_or_default();
    let axes: Vec<i32> = list(1).iter().filter_map(Value::as_i64).map(|v| v.clamp(i64::from(crate::joystick::AXIS_MIN), i64::from(crate::joystick::AXIS_MAX)) as i32).collect();
    let buttons: Vec<bool> = list(2).iter().map(|v| v.as_bool().unwrap_or(false)).collect();
    let hats: Vec<u8> = list(3).iter().filter_map(Value::as_u64).map(|v| v as u8).collect();
    let vehicle = active_vehicle();
    let support = support_for(vehicle.map(|v| v.2));
    let settings = settings_for(name);
    let now_ms = crate::hub::now_ms();
    let outs = {
        let mut host = host();
        if let Some((_, cal)) = host.calibration.as_mut().filter(|(calibrating, _)| calibrating == name) {
            cal.channel_values(&axes, now_ms);
        }
        host.joysticks.get_mut(name).map(|j| j.on_input(Input { axes: &axes, buttons: &buttons, hats: &hats }, &settings, support, now_ms)).unwrap_or_default()
    };
    if let Some((id, link, _, armed)) = vehicle {
        let actions: Vec<(String, ButtonEvent)> = outs.iter().filter_map(|o| match o {
            Out::Action { action, event } => Some((action.clone(), *event)),
            _ => None,
        }).collect();
        send((id, link), outs);
        actions.iter().for_each(|(action, event)| execute(backend, id, armed, action, *event));
    }
    json!({ "ok": true })
}

fn enable(on: bool) -> Value {
    let Some((id, ..)) = active_vehicle() else { return json!({ "ok": false, "reason": "No vehicle is connected." }) };
    let active = active_name(&host().devices);
    if on && active.as_deref().is_none_or(|name| !settings_for(name).calibrated) {
        return json!({ "ok": false, "reason": "The joystick must be calibrated before it can be enabled." });
    }
    let current = enabled_vehicles();
    let next: Vec<String> = current.iter().filter(|v| **v != id.to_string()).cloned().chain(on.then(|| id.to_string())).collect();
    crate::settingsstore::written(ENABLED_VEHICLES, &next.join(","));
    sync_polling(crate::hub::now_ms());
    json!({ "ok": true })
}

fn set_setting(text: &str) -> Value {
    let args = serde_json::from_str::<Vec<Value>>(text).unwrap_or_default();
    let name = args.first().and_then(Value::as_str).unwrap_or_default();
    let Some(active) = active_name(&host().devices) else { return json!({ "ok": false, "reason": "No joystick is connected." }) };
    let Some(meta) = METADATA.iter().find(|m| m.name == name) else { return json!({ "ok": false, "reason": format!("{name} is not a joystick setting") }) };
    match args.get(1).and_then(|v| coerce(meta, v)) {
        Some(text) => {
            crate::settingsstore::written(&setting_key(&active, name), &text);
            if name == "transmitterMode"
                && let Some(model) = host().joysticks.get_mut(&active)
            {
                model.set_transmitter_mode(transmitter_mode(&active));
            }
            sync_polling(crate::hub::now_ms());
            json!({ "ok": true })
        }
        None => json!({ "ok": false, "reason": format!("That value is out of range for {}.", meta.label) }),
    }
}

fn calibration(text: &str) -> Value {
    let op = serde_json::from_str::<Vec<Value>>(text).ok().and_then(|a| a.first().and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
    if active_vehicle().is_some_and(|v| v.3) {
        return json!({ "ok": false, "reason": "Calibration is not available while the vehicle is armed." });
    }
    let answer = {
        let mut host = host();
        let Some(active) = active_name(&host.devices) else { return json!({ "ok": false, "reason": "No joystick is connected." }) };
        let axes = host.devices.iter().find(|d| d.name == active).map_or(0, |d| d.axes);
        let fresh = host.calibration.as_ref().is_none_or(|(name, _)| *name != active);
        if fresh {
            host.calibration = Some((active.clone(), StickCal::new(crate::stickcal::JOYSTICK, axes, settings_for(&active).optional_enabled)));
        }
        let (_, cal) = host.calibration.as_mut().expect("calibration was just created");
        let outcome = match op.as_str() {
            "next" => cal.next(),
            "oneSided" => {
                cal.one_sided();
                CalOutcome::None
            }
            "cancel" => {
                cal.cancel();
                CalOutcome::None
            }
            _ => CalOutcome::Refused(format!("{op} is not a calibration step")),
        };
        let idle = !cal.calibrating();
        let answer = match outcome {
            CalOutcome::None => json!({ "ok": true }),
            CalOutcome::Refused(reason) => json!({ "ok": false, "reason": reason }),
            CalOutcome::Save(channels) => {
                if let Some(model) = host.joysticks.get_mut(&active) {
                    apply_calibration(&active, model, &channels);
                }
                json!({ "ok": true, "completed": true, "name": active })
            }
        };
        if idle {
            host.calibration = None;
        }
        answer
    };
    sync_polling(crate::hub::now_ms());
    answer
}

pub fn run(backend: &dyn Backend, path: &str, text: &str) -> Value {
    match path {
        DEVICES => devices(text),
        INPUT => input(backend, text),
        SELECT => match serde_json::from_str::<Vec<Value>>(text).ok().and_then(|a| a.first().and_then(Value::as_str).map(str::to_string)) {
            Some(name) => {
                crate::settingsstore::written(ACTIVE_NAME, &name);
                sync_polling(crate::hub::now_ms());
                json!({ "ok": true })
            }
            None => json!({ "ok": false, "reason": "joystick.select takes a joystick name" }),
        },
        ENABLE_JOYSTICK => enable(serde_json::from_str::<Vec<Value>>(text).ok().and_then(|a| a.first().and_then(Value::as_bool)).unwrap_or(false)),
        SET_SETTING => set_setting(text),
        CALIBRATION => calibration(text),
        BUTTON_ACTION => button_change(text, false),
        BUTTON_REPEAT => button_change(text, true),
        _ => json!({ "ok": false, "reason": format!("{path} is not a joystick action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [DEVICES, INPUT, SELECT, ENABLE_JOYSTICK, SET_SETTING, CALIBRATION, BUTTON_ACTION, BUTTON_REPEAT].contains(&path)
}

pub fn tick(now_ms: u64) {
    sync_polling(now_ms);
}

pub fn joystick_state_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = active_vehicle();
    let modes = settable_modes();
    let support = support_for(vehicle.map(|v| v.2));
    let host = host();
    let active = active_name(&host.devices);
    let enabled = vehicle.is_some_and(|(id, ..)| enabled_vehicles().contains(&id.to_string()));
    let now_ms = crate::hub::now_ms();
    let state = active.as_ref().and_then(|name| host.joysticks.get(name).map(|j| j.snapshot(&settings_for(name), support, now_ms)));
    let assigned: Vec<String> = active.as_ref().and_then(|name| host.joysticks.get(name)).map(|j| (0..j.total_button_count()).filter_map(|b| j.binding(b).map(|binding| binding.action.clone())).collect()).unwrap_or_default();
    let assignable: Vec<Value> = assignable_actions(&modes, &assigned).into_iter().map(|(action, repeat)| json!({ "action": action, "canRepeat": repeat })).collect();
    json!({
        "kind": "object",
        "class": "Joystick",
        "available": crate::vehiclefacade::switched_on(),
        "names": host.devices.iter().map(|d| d.name.clone()).collect::<Vec<_>>(),
        "active": active,
        "vehicle": vehicle.is_some(),
        "armed": vehicle.is_some_and(|v| v.3),
        "enabled": enabled,
        "calibrated": active.as_ref().is_some_and(|name| settings_for(name).calibrated),
        "settings": active.as_ref().map(|name| METADATA.iter().map(|m| json!({
            "name": m.name,
            "type": m.kind,
            "label": m.label,
            "units": m.units,
            "min": m.min,
            "max": m.max,
            "value": setting_value(name, &m.name),
        })).collect::<Vec<_>>()).unwrap_or_default(),
        "state": state,
        "calibration": host.calibration.as_ref().filter(|(name, _)| Some(name) == active.as_ref()).map(|(_, cal)| cal.json()),
        "transmitterMode": active.as_ref().map(|name| transmitter_mode(name)),
        "assignableActions": assignable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::joystick::{Function, OPTIONAL};

    #[test]
    fn the_settings_metadata_comes_from_qgcs_group_with_its_defaults() {
        let names: Vec<&str> = METADATA.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"axisFrequencyHz") && names.contains(&"enableAdditionalAxis6"));
        let deadband = METADATA.iter().find(|m| m.name == "useDeadband").unwrap();
        assert_eq!(deadband.default, json!(true), "QGC turns the deadband on by default");
        let rate = METADATA.iter().find(|m| m.name == "axisFrequencyHz").unwrap();
        assert_eq!((coerce(rate, &json!(30.0)), coerce(rate, &json!(500.0))), (Some("30".to_string()), None));
        assert_eq!(OPTIONAL_SETTINGS.len(), OPTIONAL.len());
    }

    #[test]
    fn an_unknown_stored_name_falls_back_to_the_first_device() {
        let devices = vec![Device { name: "Pad".into(), axes: 4, buttons: 10, hats: 1 }];
        assert_eq!(active_name(&devices), Some("Pad".into()));
        assert_eq!(active_name(&[]), None);
        assert_eq!(polling(true, true, false), Polling::default(), "an uncalibrated stick never commands");
        assert_eq!(polling(true, true, true), Polling { vehicle: true, configuration: false });
    }

    #[test]
    fn axes_round_trip_through_qgcs_axis_settings_array_in_mode_two() {
        let mut model = Joystick::new(4, 0, 0);
        let calibration = AxisCalibration { min: -30000, max: 31000, center: 50, deadband: 400, reversed: true };
        assert!(model.set_calibration(1, calibration));
        assert!(model.set_axis_function(Function::Throttle, 1));
        let entries = axis_entries("Pad", &model);
        assert_eq!(entries.get("JoystickSettingsV2/Pad/JoystickAxisSettingsArray/1/function"), Some(&Setting::Text("3".into())), "QGC stores throttleFunction as 3");
        assert_eq!(entries.get("JoystickSettingsV2/Pad/JoystickAxisSettingsArray/1/reversed"), Some(&Setting::Text("true".into())));
        let mut restored = Joystick::new(4, 0, 0);
        load_axes("Pad", &mut restored, &entries);
        assert_eq!((restored.axis_for(Function::Throttle), restored.calibration(1)), (Some(1), Some(calibration)));
    }

    #[test]
    fn buttons_round_trip_and_the_action_list_follows_joystick_cc() {
        let mut model = Joystick::new(4, 6, 0);
        assert!(model.set_button_action(2, Some("Step Zoom In")));
        assert!(model.set_button_repeat(2, true, false));
        let entries = button_entries("Pad", &model);
        assert_eq!(entries.get("JoystickSettingsV2/Pad/JoystickButtonActionSettingsArray/2/actionName"), Some(&Setting::Text("Step Zoom In".into())));
        let mut restored = Joystick::new(4, 6, 0);
        load_buttons("Pad", &mut restored, &entries);
        assert_eq!(restored.binding(2).map(|b| (b.action.clone(), b.repeat)), Some(("Step Zoom In".to_string(), true)));
        let list = assignable_actions(&["Loiter".to_string()], &["Old Mode".to_string()]);
        let names: Vec<&str> = list.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names[..6], ["No Action", "Arm", "Disarm", "Toggle Arm", "Loiter", "VTOL: Fixed Wing"]);
        assert_eq!(names.last(), Some(&"Old Mode"), "an assignment that is not available right now stays listed");
    }
}
