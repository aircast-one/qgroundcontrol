use std::ffi::{CStr, CString, c_char};
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};

use serde_json::{Value, json};

use crate::packetradio::{ANTENNA_COUNT, Adapter, DEFAULT_KEY_FILE, Key, Out, POLL_INTERVAL_MS, PacketRadio, Settings, Status};

pub trait Radio {
    fn devices(&self) -> Vec<Adapter>;
    fn start(&self, adapter: &str, channel: u8, channel_width: i64, key_path: &str) -> Result<(), String>;
    fn stop(&self);
    fn poll(&self) -> Poll;
    fn adaptive(&self, enabled: bool, tx_power: i64);
    fn rtp_packets(&self) -> i64;
    fn take_codec(&self) -> Option<String>;
    fn take_stopped(&self) -> bool;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Poll {
    pub rssi_raw: [i32; ANTENNA_COUNT],
    pub snr: [i32; ANTENNA_COUNT],
    pub score: [i32; ANTENNA_COUNT],
    pub packets_lost: i32,
}

pub trait Store {
    fn read(&self, key: &str) -> Option<String>;
    fn write(&self, key: &str, value: Option<&str>);
}

const VIDEO_SOURCE: &str = "Video/videoSource";
const VIDEO_UDP_URL: &str = "Video/udpUrl";
const VIDEO_LOW_LATENCY: &str = "Video/lowLatencyMode";
const VIDEO_KEYS: [&str; 3] = [VIDEO_SOURCE, VIDEO_UDP_URL, VIDEO_LOW_LATENCY];

#[derive(Default)]
pub struct Driver {
    pub machine: PacketRadio,
    last: Option<Settings>,
    retry_due: Option<u64>,
    poll_due: Option<u64>,
    saved_video: Option<Vec<(&'static str, Option<String>)>>,
}

fn link_fields(settings: &Settings) -> (u8, i64, &str, bool, &str) {
    (settings.channel, settings.channel_width, &settings.key_file, settings.key_file_exists, &settings.device_name)
}

impl Driver {
    pub fn tick(&mut self, radio: &dyn Radio, store: &dyn Store, settings: &Settings, default_key: Option<&str>, now_ms: u64) {
        let devices = || radio.devices();
        let outs = match self.last.replace(settings.clone()) {
            None => self.machine.on_enabled(settings, &devices()),
            Some(previous) if previous.enabled != settings.enabled => self.machine.on_enabled(settings, &devices()),
            Some(previous) if link_fields(&previous) != link_fields(settings) => self.machine.on_link_settings_changed(settings, &devices()),
            Some(previous) if (previous.alink_enabled, previous.alink_tx_power) != (settings.alink_enabled, settings.alink_tx_power) => self.machine.on_adaptive_changed(settings),
            Some(_) => Vec::new(),
        };
        self.execute(radio, store, settings, default_key, outs, now_ms);
        if self.retry_due.is_some_and(|due| now_ms >= due) {
            self.retry_due = None;
            let outs = self.machine.on_retry(settings, &devices());
            self.execute(radio, store, settings, default_key, outs, now_ms);
        }
        if self.machine.running() {
            if let Some(codec) = radio.take_codec() {
                let outs = self.machine.on_rtp_stream(&codec);
                self.execute(radio, store, settings, default_key, outs, now_ms);
            }
            if radio.take_stopped() {
                let outs = self.machine.on_wifi_stopped(settings, &devices());
                self.execute(radio, store, settings, default_key, outs, now_ms);
            }
        }
        if self.poll_due.is_some_and(|due| now_ms >= due) {
            self.poll_due = Some(now_ms + POLL_INTERVAL_MS);
            self.machine.on_video_packets(radio.rtp_packets());
            let sample = radio.poll();
            let outs = self.machine.on_poll(sample.rssi_raw, sample.snr, sample.score, sample.packets_lost, now_ms);
            self.execute(radio, store, settings, default_key, outs, now_ms);
        }
    }

    pub fn refresh(&mut self, radio: &dyn Radio) {
        let _ = self.machine.on_devices(&radio.devices());
    }

    fn execute(&mut self, radio: &dyn Radio, store: &dyn Store, settings: &Settings, default_key: Option<&str>, outs: Vec<Out>, now_ms: u64) {
        outs.into_iter().for_each(|out| match out {
            Out::Start { adapter, channel, channel_width, key } => {
                let path = match &key {
                    Key::Configured(path) => Some(path.as_str()),
                    Key::BuiltinDefault => default_key,
                };
                let next = match path.map(|path| radio.start(&adapter, channel, channel_width, path)) {
                    None => self.machine.on_key_unavailable(settings),
                    Some(Ok(())) => self.machine.on_started(settings),
                    Some(Err(error)) => self.machine.on_start_failed(&error),
                };
                self.execute(radio, store, settings, default_key, next, now_ms);
            }
            Out::Stop => radio.stop(),
            Out::ArmRetry { after_ms } => self.retry_due = Some(now_ms + after_ms),
            Out::CancelRetry => self.retry_due = None,
            Out::StartPolling { every_ms } => self.poll_due = Some(now_ms + every_ms),
            Out::StopPolling => self.poll_due = None,
            Out::Adaptive { enabled, tx_power } => radio.adaptive(enabled, tx_power),
            Out::ApplyVideo { source, host, port, low_latency, save_previous } => {
                if save_previous || self.saved_video.is_none() {
                    self.saved_video = Some(VIDEO_KEYS.iter().map(|key| (*key, store.read(key))).collect());
                }
                store.write(VIDEO_SOURCE, Some(source));
                store.write(VIDEO_UDP_URL, Some(&format!("{host}:{port}")));
                store.write(VIDEO_LOW_LATENCY, Some(if low_latency { "true" } else { "false" }));
            }
            Out::RestoreVideo => {
                self.saved_video.take().into_iter().flatten().for_each(|(key, value)| store.write(key, value.as_deref()));
            }
            Out::Status(_) | Out::Statistics | Out::Adapters => {}
        });
    }
}

pub fn status_text(status: Status, adapter: Option<&str>, start_error: Option<&str>) -> String {
    let adapter = adapter.unwrap_or_default();
    match status {
        Status::Disabled => "Off".to_string(),
        Status::NoAdapter => "No supported Wi-Fi adapter found".to_string(),
        Status::AdapterUnavailable => match start_error {
            None => format!("{adapter} found but cannot be opened — is another app using it?"),
            Some(error) => format!("{adapter} cannot be started: {error}"),
        },
        Status::InvalidKey => "Key file cannot be read — check the path in Radio settings".to_string(),
        Status::Listening => format!("Listening on {adapter} — no video yet"),
        Status::Receiving => format!("Receiving on {adapter}"),
    }
}

#[repr(C)]
pub struct Native {
    pub devices: extern "C" fn() -> *mut c_char,
    pub start: extern "C" fn(*const c_char, u8, i32, *const c_char, *mut *mut c_char) -> bool,
    pub stop: extern "C" fn(),
    pub poll: extern "C" fn(*mut i32),
    pub adaptive: extern "C" fn(bool, i32),
    pub rtp_packets: extern "C" fn() -> i64,
    pub take_codec: extern "C" fn() -> *mut c_char,
    pub take_stopped: extern "C" fn() -> bool,
    pub free: extern "C" fn(*mut c_char),
}

impl Native {
    fn taken(&self, raw: *mut c_char) -> Option<String> {
        (!raw.is_null()).then(|| {
            let text = unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned();
            (self.free)(raw);
            text
        })
    }
}

impl Radio for Native {
    fn devices(&self) -> Vec<Adapter> {
        let listed: Value = self.taken((self.devices)()).and_then(|text| serde_json::from_str(&text).ok()).unwrap_or(Value::Null);
        listed
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|device| Some(Adapter { display_name: device.get("name")?.as_str()?.to_string(), known: device.get("known").and_then(Value::as_bool).unwrap_or(false) }))
            .collect()
    }

    fn start(&self, adapter: &str, channel: u8, channel_width: i64, key_path: &str) -> Result<(), String> {
        let (Ok(adapter), Ok(key)) = (CString::new(adapter), CString::new(key_path)) else {
            return Err("the adapter name or key path holds a NUL byte".to_string());
        };
        let mut error: *mut c_char = std::ptr::null_mut();
        let started = (self.start)(adapter.as_ptr(), channel, i32::try_from(channel_width).unwrap_or(20), key.as_ptr(), &mut error);
        let reason = self.taken(error).unwrap_or_default();
        if started { Ok(()) } else { Err(reason) }
    }

    fn stop(&self) {
        (self.stop)()
    }

    fn poll(&self) -> Poll {
        let mut values = [0i32; 3 * ANTENNA_COUNT + 1];
        (self.poll)(values.as_mut_ptr());
        let antennas = |at: usize| std::array::from_fn(|index| values[at + index]);
        Poll { rssi_raw: antennas(0), snr: antennas(ANTENNA_COUNT), score: antennas(2 * ANTENNA_COUNT), packets_lost: values[3 * ANTENNA_COUNT] }
    }

    fn adaptive(&self, enabled: bool, tx_power: i64) {
        (self.adaptive)(enabled, i32::try_from(tx_power).unwrap_or_default())
    }

    fn rtp_packets(&self) -> i64 {
        (self.rtp_packets)()
    }

    fn take_codec(&self) -> Option<String> {
        self.taken((self.take_codec)())
    }

    fn take_stopped(&self) -> bool {
        (self.take_stopped)()
    }
}

pub struct WithoutUsb;

impl Radio for WithoutUsb {
    fn devices(&self) -> Vec<Adapter> {
        Vec::new()
    }
    fn start(&self, _adapter: &str, _channel: u8, _channel_width: i64, _key_path: &str) -> Result<(), String> {
        Err("this platform gives the radio no USB access".to_string())
    }
    fn stop(&self) {}
    fn poll(&self) -> Poll {
        Poll::default()
    }
    fn adaptive(&self, _enabled: bool, _tx_power: i64) {}
    fn rtp_packets(&self) -> i64 {
        0
    }
    fn take_codec(&self) -> Option<String> {
        None
    }
    fn take_stopped(&self) -> bool {
        false
    }
}

struct SettingsStore;

impl Store for SettingsStore {
    fn read(&self, key: &str) -> Option<String> {
        crate::settingsstore::stored_text(key)
    }

    fn write(&self, key: &str, value: Option<&str>) {
        match value {
            Some(text) => crate::settingsstore::written(key, text),
            None => crate::settingsstore::forgotten(key),
        }
    }
}

static NATIVE: OnceLock<Native> = OnceLock::new();
static DRIVER: LazyLock<Mutex<Driver>> = LazyLock::new(|| Mutex::new(Driver::default()));

fn driver() -> MutexGuard<'static, Driver> {
    DRIVER.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn register(native: Native) -> bool {
    NATIVE.set(native).is_ok()
}

fn radio() -> Option<&'static dyn Radio> {
    match NATIVE.get() {
        Some(native) => Some(native),
        None if cfg!(target_os = "android") => Some(&WithoutUsb),
        None => None,
    }
}

fn setting(name: &str) -> Value {
    crate::settingsstore::raw_setting(&format!("settings.packetRadioSettings.{name}")).unwrap_or(Value::Null)
}

fn default_key_path() -> Option<String> {
    let path = crate::settingsstore::folder()?.join(DEFAULT_KEY_FILE);
    let present = path.exists() || std::fs::write(&path, crate::packetradio::default_key_bytes()).is_ok();
    present.then(|| path.to_string_lossy().into_owned())
}

fn current_settings(default_key: Option<&str>) -> Settings {
    let key_file = setting("keyFile").as_str().unwrap_or_default().to_string();
    Settings {
        enabled: setting("enabled").as_bool().unwrap_or(false),
        channel: setting("channel").as_u64().and_then(|c| u8::try_from(c).ok()).unwrap_or_default(),
        channel_width: setting("channelWidth").as_i64().unwrap_or(20),
        key_file_exists: !key_file.trim().is_empty() && std::path::Path::new(key_file.trim()).exists(),
        key_file,
        default_key_writable: default_key.is_some(),
        device_name: setting("deviceName").as_str().unwrap_or_default().to_string(),
        alink_enabled: setting("alinkEnabled").as_bool().unwrap_or(false),
        alink_tx_power: setting("alinkTxPower").as_i64().unwrap_or_default(),
    }
}

pub fn tick(now_ms: u64) {
    let Some(radio) = radio() else { return };
    let default_key = default_key_path();
    let settings = current_settings(default_key.as_deref());
    driver().tick(radio, &SettingsStore, &settings, default_key.as_deref(), now_ms);
}

pub fn refresh() -> Option<Value> {
    let radio = radio()?;
    driver().refresh(radio);
    Some(json!({ "ok": true }))
}

pub fn snapshot(now_ms: u64) -> Option<Value> {
    radio()?;
    let held = driver();
    let mut view = held.machine.snapshot(now_ms);
    view["statusText"] = json!(status_text(held.machine.status(), held.machine.adapter(), held.machine.start_error()));
    Some(view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Fake {
        devices: Vec<Adapter>,
        refuse: Option<String>,
        calls: RefCell<Vec<String>>,
        rtp: RefCell<i64>,
        codec: RefCell<Option<String>>,
    }

    impl Radio for Fake {
        fn devices(&self) -> Vec<Adapter> {
            self.devices.clone()
        }
        fn start(&self, adapter: &str, channel: u8, channel_width: i64, key_path: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("start {adapter} {channel} {channel_width} {key_path}"));
            self.refuse.clone().map_or(Ok(()), Err)
        }
        fn stop(&self) {
            self.calls.borrow_mut().push("stop".into());
        }
        fn poll(&self) -> Poll {
            Poll { rssi_raw: [60, 0], snr: [20, 0], score: [1500, 0], packets_lost: 0 }
        }
        fn adaptive(&self, enabled: bool, tx_power: i64) {
            self.calls.borrow_mut().push(format!("adaptive {enabled} {tx_power}"));
        }
        fn rtp_packets(&self) -> i64 {
            *self.rtp.borrow()
        }
        fn take_codec(&self) -> Option<String> {
            self.codec.borrow_mut().take()
        }
        fn take_stopped(&self) -> bool {
            false
        }
    }

    #[derive(Default)]
    struct Memory(RefCell<BTreeMap<String, String>>);

    impl Store for Memory {
        fn read(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }
        fn write(&self, key: &str, value: Option<&str>) {
            match value {
                Some(text) => self.0.borrow_mut().insert(key.to_string(), text.to_string()),
                None => self.0.borrow_mut().remove(key),
            };
        }
    }

    fn enabled() -> Settings {
        Settings { enabled: true, channel: 161, channel_width: 20, default_key_writable: true, ..Settings::default() }
    }

    #[test]
    fn a_radio_turned_on_starts_polls_takes_over_video_and_gives_it_back_when_turned_off() {
        let radio = Fake { devices: vec![Adapter { display_name: "RTL8812AU [1:2]".into(), known: true }], ..Fake::default() };
        let store = Memory::default();
        store.write(VIDEO_SOURCE, Some("RTSP Video Stream"));
        let mut driver = Driver::default();
        driver.tick(&radio, &store, &enabled(), Some("/k/default.key"), 0);
        assert_eq!(driver.machine.status(), Status::Listening);
        assert_eq!(radio.calls.borrow()[..2], ["start RTL8812AU [1:2] 161 20 /k/default.key".to_string(), "adaptive false 0".to_string()]);
        *radio.rtp.borrow_mut() = 5;
        *radio.codec.borrow_mut() = Some("H265".into());
        driver.tick(&radio, &store, &enabled(), Some("/k/default.key"), POLL_INTERVAL_MS);
        assert_eq!(driver.machine.status(), Status::Receiving);
        assert_eq!(store.read(VIDEO_SOURCE).as_deref(), Some(crate::packetradio::video_source("H265")));
        assert_eq!(store.read(VIDEO_UDP_URL).as_deref(), Some("0.0.0.0:5600"));
        driver.tick(&radio, &store, &Settings { enabled: false, ..enabled() }, Some("/k/default.key"), 2 * POLL_INTERVAL_MS);
        assert_eq!(driver.machine.status(), Status::Disabled);
        assert_eq!(store.read(VIDEO_SOURCE).as_deref(), Some("RTSP Video Stream"));
        assert_eq!(store.read(VIDEO_UDP_URL), None, "a setting that was never written is forgotten again, not left at the radio's port");
        assert_eq!(radio.calls.borrow().last().map(String::as_str), Some("stop"));
    }

    #[test]
    fn a_platform_without_usb_access_reads_no_adapter_as_qt_on_android_does() {
        let mut driver = Driver::default();
        driver.tick(&WithoutUsb, &Memory::default(), &enabled(), Some("/k"), 0);
        assert_eq!(driver.machine.status(), Status::NoAdapter);
        assert!(driver.machine.retry_armed());
    }

    #[test]
    fn no_adapter_retries_and_a_refused_adapter_reads_as_qt_words_it() {
        let mut driver = Driver::default();
        let store = Memory::default();
        driver.tick(&Fake::default(), &store, &enabled(), Some("/k"), 0);
        assert_eq!(status_text(driver.machine.status(), driver.machine.adapter(), driver.machine.start_error()), "No supported Wi-Fi adapter found");
        let busy = Fake { devices: vec![Adapter { display_name: "RTL8812AU".into(), known: true }], refuse: Some("LIBUSB_ERROR_BUSY".into()), ..Fake::default() };
        driver.tick(&busy, &store, &enabled(), Some("/k"), 2999);
        assert!(busy.calls.borrow().is_empty(), "the retry waits its three seconds");
        driver.tick(&busy, &store, &enabled(), Some("/k"), 3000);
        assert_eq!(status_text(driver.machine.status(), driver.machine.adapter(), driver.machine.start_error()), "RTL8812AU cannot be started: LIBUSB_ERROR_BUSY");
    }
}
