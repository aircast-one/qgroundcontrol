use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{Value, json};

use crate::joystick::{AXIS_MAX, AXIS_MIN};
use crate::joystickhost::{Device, DeviceDetails};
use crate::seriallink::{Event, SerialConfig, SerialLink};
use crate::umbus::{CHANNEL_CENTER, Decoder, APP_HEARTBEAT, APP_KEEPALIVE, MCU_POLL, Sticks};

pub const STICKS_NAME: &str = "Built-in sticks";
pub const STICKS_STALE_AFTER_MS: u64 = 500;
const PUMP_EVERY: Duration = Duration::from_millis(100);
const GIMBAL_SCALE: i32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Gimbal(usize),
    Channel(usize),
}

pub const AXES: [(Source, &str); 13] = [
    (Source::Gimbal(0), "Left stick, left-right"),
    (Source::Gimbal(1), "Right stick, up-down"),
    (Source::Gimbal(2), "Left stick, up-down"),
    (Source::Gimbal(3), "Right stick, left-right"),
    (Source::Channel(6), "S1"),
    (Source::Channel(7), "S2"),
    (Source::Channel(14), "SA"),
    (Source::Channel(15), "SB"),
    (Source::Channel(16), "SC"),
    (Source::Channel(17), "SD"),
    (Source::Channel(19), "SF"),
    (Source::Channel(29), "6-position switch"),
    (Source::Channel(30), "SE"),
];

pub fn axes(sticks: &Sticks) -> Vec<i32> {
    AXES.iter()
        .map(|(source, _)| match source {
            Source::Gimbal(at) => (i32::from(sticks.gimbals[*at]) * GIMBAL_SCALE).clamp(AXIS_MIN, AXIS_MAX),
            Source::Channel(at) => i32::from(sticks.channels[*at]) - i32::from(CHANNEL_CENTER),
        })
        .collect()
}

pub fn device() -> Device {
    Device { name: STICKS_NAME.to_string(), axes: AXES.len(), buttons: 0, hats: 0, gamepad: false, details: DeviceDetails::default() }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Off,
    Busy,
    Failed(String),
    Lost(String),
    Live,
}

impl Status {
    fn id(&self) -> &'static str {
        match self {
            Status::Off => "off",
            Status::Busy => "busy",
            Status::Failed(_) => "failed",
            Status::Lost(_) => "lost",
            Status::Live => "live",
        }
    }

    fn reason(&self) -> Option<String> {
        match self {
            Status::Off | Status::Live => None,
            Status::Busy => Some("RadioMaster's radio app is using the sticks.".to_string()),
            Status::Failed(why) => Some(format!("The built-in sticks didn't open: {why}")),
            Status::Lost(why) => Some(format!("The built-in sticks stopped: {why}")),
        }
    }
}

pub fn refusal(error: &str) -> Status {
    match error.to_lowercase().contains("busy") {
        true => Status::Busy,
        false => Status::Failed(error.to_string()),
    }
}

struct Running {
    link: Arc<SerialLink>,
    stop: Arc<AtomicBool>,
    pump: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct State {
    running: Option<Running>,
    status: Status,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(Mutex::default);
static LAST_FRAME_MS: AtomicU64 = AtomicU64::new(0);

fn state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn reader(mut deliver: impl FnMut(Vec<i32>) + Send + 'static, mut gone: impl FnMut(String) + Send + 'static) -> impl FnMut(Event) + Send + 'static {
    let mut decoder = Decoder::default();
    move |event| match event {
        Event::Bytes(bytes) => decoder.feed(&bytes).iter().filter_map(|frame| crate::umbus::sticks(frame)).for_each(|sticks| deliver(axes(&sticks))),
        Event::Disconnected(why) => gone(why),
    }
}

pub fn spawn_pump(link: Arc<SerialLink>, stop: Arc<AtomicBool>) -> Option<JoinHandle<()>> {
    let batch: Vec<u8> = [MCU_POLL.as_slice(), APP_HEARTBEAT.as_slice(), APP_KEEPALIVE.as_slice()].concat();
    std::thread::Builder::new()
        .name("qgc-sticks-pump".into())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let _ = link.write(&batch);
                std::thread::sleep(PUMP_EVERY);
            }
        })
        .ok()
}

fn delivered(axes: Vec<i32>) {
    LAST_FRAME_MS.store(crate::hub::now_ms(), Ordering::Relaxed);
    crate::joystickhost::core_input(STICKS_NAME, &axes);
}

fn lost(why: String) {
    {
        let mut state = state();
        if let Some(running) = state.running.as_ref() {
            running.stop.store(true, Ordering::Relaxed);
        }
        state.status = Status::Lost(why);
    }
    crate::joystickhost::set_core_device(None);
}

pub fn set(on: bool) -> Result<(), String> {
    match on {
        true => start(),
        false => {
            stop();
            Ok(())
        }
    }
}

fn start() -> Result<(), String> {
    let sticks = crate::hostdevice::built_in_sticks().ok_or_else(|| "This controller has no built-in sticks.".to_string())?;
    if state().status == Status::Live {
        return Ok(());
    }
    stop();
    let config = SerialConfig { port_name: sticks.port.to_string(), baud: sticks.baud, data_bits: 8, parity: 0, stop_bits: 1, flow_control: 0, usb_direct: false };
    match SerialLink::open(&config, reader(delivered, lost)) {
        Ok(link) => {
            let link = Arc::new(link);
            let stop = Arc::new(AtomicBool::new(false));
            let pump = spawn_pump(Arc::clone(&link), Arc::clone(&stop));
            *state() = State { running: Some(Running { link, stop, pump }), status: Status::Live };
            crate::joystickhost::set_core_device(Some(device()));
            Ok(())
        }
        Err(error) => {
            let status = refusal(&error.to_string());
            let reason = status.reason().unwrap_or_default();
            state().status = status;
            Err(reason)
        }
    }
}

fn stop() {
    let running = state().running.take();
    if let Some(running) = running {
        running.stop.store(true, Ordering::Relaxed);
        if let Some(pump) = running.pump {
            let _ = pump.join();
        }
        running.link.close();
    }
    state().status = Status::Off;
    crate::joystickhost::set_core_device(None);
}

pub fn status_json(now_ms: u64) -> Value {
    let available = crate::hostdevice::built_in_sticks();
    let status = state().status.clone();
    let last = LAST_FRAME_MS.load(Ordering::Relaxed);
    json!({
        "available": available.is_some(),
        "status": status.id(),
        "reason": status.reason(),
        "owner": available.map(|sticks| sticks.owner),
        "receiving": status == Status::Live && last > 0 && now_ms.saturating_sub(last) <= STICKS_STALE_AFTER_MS,
        "axes": AXES.iter().map(|(_, label)| *label).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use serialport::{SerialPort, TTYPort};
    #[cfg(unix)]
    use std::io::{Read, Write};
    #[cfg(unix)]
    use std::sync::mpsc;

    const IDLE_APP_CLOSED: &str = "a657100204010300ffff36fe01000cfef4010080008000800080008000800cfef401008000800080008000800080f401000000000cfe0cfe0cfe0080008000800080008000800080008000806400000000000800a401cb";

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len()).step_by(2).map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap()).collect()
    }

    #[test]
    fn the_ax12_controls_become_joystick_axes_over_the_full_axis_range() {
        let idle = crate::umbus::sticks(&hex(IDLE_APP_CLOSED)).unwrap();
        let values = axes(&idle);
        assert_eq!(values.len(), AXES.len());
        assert_eq!(values[..4], [3 * 64, -64, -458 * 64, 64], "gimbals swing about ±500, scaled so calibration has real travel to measure");
        assert_eq!((values[4], values[5]), (65036 - 32768, 500 - 32768), "S1 and S2 are pots centred on 0x8000");
        assert_eq!((values[6], values[9]), (500 - 32768, 65036 - 32768), "SA and SD sit at opposite ends of their travel");
        assert!(values.iter().all(|value| (AXIS_MIN..=AXIS_MAX).contains(value)));
    }

    #[test]
    fn an_ax12_frame_flies_through_the_joystick_model_with_its_switch_on_an_rc_channel() {
        use crate::joystick::{AdditionalAxes, AxisCalibration, Function, Input, Joystick, Out, Polling, Settings, Support};
        let gimbal = AxisCalibration { min: -500 * 64, max: 500 * 64, center: 0, deadband: 0, reversed: false };
        let switch = AxisCalibration { min: 500 - 32768, max: 65036 - 32768, center: 500 - 32768, deadband: 0, reversed: false };
        let mut stick = Joystick::new(AXES.len(), 0, 0);
        [(Function::Yaw, 0, gimbal), (Function::Pitch, 1, gimbal), (Function::Throttle, 2, gimbal), (Function::Roll, 3, gimbal), (Function::Additional1, 6, switch)].iter().for_each(|(function, axis, calibration)| {
            assert!(stick.set_axis_function(*function, *axis));
            assert!(stick.set_calibration(*axis, *calibration));
        });
        let mut settings = Settings { calibrated: true, use_deadband: false, additional_axes: AdditionalAxes::RcChannelsOverride, ..Settings::default() };
        settings.optional_enabled[Function::Additional1.extension_bit().unwrap()] = true;
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let frame = crate::umbus::sticks(&hex(IDLE_APP_CLOSED)).unwrap();
        let out = stick.on_input(Input { axes: &axes(&frame), buttons: &[], hats: &[] }, &settings, Support::default(), 41);
        let control = out.iter().find_map(|o| match o {
            Out::ManualControl { z, .. } => Some(*z),
            _ => None,
        });
        assert_eq!(control, Some(41), "throttle at -458 of a -500..500 stick is about 4% up, so the aircraft gets the stick where the pilot left it");
        let channels = out.iter().find_map(|o| match o {
            Out::RcChannelsOverride { channels } => Some(*channels),
            _ => None,
        });
        assert_eq!(channels.map(|c| c[4]), Some(1000), "SA at its low end drives channel 5 low, the way RadioMaster's own mixer would send it");
    }

    #[test]
    fn a_port_held_by_radiomasters_app_is_reported_as_busy_not_broken() {
        assert_eq!(refusal("/dev/ttyS0: Device or resource busy"), Status::Busy, "RadioMaster's app takes an exclusive TIOCEXCL lock on ttyS0");
        assert_eq!(refusal("Permission denied").id(), "failed");
        assert_eq!(set(true), Err("This controller has no built-in sticks.".to_string()), "a phone or tablet never opens ttyS0");
        assert_eq!(status_json(0)["available"], false);
    }

    #[cfg(unix)]
    #[test]
    fn the_reader_decodes_the_mcu_stream_and_the_pump_keeps_it_flowing() {
        let (mut mcu, app) = TTYPort::pair().expect("a pty pair stands in for ttyS0");
        mcu.set_timeout(Duration::from_secs(2)).unwrap();
        let (frames, received) = mpsc::channel();
        let (gone_tx, gone) = mpsc::channel();
        let link = Arc::new(SerialLink::from_port(Box::new(app), reader(move |axes| frames.send(axes).unwrap(), move |why| gone_tx.send(why).unwrap())).unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let pump = spawn_pump(Arc::clone(&link), Arc::clone(&stop)).unwrap();

        let mut asked = vec![0u8; MCU_POLL.len() + APP_HEARTBEAT.len() + APP_KEEPALIVE.len()];
        mcu.read_exact(&mut asked).unwrap();
        assert_eq!(asked, [MCU_POLL.as_slice(), APP_HEARTBEAT.as_slice(), APP_KEEPALIVE.as_slice()].concat(), "the MCU only streams at 25 Hz while it hears RadioMaster's poll, heartbeat and keep-alive");

        let frame = hex(IDLE_APP_CLOSED);
        mcu.write_all(&frame[..40]).unwrap();
        mcu.write_all(&frame[40..]).unwrap();
        let axes = received.recv_timeout(Duration::from_secs(2)).expect("a frame split across reads still arrives");
        assert_eq!(axes[2], -458 * 64);

        stop.store(true, Ordering::Relaxed);
        pump.join().unwrap();
        link.close();
        assert!(gone.try_recv().is_err(), "closing on purpose is not reported as a lost port");
    }
}
