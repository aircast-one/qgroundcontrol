use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::gpsrtk::{Driver, Fault, INITIAL_BAUD, Out, RECEIVE_TIMEOUT_MS, Session, Settings};
use crate::router::Backend;
use crate::ubxbase::{Event, Transport, UbxBase};

const CONFIGURE_RETRY_MS: u64 = 500;
#[cfg(target_os = "android")]
const RTK_SERIAL_ID: u32 = 0xfff0_0001;

static SESSION: LazyLock<Mutex<Session>> = LazyLock::new(|| Mutex::new(Session::default()));
static STOP: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

fn session() -> MutexGuard<'static, Session> {
    SESSION.lock().unwrap_or_else(PoisonError::into_inner)
}

fn setting(name: &str) -> Value {
    crate::settingsstore::raw_setting(&format!("{}.{name}", crate::gpsrtk::SETTINGS_GROUP)).unwrap_or(Value::Null)
}

fn settings() -> Settings {
    let number = |name: &str| setting(name).as_f64().unwrap_or(0.0);
    Settings {
        survey_in_accuracy_m: number("surveyInAccuracyLimit"),
        survey_in_duration_s: number("surveyInMinObservationDuration") as u32,
        use_fixed_base: setting("useFixedBasePosition").as_bool().unwrap_or(false),
        fixed_latitude: number("fixedBasePositionLatitude"),
        fixed_longitude: number("fixedBasePositionLongitude"),
        fixed_altitude_m: number("fixedBasePositionAltitude") as f32,
        fixed_accuracy_m: number("fixedBasePositionAccuracy") as f32,
    }
}

enum Incoming {
    Bytes(Vec<u8>),
    Closed,
}

trait Port: Send {
    fn write(&self, bytes: &[u8]) -> bool;
    fn close(&self);
}

#[cfg(target_os = "android")]
impl Port for crate::platformserial::PlatformSerial {
    fn write(&self, bytes: &[u8]) -> bool {
        crate::platformserial::PlatformSerial::write(self, bytes)
    }

    fn close(&self) {
        crate::platformserial::PlatformSerial::close(self)
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl Port for crate::seriallink::SerialLink {
    fn write(&self, bytes: &[u8]) -> bool {
        crate::seriallink::SerialLink::write(self, bytes).is_ok()
    }

    fn close(&self) {
        crate::seriallink::SerialLink::close(self)
    }
}

#[cfg(target_os = "android")]
fn open_port(name: &str, baud: u32, sink: Sender<Incoming>) -> Result<Box<dyn Port>, String> {
    let sink = Mutex::new(sink);
    crate::platformserial::PlatformSerial::open(RTK_SERIAL_ID, name, baud, 8, 1, 0, move |event| {
        let incoming = match event {
            crate::platformserial::Event::Bytes(bytes) => Incoming::Bytes(bytes),
            crate::platformserial::Event::Disconnected(_) => Incoming::Closed,
        };
        let _ = sink.lock().unwrap_or_else(PoisonError::into_inner).send(incoming);
    })
    .map(|port| Box::new(port) as Box<dyn Port>)
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn open_port(name: &str, baud: u32, sink: Sender<Incoming>) -> Result<Box<dyn Port>, String> {
    let config = crate::seriallink::SerialConfig { port_name: name.to_string(), baud, data_bits: 8, parity: 0, stop_bits: 1, flow_control: 0, usb_direct: false };
    crate::seriallink::SerialLink::open(&config, move |event| {
        let _ = sink.send(match event {
            crate::seriallink::Event::Bytes(bytes) => Incoming::Bytes(bytes),
            crate::seriallink::Event::Disconnected(_) => Incoming::Closed,
        });
    })
    .map(|link| Box::new(link) as Box<dyn Port>)
    .map_err(|error| error.to_string())
}

#[cfg(target_os = "ios")]
fn open_port(name: &str, _baud: u32, _sink: Sender<Incoming>) -> Result<Box<dyn Port>, String> {
    Err(format!("{name}: this build has no serial ports"))
}

struct SerialTransport {
    name: String,
    port: Box<dyn Port>,
    incoming: Receiver<Incoming>,
    started: Instant,
}

impl SerialTransport {
    fn open(name: &str, baud: u32) -> Result<SerialTransport, String> {
        let (sink, incoming) = channel();
        let port = open_port(name, baud, sink)?;
        Ok(SerialTransport { name: name.to_string(), port, incoming, started: Instant::now() })
    }
}

impl Drop for SerialTransport {
    fn drop(&mut self) {
        self.port.close();
    }
}

impl Transport for SerialTransport {
    fn read(&mut self, timeout_ms: u64) -> Option<Vec<u8>> {
        match self.incoming.recv_timeout(Duration::from_millis(timeout_ms)) {
            Ok(Incoming::Bytes(bytes)) => Some(bytes),
            Err(RecvTimeoutError::Timeout) => Some(Vec::new()),
            Ok(Incoming::Closed) | Err(RecvTimeoutError::Disconnected) => None,
        }
    }

    fn write(&mut self, bytes: &[u8]) -> bool {
        self.port.write(bytes)
    }

    fn set_baud(&mut self, baud: u32) -> bool {
        self.port.close();
        let (sink, incoming) = channel();
        match open_port(&self.name, baud, sink) {
            Ok(port) => {
                self.port = port;
                self.incoming = incoming;
                true
            }
            Err(_) => false,
        }
    }

    fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }
}

fn apply(event: Event, now_ms: u64) {
    match event {
        Event::SurveyIn(status) => {
            session().survey_in_status(status.duration_s as f32, status.mean_accuracy_mm as f32, status.latitude, status.longitude, status.altitude_m, status.flags, now_ms)
        }
        Event::Satellites { count, list } => session().satellite_info(count, &list, now_ms),
        Event::Rtcm(bytes) => {
            session().rtcm(&bytes, now_ms);
            crate::ntrip::inject(&bytes);
        }
    }
}

fn stream<T: Transport>(base: &mut UbxBase<T>, stop: &AtomicBool) -> Result<(), Fault> {
    while !stop.load(Ordering::Relaxed) {
        let handled = base.receive(RECEIVE_TIMEOUT_MS as u64).ok_or(Fault::SerialError)?;
        let events = base.take_events();
        let progress = handled || !events.is_empty();
        let now = crate::hub::now_ms();
        events.into_iter().for_each(|event| apply(event, now));
        if session().received(i32::from(progress)).contains(&Out::RestartDriver) {
            return Ok(());
        }
    }
    Ok(())
}

pub fn run<T: Transport>(transport: T, stop: &AtomicBool) {
    let configure = session().serial_opened().into_iter().find_map(|out| match out {
        Out::ConfigureDriver { driver, plan, .. } => Some((driver, plan)),
        _ => None,
    });
    let Some((Driver::UBlox, plan)) = configure else {
        session().serial_failed(Fault::ConfigureFailed);
        return;
    };
    let mut base = UbxBase::new(transport, plan);
    while !stop.load(Ordering::Relaxed) {
        if !base.configure() {
            if base.transport_lost() {
                session().serial_failed(Fault::SerialError);
                return;
            }
            if !stop.load(Ordering::Relaxed) {
                session().driver_configure_failed();
                std::thread::sleep(Duration::from_millis(CONFIGURE_RETRY_MS));
            }
            continue;
        }
        session().driver_configured();
        if let Err(fault) = stream(&mut base, stop) {
            session().serial_failed(fault);
            return;
        }
    }
}

pub fn connect(port: &str, name: &str) {
    disconnect();
    let opening = session().connect(port, name, settings());
    if !opening.iter().any(|out| matches!(out, Out::OpenSerial { .. })) {
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    *STOP.lock().unwrap_or_else(PoisonError::into_inner) = Some(stop.clone());
    let port = port.to_string();
    log::info!("RTK base connecting on {port} as {name}");
    let _ = std::thread::Builder::new().name("groundstation-rtk-base".to_string()).spawn(move || match SerialTransport::open(&port, INITIAL_BAUD) {
        Ok(transport) => run(transport, &stop),
        Err(reason) => {
            log::warn!("RTK base {reason}");
            session().serial_failed(Fault::DeviceMissing);
        }
    });
}

pub fn disconnect() {
    if let Some(stop) = STOP.lock().unwrap_or_else(PoisonError::into_inner).take() {
        stop.store(true, Ordering::Relaxed);
    }
    session().disconnect();
}

pub fn rtk_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    session().facts(crate::hub::now_ms())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpsrtk::Link;

    struct Silent(u64);

    impl Transport for Silent {
        fn read(&mut self, timeout_ms: u64) -> Option<Vec<u8>> {
            self.0 += timeout_ms.max(1);
            (self.0 < 3_000).then(Vec::new)
        }
        fn write(&mut self, _bytes: &[u8]) -> bool {
            true
        }
        fn set_baud(&mut self, _baud: u32) -> bool {
            true
        }
        fn now_ms(&self) -> u64 {
            self.0
        }
    }

    #[test]
    fn a_receiver_that_never_answers_counts_failed_configures_until_the_port_dies() {
        session().connect("/dev/gps", "u-blox", Settings::default());
        run(Silent(0), &AtomicBool::new(false));
        let after = session();
        assert_eq!(after.link, Link::Failed);
        assert_eq!(after.fault, Some(Fault::SerialError));
        assert_eq!(after.failed_configures, 0, "the port died during the first configure, so it is reported as a serial fault rather than retried");
    }
}
