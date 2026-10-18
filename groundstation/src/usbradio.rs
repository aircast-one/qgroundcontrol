use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::mavout::Outbound;
use crate::transport::LinkId;

const PORT_SETTING: &str = "settings.autoConnectSettings.usbRadioPort";
const USB_RADIO_STANDBY: &str = "settings.autoConnectSettings.usbRadioStandby";
pub const SEND_EVERY_MS: u64 = 40;
pub const STALE_AFTER_MS: u64 = 500;
const REOPEN_AFTER_MS: u64 = 1_000;
pub const ECHO_EVERY_MS: u64 = 250;
const RELEASE_FRAMES: usize = 3;
#[cfg(hosted_serial)]
const SERIAL_ID: u32 = 0xfff0_0004;
const BAUD: u32 = 115_200;
const MAX_LINE_BYTES: usize = 1024;
const VALUES: std::ops::RangeInclusive<usize> = 4..=24;
const OVERRIDE_PWM: std::ops::RangeInclusive<i32> = 800..=2200;

pub type Frame = [u16; crate::joystick::RC_FRAME_CHANNELS];
pub type Target = (u8, LinkId);

pub fn pwm(raw: i32) -> u16 {
    (raw / 2 + 1500).clamp(*OVERRIDE_PWM.start(), *OVERRIDE_PWM.end()) as u16
}

pub fn parse(line: &str) -> Option<Frame> {
    let values: Vec<u16> = line.trim().strip_prefix('$')?.split(',').map(str::trim).filter(|value| !value.is_empty()).map(|value| value.parse::<i32>().ok().map(pwm)).collect::<Option<_>>()?;
    VALUES.contains(&values.len()).then(|| std::array::from_fn(|at| values.get(at).copied().unwrap_or(u16::MAX)))
}

pub fn released() -> Frame {
    std::array::from_fn(|at| crate::joystick::rc_release(at + 1))
}

#[derive(Default)]
pub struct Lines {
    pending: String,
}

impl Lines {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Frame> {
        self.pending.push_str(&String::from_utf8_lossy(bytes));
        let complete = self.pending.rfind('\n').map(|end| self.pending.drain(..=end).collect::<String>()).unwrap_or_default();
        if self.pending.len() > MAX_LINE_BYTES {
            self.pending.clear();
        }
        complete.lines().filter_map(parse).collect()
    }
}

#[derive(Debug, Default)]
pub struct Radio {
    latest: Option<(Frame, u64)>,
    sent_at: Option<u64>,
    holding: Option<Target>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Step {
    pub sends: Vec<(Target, Frame)>,
    pub handed_back: Option<Target>,
}

impl Radio {
    pub fn live(&self, now_ms: u64) -> bool {
        self.latest.is_some_and(|(_, at)| now_ms.saturating_sub(at) <= STALE_AFTER_MS)
    }

    pub fn step(&mut self, now_ms: u64, target: Option<Target>, fresh: Option<Frame>) -> Step {
        if let Some(frame) = fresh {
            self.latest = Some((frame, now_ms));
        }
        let flying = target.filter(|_| self.live(now_ms));
        let handed_back = self.holding.filter(|held| flying != Some(*held));
        let due = fresh.is_some() && self.sent_at.is_none_or(|at| now_ms.saturating_sub(at) >= SEND_EVERY_MS);
        let send = flying.zip(self.latest).filter(|_| due).map(|(to, (frame, _))| (to, frame));
        if send.is_some() {
            self.sent_at = Some(now_ms);
        }
        self.holding = match send {
            Some((to, _)) => Some(to),
            None => self.holding.filter(|_| handed_back.is_none()),
        };
        let releases = handed_back.into_iter().flat_map(|held| std::iter::repeat_n((held, released()), RELEASE_FRAMES));
        Step { sends: releases.chain(send).collect(), handed_back }
    }
}

struct Port {
    write: Box<dyn Fn(&[u8]) + Send>,
    close: Box<dyn Fn() + Send>,
    dead: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flying {
    NoVehicle,
    Vehicle(u8),
    Standby(u8),
    Joystick(u8),
}

pub fn echo(flying: Flying) -> String {
    match flying {
        Flying::NoVehicle => "@N\n".to_string(),
        Flying::Vehicle(id) => format!("@F,{id}\n"),
        Flying::Standby(id) => format!("@S,{id}\n"),
        Flying::Joystick(id) => format!("@J,{id}\n"),
    }
}

fn flying() -> Flying {
    let vehicle = crate::hub::lock().active().filter(|v| !v.on_high_latency_link()).map(|v| v.id);
    match vehicle {
        None => Flying::NoVehicle,
        Some(id) if crate::joystickhost::drives(id) => Flying::Joystick(id),
        Some(id) if standby(id) => Flying::Standby(id),
        Some(id) => Flying::Vehicle(id),
    }
}

static ECHOED_AT: Mutex<Option<u64>> = Mutex::new(None);

pub fn echo_due(last: Option<u64>, now_ms: u64, live: bool) -> bool {
    live && last.is_none_or(|at| now_ms.saturating_sub(at) >= ECHO_EVERY_MS)
}

fn answer(now_ms: u64, live: bool) {
    let due = {
        let mut echoed = ECHOED_AT.lock().unwrap_or_else(PoisonError::into_inner);
        let due = echo_due(*echoed, now_ms, live);
        if due {
            *echoed = Some(now_ms);
        }
        due
    };
    if due {
        let line = echo(flying());
        if let Some(port) = ATTEMPT.lock().unwrap_or_else(PoisonError::into_inner).as_ref().and_then(|attempt| attempt.port.as_ref()) {
            (port.write)(line.as_bytes());
        }
    }
}

impl Drop for Port {
    fn drop(&mut self) {
        (self.close)();
    }
}

struct Attempt {
    wanted: String,
    port: Option<Port>,
    at: u64,
}

static ATTEMPT: Mutex<Option<Attempt>> = Mutex::new(None);
static RADIO: Mutex<Radio> = Mutex::new(Radio { latest: None, sent_at: None, holding: None });

fn radio() -> MutexGuard<'static, Radio> {
    RADIO.lock().unwrap_or_else(PoisonError::into_inner)
}

fn wanted() -> Option<String> {
    crate::settingsstore::raw_setting(PORT_SETTING).and_then(|value| value.as_str().map(str::trim).map(str::to_string)).filter(|port| !port.is_empty())
}

pub fn component(standby: bool) -> u8 {
    match standby {
        true => crate::joystickhost::STANDBY_COMPONENT,
        false => crate::hub::COMP_AUTOPILOT1,
    }
}

fn standby(vehicle: u8) -> bool {
    let on = crate::settingsstore::raw_setting(USB_RADIO_STANDBY).is_some_and(|value| value.as_bool() == Some(true) || value.as_str() == Some("true"));
    on && crate::joystickhost::standby_capable(vehicle)
}

static STANDBY: AtomicBool = AtomicBool::new(false);

fn target() -> Option<Target> {
    let vehicle = crate::hub::lock().active().filter(|v| !v.on_high_latency_link()).map(|v| (v.id, v.link))?;
    (!crate::joystickhost::drives(vehicle.0)).then_some(vehicle)
}

fn why_handed_back(live: bool, vehicle: u8) -> String {
    match (live, crate::joystickhost::drives(vehicle)) {
        (false, _) => format!("The radio on USB stopped sending, so vehicle {vehicle}'s RC channels were handed back."),
        (true, true) => format!("A joystick flies vehicle {vehicle}, so the radio on USB handed its RC channels back."),
        (true, false) => format!("The radio on USB handed vehicle {vehicle}'s RC channels back."),
    }
}

fn advance(fresh: Option<Frame>) {
    let now = crate::hub::now_ms();
    let target = target();
    let (step, live) = {
        let mut radio = radio();
        let step = radio.step(now, target, fresh);
        (step, radio.live(now))
    };
    let standby_now = target.is_some_and(|(id, _)| standby(id));
    let was = STANDBY.swap(standby_now, Ordering::Relaxed);
    let moved = (was != standby_now && live).then_some(()).and(target);
    moved.into_iter().for_each(|(id, link)| {
        let handed: Vec<Outbound> = std::iter::repeat_n(Outbound::RcOverride { target: (id, component(was)), channels: released() }, RELEASE_FRAMES).collect();
        crate::hub::send_for(id, link, &handed);
    });
    step.sends.iter().for_each(|((id, link), channels)| crate::hub::send_for(*id, *link, &[Outbound::RcOverride { target: (*id, component(standby_now)), channels: *channels }]));
    if let Some((vehicle, _)) = step.handed_back {
        crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &why_handed_back(live, vehicle));
    }
    answer(now, live);
}

fn heard(frames: Vec<Frame>) {
    if let Some(frame) = frames.last() {
        advance(Some(*frame));
    }
}

#[cfg(hosted_serial)]
fn open(port: &str) -> Option<Port> {
    let lines = Mutex::new(Lines::default());
    let dead = Arc::new(AtomicBool::new(false));
    let lost = dead.clone();
    let opened = crate::platformserial::PlatformSerial::open(SERIAL_ID, port, BAUD, 8, 1, 0, move |event| match event {
        crate::platformserial::Event::Bytes(bytes) => heard(lines.lock().unwrap_or_else(PoisonError::into_inner).feed(&bytes)),
        crate::platformserial::Event::Disconnected(_) => lost.store(true, Ordering::Relaxed),
    })
    .ok()?;
    let opened = Arc::new(opened);
    let writer = opened.clone();
    Some(Port {
        write: Box::new(move |bytes| {
            writer.write(bytes);
        }),
        close: Box::new(move || opened.close()),
        dead,
    })
}

#[cfg(not(hosted_serial))]
fn open(port: &str) -> Option<Port> {
    let config = crate::seriallink::SerialConfig { port_name: port.to_string(), baud: BAUD, data_bits: 8, parity: 0, stop_bits: 1, flow_control: 0, usb_direct: false };
    let mut lines = Lines::default();
    let dead = Arc::new(AtomicBool::new(false));
    let lost = dead.clone();
    let link = crate::seriallink::SerialLink::open(&config, move |event| match event {
        crate::seriallink::Event::Bytes(bytes) => heard(lines.feed(&bytes)),
        crate::seriallink::Event::Disconnected(_) => lost.store(true, Ordering::Relaxed),
    })
    .ok()?;
    let link = Arc::new(link);
    let writer = link.clone();
    Some(Port { write: Box::new(move |bytes| drop(writer.write(bytes))), close: Box::new(move || link.close()), dead })
}

pub fn maintain() {
    let now = crate::hub::now_ms();
    let wanted = wanted();
    let replaced = {
        let mut current = ATTEMPT.lock().unwrap_or_else(PoisonError::into_inner);
        let alive = |attempt: &Attempt| attempt.port.as_ref().is_some_and(|port| !port.dead.load(Ordering::Relaxed));
        let settled = current.as_ref().is_some_and(|attempt| Some(&attempt.wanted) == wanted.as_ref() && (alive(attempt) || now.saturating_sub(attempt.at) < REOPEN_AFTER_MS));
        match settled || (current.is_none() && wanted.is_none()) {
            true => None,
            false => Some(current.take()),
        }
    };
    if let Some(old) = replaced {
        drop(old);
        let attempt = wanted.map(|wanted| Attempt { port: open(&wanted), wanted, at: now });
        *ATTEMPT.lock().unwrap_or_else(PoisonError::into_inner) = attempt;
    }
    advance(None);
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "$-1024,1024,0,-512,-1024,-1024,0,0,0,0,-1024,0,0,0,0,-160,0,1024,8,-40,-160,0,0,-1024,\r\n";

    fn frame(pairs: &[(usize, u16)]) -> Frame {
        std::array::from_fn(|at| pairs.iter().find(|(channel, _)| *channel == at + 1).map_or(1500, |(_, pwm)| *pwm))
    }

    #[test]
    fn the_r2d2_script_line_reads_as_the_radio_shows_its_channels() {
        let read = parse(LINE).unwrap();
        assert_eq!(read[..5], [988, 2012, 1500, 1244, 988], "raw -1024..1024 is (raw / 2) + 1500, the 988-2012 the radio's own monitor shows");
        assert_eq!((read[10], read[15], read[17]), (988, 1420, 2012), "CH11 skld low, CH16 S2, CH18 camREB high");
        assert_eq!(pwm(1536), 2200, "an extended 150% limit is clamped to what an override may carry");
        assert_eq!(parse("$1500,1500\r\n"), None, "too few channels to be a radio");
        assert_eq!(parse("$0,0,0,x,0\r\n"), None);
        assert_eq!(parse("GPGGA,0,0,0,0"), None);
        assert_eq!(parse("$0,0,0,0").map(|read| read[4]), Some(u16::MAX), "channels the radio did not send are left to the pilot");
    }

    #[test]
    fn in_4g_standby_the_radio_streams_to_the_aircraft_computer_not_the_flight_controller() {
        assert_eq!(component(false), crate::hub::COMP_AUTOPILOT1);
        assert_eq!(component(true), crate::joystickhost::STANDBY_COMPONENT, "ArduPilot processes an override for another component of its own system nowhere, so only aircastd acts on it");
    }

    #[test]
    fn the_radio_is_told_what_its_sticks_are_doing() {
        assert_eq!(echo(Flying::Vehicle(1)), "@F,1\n");
        assert_eq!(echo(Flying::Standby(3)), "@S,3\n", "in 4G standby the sticks go to the aircraft computer");
        assert_eq!(echo(Flying::Joystick(1)), "@J,1\n");
        assert_eq!(echo(Flying::NoVehicle), "@N\n");
        assert!(echo(Flying::Vehicle(1)).lines().all(|line| parse(line).is_none()), "an echo is never read back as stick values");
    }

    #[test]
    fn the_radio_hears_back_a_few_times_a_second_and_only_while_it_streams() {
        assert!(!echo_due(None, 0, false), "a silent radio is not answered");
        assert!(echo_due(None, 1_000, true));
        assert!(!echo_due(Some(1_000), 1_000 + ECHO_EVERY_MS - 1, true));
        assert!(echo_due(Some(1_000), 1_000 + ECHO_EVERY_MS, true));
    }

    #[test]
    fn a_streaming_radio_is_written_its_status_on_its_own_port() {
        let written = Arc::new(Mutex::new(Vec::<u8>::new()));
        let sink = written.clone();
        *ATTEMPT.lock().unwrap() = Some(Attempt {
            wanted: "radio".into(),
            port: Some(Port { write: Box::new(move |bytes| sink.lock().unwrap().extend_from_slice(bytes)), close: Box::new(|| {}), dead: Arc::new(AtomicBool::new(false)) }),
            at: 0,
        });
        *ECHOED_AT.lock().unwrap() = None;
        answer(5_000, false);
        assert!(written.lock().unwrap().is_empty(), "nothing is written to a radio that is not streaming");
        answer(5_000, true);
        assert_eq!(String::from_utf8(written.lock().unwrap().clone()).unwrap(), "@N\n", "with no vehicle the radio is told so");
        *ATTEMPT.lock().unwrap() = None;
    }

    #[test]
    fn lines_split_across_usb_reads_are_joined_and_garbage_is_dropped() {
        let mut lines = Lines::default();
        assert!(lines.feed(&LINE.as_bytes()[..30]).is_empty());
        assert_eq!(lines.feed(&LINE.as_bytes()[30..]).len(), 1);
        assert!(lines.feed(&[b'x'; MAX_LINE_BYTES + 1]).is_empty());
        assert_eq!(lines.feed(format!("\n{LINE}{LINE}").as_bytes()).len(), 2, "an overlong run without a newline is dropped, not glued to the next line");
    }

    #[test]
    fn the_radio_flies_the_vehicle_at_most_every_send_period_and_hands_back_when_it_goes_quiet() {
        let mut radio = Radio::default();
        let vehicle = Some((1, 7));
        let sticks = frame(&[(3, 1100)]);
        assert_eq!(radio.step(0, vehicle, Some(sticks)).sends, vec![((1, 7), sticks)]);
        assert!(radio.step(20, vehicle, Some(sticks)).sends.is_empty(), "a script streaming faster than the send period is thinned");
        assert_eq!(radio.step(SEND_EVERY_MS, vehicle, Some(sticks)).sends.len(), 1);
        assert_eq!(radio.step(SEND_EVERY_MS + STALE_AFTER_MS, vehicle, None), Step::default(), "still fresh at the stale limit");
        let quiet = radio.step(SEND_EVERY_MS + STALE_AFTER_MS + 1, vehicle, None);
        assert_eq!(quiet.handed_back, Some((1, 7)));
        assert_eq!(quiet.sends, vec![((1, 7), released()); RELEASE_FRAMES], "channels go back to the radio link at once instead of after RC_OVERRIDE_TIME");
        assert_eq!(released()[0], 0, "a released low channel is 0");
        assert_eq!(released()[10], u16::MAX - 1, "a released high channel is UINT16_MAX - 1");
        assert_eq!(radio.step(SEND_EVERY_MS + STALE_AFTER_MS + 2, vehicle, None), Step::default(), "handing back happens once");
    }

    #[test]
    fn losing_the_vehicle_or_yielding_to_a_joystick_hands_the_channels_back_and_a_new_vehicle_is_flown() {
        let mut radio = Radio::default();
        let sticks = frame(&[]);
        radio.step(0, Some((1, 7)), Some(sticks));
        let yielded = radio.step(50, None, Some(sticks));
        assert_eq!((yielded.handed_back, yielded.sends.len()), (Some((1, 7)), RELEASE_FRAMES));
        assert!(radio.step(100, None, Some(sticks)).sends.is_empty());
        let other = radio.step(150, Some((2, 8)), Some(sticks));
        assert_eq!((other.handed_back, other.sends), (None, vec![((2, 8), sticks)]));
        let moved = radio.step(200, Some((1, 7)), Some(sticks));
        assert_eq!(moved.handed_back, Some((2, 8)));
        assert_eq!(moved.sends.last(), Some(&((1, 7), sticks)), "the old vehicle is released before the new one is flown");
    }
}
