use serde_json::{Value, json};

use crate::videostate::{SOURCE_UDP_H264, SOURCE_UDP_H265};

pub const ANTENNA_COUNT: usize = 2;
pub const VIDEO_PORT: u16 = 5600;
pub const VIDEO_HOST: &str = "0.0.0.0";
pub const RETRY_INTERVAL_MS: u64 = 3000;
pub const POLL_INTERVAL_MS: u64 = 1000;
pub const STATS_STALE_MS: u64 = 2 * POLL_INTERVAL_MS;
pub const DEFAULT_KEY_FILE: &str = "openipc-default-gs.key";
pub const DEFAULT_GS_KEY_HEX: &str = "bbb7ed6e83a46a8a9b8a12a0f98ece2bdc978705b8204701b2085fa28cac7b460e05c48a6195fb70921c747a66e83c02e640bd6bbeb5b251537a98a27416a263";
pub const CODEC_H265: &str = "H265";
pub const RSSI_RAW_TO_DBM: i32 = -110;
pub const RSSI_UNIT: &str = "dBm";
pub const SNR_UNIT: &str = "dB";
pub const PACKET_LOSS_UNIT: &str = "packetsPerSecond";
pub const LINK_SCORE_MIN: i32 = 1000;
pub const LINK_SCORE_MAX: i32 = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Disabled,
    NoAdapter,
    AdapterUnavailable,
    InvalidKey,
    Listening,
    Receiving,
}

pub const STATUS_TOKENS: [&str; 6] = ["disabled", "noAdapter", "adapterUnavailable", "invalidKey", "listening", "receiving"];

pub const STATUSES: [Status; STATUS_TOKENS.len()] =
    [Status::Disabled, Status::NoAdapter, Status::AdapterUnavailable, Status::InvalidKey, Status::Listening, Status::Receiving];

impl Status {
    pub fn token(self) -> &'static str {
        STATUS_TOKENS[self as usize]
    }

    pub fn parse(token: &str) -> Option<Status> {
        STATUSES.into_iter().find(|status| status.token() == token)
    }

    pub fn link_active(self) -> bool {
        matches!(self, Status::Listening | Status::Receiving)
    }

    pub fn retrying(self) -> bool {
        matches!(self, Status::NoAdapter | Status::AdapterUnavailable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub rssi_raw: [i32; ANTENNA_COUNT],
    pub snr: [i32; ANTENNA_COUNT],
    pub score: [i32; ANTENNA_COUNT],
    pub packets_lost: i32,
    pub at_ms: u64,
}

impl Reading {
    pub fn have_signal(&self) -> bool {
        self.rssi_raw.iter().any(|raw| *raw != 0)
    }

    pub fn rssi_dbm(&self) -> Vec<Option<i32>> {
        self.rssi_raw.iter().map(|raw| (*raw != 0).then(|| raw + RSSI_RAW_TO_DBM)).collect()
    }

    pub fn antenna_scores(&self) -> Vec<Option<i32>> {
        self.score.iter().map(|score| (LINK_SCORE_MIN..=LINK_SCORE_MAX).contains(score).then_some(*score)).collect()
    }

    pub fn link_score(&self) -> Option<i32> {
        self.score.iter().copied().filter(|score| (LINK_SCORE_MIN..=LINK_SCORE_MAX).contains(score)).max()
    }

    pub fn same_sample(&self, other: &Reading) -> bool {
        (self.rssi_raw, self.snr, self.score, self.packets_lost) == (other.rssi_raw, other.snr, other.score, other.packets_lost)
    }

    pub fn age_ms(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.at_ms)
    }

    pub fn stale(&self, now_ms: u64) -> bool {
        self.age_ms(now_ms) > STATS_STALE_MS
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Configured(String),
    BuiltinDefault,
}

impl Key {
    pub fn token(&self) -> &'static str {
        match self {
            Key::Configured(_) => "configured",
            Key::BuiltinDefault => "builtinDefault",
        }
    }

    pub fn path(&self) -> Option<&str> {
        match self {
            Key::Configured(path) => Some(path),
            Key::BuiltinDefault => None,
        }
    }
}

pub fn choose_key(configured: &str, configured_exists: bool, default_writable: bool) -> Option<Key> {
    match configured.trim() {
        "" => default_writable.then_some(Key::BuiltinDefault),
        path => configured_exists.then(|| Key::Configured(path.to_string())),
    }
}

pub fn rejected_key_path(configured: &str) -> String {
    match configured.trim() {
        "" => DEFAULT_KEY_FILE.to_string(),
        path => path.to_string(),
    }
}

pub fn default_key_bytes() -> Vec<u8> {
    DEFAULT_GS_KEY_HEX.as_bytes().chunks(2).filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    pub display_name: String,
    pub known: bool,
}

pub fn matches_saved_name(adapter: &Adapter, saved: &str) -> bool {
    !saved.is_empty() && saved == adapter.display_name
}

pub fn select_adapter<'a>(devices: &'a [Adapter], saved: &str) -> Option<&'a Adapter> {
    devices.iter().find(|device| device.known && matches_saved_name(device, saved)).or_else(|| devices.iter().find(|device| device.known))
}

pub fn adapter_names(devices: &[Adapter]) -> Vec<String> {
    devices.iter().filter(|device| device.known).map(|device| device.display_name.clone()).collect()
}

pub fn unsupported_names(devices: &[Adapter]) -> Vec<String> {
    devices.iter().filter(|device| !device.known).map(|device| device.display_name.clone()).collect()
}

pub fn video_source(codec: &str) -> &'static str {
    match codec == CODEC_H265 {
        true => SOURCE_UDP_H265,
        false => SOURCE_UDP_H264,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    pub channel: u8,
    pub channel_width: i64,
    pub key_file: String,
    pub key_file_exists: bool,
    pub default_key_writable: bool,
    pub device_name: String,
    pub alink_enabled: bool,
    pub alink_tx_power: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    pub channel: u8,
    pub channel_width: i64,
    pub key: Key,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    Start { adapter: String, channel: u8, channel_width: i64, key: Key },
    Stop,
    ArmRetry { after_ms: u64 },
    CancelRetry,
    StartPolling { every_ms: u64 },
    StopPolling,
    Adaptive { enabled: bool, tx_power: i64 },
    ApplyVideo { source: &'static str, host: &'static str, port: u16, low_latency: bool, save_previous: bool },
    RestoreVideo,
    Status(Status),
    Statistics,
    Adapters,
}

#[derive(Debug, Default)]
pub struct PacketRadio {
    status: Status,
    adapter: Option<String>,
    start_error: Option<String>,
    adapters: Vec<String>,
    unsupported: Vec<String>,
    opened: Option<Opened>,
    rejected_key: Option<String>,
    reading: Option<Reading>,
    video_packets: Option<i64>,
    acknowledged_video_packets: Option<i64>,
    running: bool,
    starting: bool,
    retry_armed: bool,
    polling: bool,
    video_overridden: bool,
}

impl PacketRadio {
    pub fn reported(status: Status, adapter: Option<&str>, reading: Option<Reading>, video_packets: Option<i64>, start_error: Option<&str>) -> Self {
        PacketRadio {
            status,
            adapter: adapter.map(str::to_string),
            adapters: adapter.map(str::to_string).into_iter().collect(),
            start_error: start_error.filter(|error| !error.is_empty()).map(str::to_string),
            reading,
            video_packets,
            running: status.link_active(),
            polling: status.link_active(),
            retry_armed: status.retrying(),
            ..PacketRadio::default()
        }
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn link_active(&self) -> bool {
        self.status.link_active()
    }

    pub fn adapter(&self) -> Option<&str> {
        self.adapter.as_deref()
    }

    pub fn adapters(&self) -> &[String] {
        &self.adapters
    }

    pub fn unsupported(&self) -> &[String] {
        &self.unsupported
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn starting(&self) -> bool {
        self.starting
    }

    pub fn retry_armed(&self) -> bool {
        self.retry_armed
    }

    pub fn polling(&self) -> bool {
        self.polling
    }

    pub fn video_overridden(&self) -> bool {
        self.video_overridden
    }

    pub fn reading(&self) -> Option<Reading> {
        self.reading
    }

    pub fn opened(&self) -> Option<&Opened> {
        self.opened.as_ref()
    }

    pub fn video_packets(&self) -> Option<i64> {
        self.video_packets
    }

    pub fn start_error(&self) -> Option<&str> {
        self.start_error.as_deref()
    }

    fn fresh(&self, now_ms: u64) -> Option<Reading> {
        self.reading.filter(|reading| !reading.stale(now_ms))
    }

    fn video_seen(&self) -> bool {
        self.video_packets.unwrap_or_default() > 0
    }

    fn packet_loss(&self, now_ms: u64) -> Option<i32> {
        self.fresh(now_ms).filter(|_| self.video_seen()).map(|reading| reading.packets_lost)
    }

    fn set_status(&mut self, status: Status) -> Vec<Out> {
        match self.status == status {
            true => Vec::new(),
            false => {
                self.status = status;
                vec![Out::Status(status)]
            }
        }
    }

    fn arm_retry(&mut self) -> Vec<Out> {
        match std::mem::replace(&mut self.retry_armed, true) {
            true => Vec::new(),
            false => vec![Out::ArmRetry { after_ms: RETRY_INTERVAL_MS }],
        }
    }

    fn cancel_retry(&mut self) -> Vec<Out> {
        match std::mem::take(&mut self.retry_armed) {
            true => vec![Out::CancelRetry],
            false => Vec::new(),
        }
    }

    fn stop_polling(&mut self) -> Vec<Out> {
        match std::mem::take(&mut self.polling) {
            true => vec![Out::StopPolling],
            false => Vec::new(),
        }
    }

    pub fn on_devices(&mut self, devices: &[Adapter]) -> Vec<Out> {
        let (known, unsupported) = (adapter_names(devices), unsupported_names(devices));
        match (&known, &unsupported) == (&self.adapters, &self.unsupported) {
            true => Vec::new(),
            false => {
                self.adapters = known;
                self.unsupported = unsupported;
                vec![Out::Adapters]
            }
        }
    }

    pub fn on_enabled(&mut self, settings: &Settings, devices: &[Adapter]) -> Vec<Out> {
        match (settings.enabled, self.running || self.starting) {
            (false, _) => self.stop(),
            (true, true) => Vec::new(),
            (true, false) => self.try_start(settings, devices),
        }
    }

    pub fn on_retry(&mut self, settings: &Settings, devices: &[Adapter]) -> Vec<Out> {
        self.try_start(settings, devices)
    }

    fn try_start(&mut self, settings: &Settings, devices: &[Adapter]) -> Vec<Out> {
        if self.running || self.starting {
            return Vec::new();
        }
        let found = self.on_devices(devices);
        let Some(adapter) = select_adapter(devices, &settings.device_name).map(|device| device.display_name.clone()) else {
            self.adapter = None;
            return found.into_iter().chain(self.set_status(Status::NoAdapter)).chain(self.arm_retry()).collect();
        };
        self.adapter = Some(adapter.clone());
        let Some(key) = choose_key(&settings.key_file, settings.key_file_exists, settings.default_key_writable) else {
            self.rejected_key = Some(rejected_key_path(&settings.key_file));
            return found.into_iter().chain(self.set_status(Status::InvalidKey)).chain(self.cancel_retry()).collect();
        };
        self.start_error = None;
        self.rejected_key = None;
        self.starting = true;
        self.opened = Some(Opened { channel: settings.channel, channel_width: settings.channel_width, key: key.clone() });
        found
            .into_iter()
            .chain(self.cancel_retry())
            .chain([Out::Start { adapter, channel: settings.channel, channel_width: settings.channel_width, key }])
            .collect()
    }

    pub fn on_started(&mut self, settings: &Settings) -> Vec<Out> {
        self.running = true;
        self.starting = false;
        self.polling = true;
        self.start_error = None;
        self.video_packets = Some(0);
        self.acknowledged_video_packets = Some(0);
        self.cancel_retry()
            .into_iter()
            .chain([Out::StartPolling { every_ms: POLL_INTERVAL_MS }, Out::Adaptive { enabled: settings.alink_enabled, tx_power: settings.alink_tx_power }])
            .chain(self.set_status(Status::Listening))
            .collect()
    }

    pub fn on_start_failed(&mut self, error: &str) -> Vec<Out> {
        self.starting = false;
        self.opened = None;
        self.start_error = (!error.is_empty()).then(|| error.to_string());
        [Out::Stop].into_iter().chain(self.set_status(Status::AdapterUnavailable)).chain(self.arm_retry()).collect()
    }

    pub fn on_key_unavailable(&mut self, settings: &Settings) -> Vec<Out> {
        self.starting = false;
        self.opened = None;
        self.rejected_key = Some(rejected_key_path(&settings.key_file));
        [Out::Stop].into_iter().chain(self.set_status(Status::InvalidKey)).chain(self.cancel_retry()).collect()
    }

    pub fn on_video_packets(&mut self, total: i64) {
        self.video_packets = Some(total);
    }

    pub fn on_poll(&mut self, rssi_raw: [i32; ANTENNA_COUNT], snr: [i32; ANTENNA_COUNT], score: [i32; ANTENNA_COUNT], packets_lost: i32, now_ms: u64) -> Vec<Out> {
        if !self.running {
            return Vec::new();
        }
        let sample = Reading { rssi_raw, snr, score, packets_lost, at_ms: now_ms };
        self.reading = Some(match self.reading.filter(|previous| previous.same_sample(&sample)) {
            Some(unchanged) => unchanged,
            None => sample,
        });
        let receiving = self.video_packets > self.acknowledged_video_packets;
        self.acknowledged_video_packets = self.video_packets;
        self.set_status(match receiving {
            true => Status::Receiving,
            false => Status::Listening,
        })
        .into_iter()
        .chain([Out::Statistics])
        .collect()
    }

    pub fn on_rtp_stream(&mut self, codec: &str) -> Vec<Out> {
        if !self.running {
            return Vec::new();
        }
        let save_previous = !std::mem::replace(&mut self.video_overridden, true);
        vec![Out::ApplyVideo { source: video_source(codec), host: VIDEO_HOST, port: VIDEO_PORT, low_latency: true, save_previous }]
    }

    pub fn on_adaptive_changed(&mut self, settings: &Settings) -> Vec<Out> {
        match self.running {
            false => Vec::new(),
            true => vec![Out::Adaptive { enabled: settings.alink_enabled, tx_power: settings.alink_tx_power }],
        }
    }

    pub fn on_link_settings_changed(&mut self, settings: &Settings, devices: &[Adapter]) -> Vec<Out> {
        match self.running || self.starting || self.status != Status::Disabled {
            false => Vec::new(),
            true => self.stop().into_iter().chain(self.on_enabled(settings, devices)).collect(),
        }
    }

    pub fn on_wifi_stopped(&mut self, settings: &Settings, devices: &[Adapter]) -> Vec<Out> {
        self.release().into_iter().chain(self.on_enabled(settings, devices)).collect()
    }

    fn release(&mut self) -> Vec<Out> {
        let had_link = std::mem::take(&mut self.running) || std::mem::take(&mut self.starting);
        self.cancel_retry().into_iter().chain(self.stop_polling()).chain(had_link.then_some(Out::Stop)).collect()
    }

    pub fn stop(&mut self) -> Vec<Out> {
        let released = self.release();
        self.adapter = None;
        self.start_error = None;
        self.opened = None;
        self.rejected_key = None;
        self.reading = None;
        self.video_packets = None;
        self.acknowledged_video_packets = None;
        let restore = std::mem::take(&mut self.video_overridden).then_some(Out::RestoreVideo);
        released.into_iter().chain(restore).chain(self.set_status(Status::Disabled)).chain([Out::Statistics]).collect()
    }

    pub fn snapshot(&self, now_ms: u64) -> Value {
        let fresh = self.fresh(now_ms);
        json!({
            "kind": "object",
            "class": "PacketRadio",
            "status": self.status.token(),
            "linkActive": self.link_active(),
            "running": self.running,
            "starting": self.starting,
            "polling": self.polling,
            "adapter": &self.adapter,
            "adapters": &self.adapters,
            "unsupportedAdapters": &self.unsupported,
            "startError": &self.start_error,
            "retryArmed": self.retry_armed,
            "retryIntervalMs": RETRY_INTERVAL_MS,
            "pollIntervalMs": POLL_INTERVAL_MS,
            "antennaCount": ANTENNA_COUNT,
            "videoPort": VIDEO_PORT,
            "videoHost": VIDEO_HOST,
            "videoOverridden": self.video_overridden,
            "channel": self.opened.as_ref().map(|opened| opened.channel),
            "channelWidth": self.opened.as_ref().map(|opened| opened.channel_width),
            "keySource": self.opened.as_ref().map(|opened| opened.key.token()),
            "keyPath": self.opened.as_ref().and_then(|opened| opened.key.path()),
            "rejectedKeyPath": &self.rejected_key,
            "antennaRssi": fresh.map(|reading| reading.rssi_dbm()),
            "antennaSnr": fresh.map(|reading| reading.snr),
            "antennaScore": fresh.map(|reading| reading.antenna_scores()),
            "haveSignal": fresh.map(|reading| reading.have_signal()),
            "linkScore": fresh.and_then(|reading| reading.link_score()),
            "linkScoreMin": LINK_SCORE_MIN,
            "linkScoreMax": LINK_SCORE_MAX,
            "packetLoss": self.packet_loss(now_ms),
            "videoPackets": self.video_packets,
            "readingAgeMs": self.reading.map(|reading| reading.age_ms(now_ms)),
            "stale": self.reading.map(|reading| reading.stale(now_ms)),
            "staleAfterMs": STATS_STALE_MS,
            "rssiUnit": RSSI_UNIT,
            "snrUnit": SNR_UNIT,
            "packetLossUnit": PACKET_LOSS_UNIT,
        })
    }
}

const ANTENNA_SHAPE: &str = "two whole numbers written a/b";
const WHOLE_SHAPE: &str = "a whole number";

fn wanted(position: usize, shape: &str) -> String {
    format!("argument {} is not {shape}", position + 1)
}

fn present(args: &[String], position: usize) -> Option<&str> {
    args.get(position).map(|arg| arg.trim()).filter(|arg| !arg.is_empty())
}

fn antenna(args: &[String], position: usize) -> Result<Option<[i32; ANTENNA_COUNT]>, String> {
    let Some(arg) = present(args, position) else { return Ok(None) };
    let values: Vec<i32> = arg.split('/').map(str::trim).map(str::parse).collect::<Result<_, _>>().map_err(|_| wanted(position, ANTENNA_SHAPE))?;
    <[i32; ANTENNA_COUNT]>::try_from(values).map(Some).map_err(|_| wanted(position, ANTENNA_SHAPE))
}

fn whole<T: std::str::FromStr>(args: &[String], position: usize) -> Result<Option<T>, String> {
    let Some(arg) = present(args, position) else { return Ok(None) };
    arg.parse().map(Some).map_err(|_| wanted(position, WHOLE_SHAPE))
}

fn reported_snapshot(args: &[String]) -> Result<Value, String> {
    let Some(status) = present(args, 0).and_then(Status::parse) else {
        return Err(format!(
            "this needs a packet radio status token ({}), then optionally the adapter name, the per-antenna raw rssi, snr and link score written a/b, the packets lost in the last second, the \
             video packet count and the host's start error",
            STATUS_TOKENS.join(", ")
        ));
    };
    let reading = antenna(args, 2)?
        .map(|rssi_raw| -> Result<Reading, String> {
            Ok(Reading {
                rssi_raw,
                snr: antenna(args, 3)?.unwrap_or_default(),
                score: antenna(args, 4)?.unwrap_or_default(),
                packets_lost: whole(args, 5)?.unwrap_or_default(),
                at_ms: 0,
            })
        })
        .transpose()?;
    Ok(PacketRadio::reported(status, present(args, 1), reading, whole(args, 6)?, present(args, 7)).snapshot(0))
}

pub fn packet_radio_view(_backend: &dyn crate::router::Backend, args: &[String]) -> Value {
    reported_snapshot(args).unwrap_or_else(|reason| crate::read::refused(&reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            enabled: true,
            channel: 161,
            channel_width: 20,
            key_file: String::new(),
            key_file_exists: false,
            default_key_writable: true,
            device_name: String::new(),
            alink_enabled: true,
            alink_tx_power: 30,
        }
    }

    fn devices() -> Vec<Adapter> {
        vec![
            Adapter { display_name: "ALFA AWUS036ACM [1]".to_string(), known: true },
            Adapter { display_name: "Realtek 8812au [2]".to_string(), known: true },
            Adapter { display_name: "Some webcam [3]".to_string(), known: false },
        ]
    }

    fn started() -> PacketRadio {
        let mut radio = PacketRadio::default();
        radio.on_enabled(&settings(), &devices());
        radio.on_started(&settings());
        radio
    }

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    struct Nothing;

    impl crate::router::Backend for Nothing {
        fn get(&self, _path: &str) -> String {
            panic!("the packet radio view must not read Qt")
        }
        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            panic!("the packet radio view must not read Qt")
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            panic!("the packet radio view must not write Qt")
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            panic!("the packet radio view must not invoke Qt")
        }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn the_saved_adapter_wins_and_an_unsupported_one_is_named_rather_than_discarded() {
        assert_eq!(select_adapter(&devices(), "Realtek 8812au [2]").map(|d| d.display_name.as_str()), Some("Realtek 8812au [2]"), "a saved name that is still plugged in is the one to open");
        assert_eq!(select_adapter(&devices(), "").map(|d| d.display_name.as_str()), Some("ALFA AWUS036ACM [1]"), "with nothing saved the first known adapter is taken");
        assert_eq!(select_adapter(&devices(), "unplugged long ago").map(|d| d.display_name.as_str()), Some("ALFA AWUS036ACM [1]"), "a saved name that is gone falls back rather than refusing to start");
        assert_eq!(select_adapter(&[Adapter { display_name: "Some webcam [3]".to_string(), known: false }], "Some webcam [3]"), None, "an unknown adapter is never selected, even when its name is the saved one");
        assert_eq!(adapter_names(&devices()), vec!["ALFA AWUS036ACM [1]".to_string(), "Realtek 8812au [2]".to_string()], "the offered list holds known adapters only");
        assert!(!matches_saved_name(&devices()[0], ""), "an empty saved name matches nothing, so it can never pin the choice");

        let wrong_chipset = vec![Adapter { display_name: "Realtek 8188eu [4]".to_string(), known: false }];
        let mut radio = PacketRadio::default();
        assert_eq!(radio.on_enabled(&settings(), &wrong_chipset).first(), Some(&Out::Adapters), "the enumeration changed, so the head is told even though no adapter can be opened");
        let snapshot = radio.snapshot(0);
        assert_eq!(
            (&snapshot["status"], &snapshot["adapters"], &snapshot["unsupportedAdapters"]),
            (&json!("noAdapter"), &json!([] as [&str; 0]), &json!(["Realtek 8188eu [4]"])),
            "a seated adapter with the wrong chipset must not read the same as an empty USB port - the operator would go hunting for a dongle that is already plugged in"
        );

        let mut empty = PacketRadio::default();
        empty.on_enabled(&settings(), &[]);
        assert_eq!(empty.snapshot(0)["unsupportedAdapters"], json!([] as [&str; 0]), "and nothing plugged in names nothing");
    }

    #[test]
    fn a_start_that_fails_tears_the_link_down_retries_and_an_unreadable_key_stops_until_a_setting_changes() {
        let mut radio = PacketRadio::default();
        assert_eq!(radio.on_enabled(&settings(), &[]), vec![Out::Status(Status::NoAdapter), Out::ArmRetry { after_ms: RETRY_INTERVAL_MS }], "no adapter is a state worth retrying, and the retry interval is the C++ one");
        assert_eq!(radio.on_retry(&settings(), &[]), Vec::new(), "the host owns one repeating 3s timer, so re-arming an armed retry is a no-op and announcing it twice would be noise");
        assert_eq!(
            radio.on_retry(&settings(), &devices()),
            vec![Out::Adapters, Out::CancelRetry, Out::Start { adapter: "ALFA AWUS036ACM [1]".to_string(), channel: 161, channel_width: 20, key: Key::BuiltinDefault }],
            "the retry is cancelled the moment the start goes out, not when it lands, or a slow libusb claim lets the 3s timer open the same adapter twice"
        );
        assert_eq!(
            radio.on_start_failed("libusb: device busy"),
            vec![Out::Stop, Out::Status(Status::AdapterUnavailable), Out::ArmRetry { after_ms: RETRY_INTERVAL_MS }],
            "a failed start has to destroy the link object before the next attempt, because WfbngLink::start refuses forever once its usb thread and libusb context exist"
        );
        assert_eq!(radio.start_error(), Some("libusb: device busy"), "the host's own words travel as data so the head can write the sentence");
        assert!(radio.retry_armed() && !radio.running() && !radio.starting());

        let missing_key = Settings { key_file: "/nowhere/gs.key".to_string(), key_file_exists: false, ..settings() };
        assert_eq!(radio.on_retry(&missing_key, &devices()), vec![Out::Status(Status::InvalidKey), Out::CancelRetry], "an unreadable key is not a transient fault, so the retry is cancelled rather than left armed");
        assert!(!radio.retry_armed(), "and nothing may leave the retry flag set behind an invalid key");
        assert_eq!(radio.snapshot(0)["rejectedKeyPath"], json!("/nowhere/gs.key"), "a rejected key has to name the path it rejected, or the operator checks the wrong file");

        let fixed_key = Settings { key_file_exists: true, ..missing_key };
        let out = radio.on_link_settings_changed(&fixed_key, &devices());
        assert_eq!(out.last(), Some(&Out::Start { adapter: "ALFA AWUS036ACM [1]".to_string(), channel: 161, channel_width: 20, key: Key::Configured("/nowhere/gs.key".to_string()) }), "a settings change is what clears the invalid key state, so it must reach a fresh start");
        assert_eq!(choose_key("  ", false, true), Some(Key::BuiltinDefault), "a blank path means the built-in key, which needs no file to exist");
    }

    #[test]
    fn a_built_in_key_that_cannot_be_written_is_a_key_fault_and_not_a_usb_fault() {
        assert_eq!(choose_key("", false, false), None, "the C++ returns an empty path when it cannot create the fallback key, and an empty path is InvalidKey there");
        let unwritable = Settings { default_key_writable: false, ..settings() };
        let mut radio = PacketRadio::default();
        radio.on_enabled(&unwritable, &[]);
        assert!(radio.retry_armed());
        assert_eq!(
            radio.on_retry(&unwritable, &devices()),
            vec![Out::Adapters, Out::Status(Status::InvalidKey), Out::CancelRetry],
            "an app-data directory that cannot be written is diagnosed as a key fault with the retry stopped, never as an adapter another app is holding"
        );
        assert!(!radio.retry_armed(), "retrying an unwritable directory every three seconds forever is the latch the C++ avoided by stopping the timer");
        assert_eq!(radio.snapshot(0)["rejectedKeyPath"], json!(DEFAULT_KEY_FILE), "and it names the built-in key file, which is the file that could not be created");

        let mut late = started();
        assert_eq!(
            late.on_key_unavailable(&unwritable),
            vec![Out::Stop, Out::Status(Status::InvalidKey)],
            "the host can only discover an unwritable key while resolving the path, so it needs a way in that does not land on adapterUnavailable"
        );
        assert!(!late.retry_armed() && !late.starting(), "and the way in leaves nothing armed and nothing half-open");
    }

    #[test]
    fn nothing_opens_the_same_adapter_twice_while_a_start_is_still_in_flight() {
        let mut radio = PacketRadio::default();
        assert!(matches!(radio.on_enabled(&settings(), &devices()).last(), Some(Out::Start { .. })));
        assert!(radio.starting() && !radio.running());
        assert_eq!(radio.on_retry(&settings(), &devices()), Vec::new(), "a retry that fires between the start command and the host's answer must not issue a second start");
        assert_eq!(radio.on_enabled(&settings(), &devices()), Vec::new(), "and neither may the enable switch being toggled during the same window");
        assert_eq!(radio.snapshot(0)["starting"], json!(true), "the in-flight start is visible, so a head is not left showing a dead 'off' while libusb claims the adapter");

        let mut dropped = PacketRadio::default();
        dropped.on_enabled(&settings(), &devices());
        let out = dropped.on_wifi_stopped(&settings(), &devices());
        assert_eq!(out.first(), Some(&Out::Stop), "an adapter that vanishes mid-start still has to tear the half-open link down, or the fresh start is stacked on an orphan");
        assert!(out.iter().any(|o| matches!(o, Out::Start { .. })));
    }

    #[test]
    fn the_link_parameters_the_receiver_opened_with_are_the_ones_it_reports() {
        let radio = started();
        let snapshot = radio.snapshot(0);
        assert_eq!(
            (&snapshot["channel"], &snapshot["channelWidth"], &snapshot["keySource"], &snapshot["keyPath"]),
            (&json!(161), &json!(20), &json!("builtinDefault"), &Value::Null),
            "listening with no signal is almost always a channel or width mismatch with the aircraft, so the snapshot has to say which channel it is listening on"
        );

        let configured = Settings { key_file: "/etc/gs.key".to_string(), key_file_exists: true, channel: 149, ..settings() };
        let mut other = PacketRadio::default();
        other.on_enabled(&configured, &devices());
        other.on_started(&configured);
        let snapshot = other.snapshot(0);
        assert_eq!((&snapshot["channel"], &snapshot["keySource"], &snapshot["keyPath"]), (&json!(149), &json!("configured"), &json!("/etc/gs.key")));
        assert_eq!(Key::BuiltinDefault.path(), None);

        let mut stopped = started();
        stopped.stop();
        let snapshot = stopped.snapshot(0);
        assert_eq!((&snapshot["channel"], &snapshot["keySource"]), (&Value::Null, &Value::Null), "a stopped receiver is not listening on anything, so it reports no channel");
    }

    #[test]
    fn rssi_is_converted_to_the_dbm_the_operator_reads_it_against() {
        let mut radio = started();
        radio.on_poll([38, 26], [12, 6], [1800, 1650], 0, 1000);
        let snapshot = radio.snapshot(1000);
        assert_eq!(
            snapshot["antennaRssi"],
            json!([-72, -84]),
            "wfb feeds add_rssi(uint8_t, uint8_t) a raw gain figure and RxQuality.h states the conversion as raw - 110; publishing the raw 38 under rssiUnit dBm reads as a kilowatt \
             transmitter instead of a marginal -72 dBm link"
        );
        assert_eq!(snapshot["rssiUnit"], json!("dBm"));
        assert_eq!(RSSI_RAW_TO_DBM, -110, "110 is wfb's own offset, not a number of ours");

        radio.on_poll([38, 0], [12, 0], [1800, 0], 0, 2000);
        let snapshot = radio.snapshot(2000);
        assert_eq!(snapshot["antennaRssi"], json!([-72, Value::Null]), "a raw zero is no sample at all, and -110 dBm would be a reading");
        assert_eq!(snapshot["haveSignal"], json!(true), "having signal is still any antenna carrying a raw level, which is the C++ test");

        let mut silent = started();
        silent.on_poll([0, 0], [0, 0], [0, 0], 0, 1000);
        assert_eq!(silent.snapshot(1000)["haveSignal"], json!(false));
    }

    #[test]
    fn freshness_is_timed_from_the_hardware_numbers_moving_and_not_from_our_own_poll() {
        let mut radio = started();
        radio.on_poll([38, 26], [12, 6], [1800, 1650], 4, 1000);
        radio.on_poll([38, 26], [12, 6], [1800, 1650], 4, 2000);
        radio.on_poll([38, 26], [12, 6], [1800, 1650], 4, 3000);
        let snapshot = radio.snapshot(3500);
        assert_eq!(
            (&snapshot["readingAgeMs"], &snapshot["stale"]),
            (&json!(2500), &json!(true)),
            "wfb writes rssi/snr/score/packets_lost only inside the matching-video-channel branch and never decays them, so a dead link keeps returning its last numbers forever - \
             stamping the poll instead of the change makes the one field that would catch a frozen display swear the display is live"
        );
        assert_eq!(
            (&snapshot["haveSignal"], &snapshot["linkScore"], &snapshot["packetLoss"], &snapshot["antennaRssi"]),
            (&Value::Null, &Value::Null, &Value::Null, &Value::Null),
            "and past that threshold the judgements go absent rather than repeating the last good link back to the operator"
        );

        radio.on_poll([37, 26], [12, 6], [1790, 1650], 6, 3600);
        let snapshot = radio.snapshot(3600);
        assert_eq!((&snapshot["readingAgeMs"], &snapshot["stale"]), (&json!(0), &json!(false)), "numbers that move restamp the reading, because that is the only evidence the link is alive");
        assert_eq!(STATS_STALE_MS, 2000, "two missed polls at the C++ interval");
    }

    #[test]
    fn polling_calls_it_receiving_only_while_the_video_counter_moves() {
        let mut radio = PacketRadio::default();
        radio.on_enabled(&settings(), &devices());
        assert_eq!(
            radio.on_started(&settings()),
            vec![Out::StartPolling { every_ms: POLL_INTERVAL_MS }, Out::Adaptive { enabled: true, tx_power: 30 }, Out::Status(Status::Listening)],
            "a started receiver polls at the C++ interval and gets the adaptive link settings before it is called listening"
        );
        assert_eq!(radio.on_poll([38, 26], [12, 6], [1800, 1650], 0, 1000), vec![Out::Statistics], "the first reading with no video yet leaves it listening, so no status is announced");
        assert_eq!(radio.reading().unwrap().link_score(), Some(1800), "the link score is the best antenna, not their sum or mean");
        assert!(radio.reading().unwrap().have_signal());
        radio.on_video_packets(64);
        assert_eq!(radio.on_poll([38, 26], [12, 6], [1800, 1650], 2, 2000), vec![Out::Status(Status::Receiving), Out::Statistics], "a video counter that moved between polls is what receiving means");
        assert_eq!(radio.on_poll([38, 26], [12, 6], [1800, 1650], 2, 3000), vec![Out::Status(Status::Listening), Out::Statistics], "and a counter that stopped moving drops back to listening");

        let mut stopped = PacketRadio::default();
        assert_eq!(stopped.on_poll([38, 26], [12, 6], [1800, 1650], 0, 1000), Vec::new(), "a poll with no receiver open cannot invent a reading");
        assert_eq!(stopped.reading(), None);
    }

    #[test]
    fn a_link_score_and_a_packet_loss_outside_their_domain_are_absences_not_values() {
        let fresh = started();
        let snapshot = fresh.snapshot(0);
        assert_eq!((&snapshot["haveSignal"], &snapshot["linkScore"], &snapshot["packetLoss"], &snapshot["stale"]), (&Value::Null, &Value::Null, &Value::Null, &Value::Null), "a receiver with no statistics yet must not read as a receiver with no signal");
        assert_eq!(snapshot["videoPackets"], json!(0), "a started receiver that has forwarded nothing has a real zero, which is not the same as no feed");
        assert_eq!((&snapshot["linkScoreMin"], &snapshot["linkScoreMax"]), (&json!(1000), &json!(2000)), "signal_quality.h fixes the score domain, and a bare integer with no domain cannot be drawn on a bar");

        let mut zeroed = started();
        zeroed.on_poll([0, 0], [0, 0], [0, 0], 0, 1000);
        let snapshot = zeroed.snapshot(1000);
        assert_eq!(
            (&snapshot["haveSignal"], &snapshot["linkScore"], &snapshot["antennaScore"], &snapshot["packetLoss"], &snapshot["stale"]),
            (&json!(false), &Value::Null, &json!([Value::Null, Value::Null]), &Value::Null, &json!(false)),
            "the members are zero-initialised until the first video frame, and a zero is below the worst real score of 1000 - drawn on a 1000..2000 bar it reads as a dead link on a healthy adapter"
        );

        let mut receiving = started();
        receiving.on_video_packets(4096);
        receiving.on_poll([38, 26], [12, 6], [1800, 1650], 0, 1000);
        let snapshot = receiving.snapshot(1000);
        assert_eq!(
            (&snapshot["packetLoss"], &snapshot["linkScore"]),
            (&json!(0), &json!(1800)),
            "once video has actually arrived a zero loss is a clean feed rather than the absence of one, and that is the only time it may be published"
        );
        assert_eq!(Reading { rssi_raw: [38, 26], snr: [12, 6], score: [1800, 2500], packets_lost: 0, at_ms: 0 }.link_score(), Some(1800), "a score above the domain is as unusable as one below it");
    }

    #[test]
    fn stopping_releases_the_link_restores_the_video_once_and_clears_every_flag() {
        let mut radio = started();
        radio.on_poll([38, 26], [12, 6], [1800, 1650], 3, 1000);
        assert_eq!(
            radio.on_rtp_stream(CODEC_H265),
            vec![Out::ApplyVideo { source: SOURCE_UDP_H265, host: VIDEO_HOST, port: VIDEO_PORT, low_latency: true, save_previous: true }],
            "the first stream is the one whose previous video settings the head has to keep"
        );
        assert_eq!(radio.on_rtp_stream("H264"), vec![Out::ApplyVideo { source: SOURCE_UDP_H264, host: VIDEO_HOST, port: VIDEO_PORT, low_latency: true, save_previous: false }], "a later codec change reapplies without asking the head to save again");

        let out = radio.stop();
        assert_eq!(out, vec![Out::StopPolling, Out::Stop, Out::RestoreVideo, Out::Status(Status::Disabled), Out::Statistics]);
        assert!(!radio.running() && !radio.polling() && !radio.retry_armed() && !radio.video_overridden(), "stopping is what clears every flag the running receiver set");
        assert_eq!((radio.reading(), radio.video_packets(), radio.adapter(), radio.start_error()), (None, None, None, None), "a stopped receiver has no readings at all, rather than a last one frozen on screen");
        assert_eq!(radio.stop().iter().filter(|out| **out == Out::RestoreVideo).count(), 0, "and the video is restored once, not on every stop");
        assert_eq!(radio.on_link_settings_changed(&settings(), &devices()), Vec::new(), "an idle disabled receiver is not restarted by a channel change; only the enable switch starts it");

        assert_eq!(
            radio.on_rtp_stream(CODEC_H265),
            Vec::new(),
            "a packet already in flight when the operator switched the radio off must not re-pin videoSource to udp 0.0.0.0:5600 and snapshot the just-restored camera as the new 'previous'"
        );
        assert!(!radio.video_overridden(), "so the override flag stays clear and their own camera feed is not hijacked until they toggle the radio twice");

        let mut lost = started();
        let out = lost.on_wifi_stopped(&settings(), &devices());
        assert_eq!(out.first(), Some(&Out::StopPolling));
        assert!(out.contains(&Out::Stop) && out.iter().any(|o| matches!(o, Out::Start { .. })), "a receiver the adapter dropped is released and then tried again from scratch");
        assert_eq!(lost.on_enabled(&Settings { enabled: false, ..settings() }, &devices()).last(), Some(&Out::Statistics));
    }

    #[test]
    fn the_view_answers_from_its_arguments_alone_and_refuses_the_ones_it_cannot_read() {
        let args = strings(&["receiving", "ALFA AWUS036ACM [1]", "38/26", "12/6", "1800/1650", "4", "2048"]);
        let view = packet_radio_view(&Nothing, &args);
        assert_eq!(view["status"], "receiving");
        assert_eq!((&view["linkActive"], &view["haveSignal"], &view["linkScore"], &view["packetLoss"], &view["videoPackets"]), (&json!(true), &json!(true), &json!(1800), &json!(4), &json!(2048)));
        assert_eq!((&view["rssiUnit"], &view["snrUnit"], &view["packetLossUnit"]), (&json!("dBm"), &json!("dB"), &json!("packetsPerSecond")), "the head formats the numbers, so the core names the units and never writes a sentence");
        assert_eq!(view["antennaRssi"], json!([-72, -84]));
        assert_eq!(view["adapter"], "ALFA AWUS036ACM [1]");
        assert!(view.get("statusTokens").is_none(), "the token catalogue is the contract, not payload the head enumerates from a reading");

        let bare = packet_radio_view(&Nothing, &strings(&["disabled"]));
        assert_eq!((&bare["linkActive"], &bare["haveSignal"], &bare["videoPackets"]), (&json!(false), &Value::Null, &Value::Null), "a status on its own carries no reading, and that is reported as absence");
        assert_eq!(packet_radio_view(&Nothing, &[])["kind"], "null");
        assert!(packet_radio_view(&Nothing, &strings(&["sideways"]))["reason"].as_str().unwrap().contains("noAdapter"), "a refused status token has to list the tokens that would have worked");

        let comma = packet_radio_view(&Nothing, &strings(&["receiving", "", "38,26"]));
        assert_eq!(comma["kind"], "null", "the wrong separator has to be refused, not collapsed into a receiver that reports no statistics at all");
        assert!(comma["reason"].as_str().unwrap().contains("argument 3"), "and the refusal names the argument that was wrong: {comma}");

        let bad_snr = packet_radio_view(&Nothing, &strings(&["receiving", "", "38/26", "12;6"]));
        assert_eq!((&bad_snr["kind"], &bad_snr["reason"]), (&json!("null"), &json!(wanted(3, ANTENNA_SHAPE))), "an unparseable snr defaulted to [0, 0] is indistinguishable from a real all-zero reading");
        assert_eq!(packet_radio_view(&Nothing, &strings(&["receiving", "", "38/26", "12/6", "1800/1650", "lots"]))["kind"], "null", "and so is a packet loss that is not a number");
        assert_eq!(
            packet_radio_view(&Nothing, &strings(&["receiving", "", "38/26", "12/6", "1800/1650", "4294967296"]))["kind"],
            "null",
            "a packet loss too large for the field is refused rather than wrapped into a small plausible number"
        );
        assert_eq!(packet_radio_view(&Nothing, &strings(&["receiving", "", "38/26", "12/6", "1800/1650", "", "2048"]))["antennaSnr"], json!([12, 6]), "an argument that is simply absent still defaults, because absence is a legitimate answer");

        assert_eq!(Status::parse("adapterUnavailable"), Some(Status::AdapterUnavailable));
    }

    #[test]
    fn the_view_can_show_the_two_error_states_it_exists_to_render() {
        let failed = packet_radio_view(&Nothing, &strings(&["adapterUnavailable", "ALFA AWUS036ACM [1]", "", "", "", "", "", "libusb: device busy"]));
        assert_eq!(
            (&failed["startError"], &failed["retryArmed"]),
            (&json!("libusb: device busy"), &json!(true)),
            "startError is the one piece of host prose the core carries as data, so the head developer writing that sentence has to be able to see it through the view"
        );
        let none = packet_radio_view(&Nothing, &strings(&["noAdapter"]));
        assert_eq!((&none["startError"], &none["retryArmed"]), (&Value::Null, &json!(true)), "a missing adapter is retried too, and a view that reports retryArmed false there is simply wrong");
        assert_eq!(packet_radio_view(&Nothing, &strings(&["invalidKey"]))["retryArmed"], json!(false), "an unreadable key is the one fault that stops the retry");
        assert_eq!(packet_radio_view(&Nothing, &strings(&["listening", "ALFA AWUS036ACM [1]"]))["retryArmed"], json!(false));
    }

    #[test]
    fn the_status_tokens_and_the_enum_are_one_list_in_one_order() {
        assert!(STATUSES.iter().enumerate().all(|(index, status)| *status as usize == index), "token() indexes STATUS_TOKENS by discriminant, so the two arrays have to stay in the same order");
        assert_eq!(
            STATUSES.map(Status::token),
            ["disabled", "noAdapter", "adapterUnavailable", "invalidKey", "listening", "receiving"],
            "the head switches on these exact spellings, so pin them to their variants here rather than to the array token() reads them out of"
        );
        assert_eq!(STATUS_TOKENS.len(), 6, "the head switches on these tokens, so the catalog and the enum stay the same size");
        assert!(STATUSES.into_iter().all(|status| Status::parse(status.token()) == Some(status)), "every token the core publishes is one it can read back");
        assert_eq!(STATUSES.iter().filter(|status| status.retrying()).count(), 2, "noAdapter and adapterUnavailable are the transient faults; the others are not retried");
    }

    #[test]
    fn the_numbers_are_the_literals_the_cpp_declares_and_not_whatever_this_module_says_they_are() {
        assert_eq!((RETRY_INTERVAL_MS, POLL_INTERVAL_MS, VIDEO_PORT, ANTENNA_COUNT), (3000, 1000, 5600, 2));
        assert_eq!(default_key_bytes().len(), 64);
        assert_eq!(&default_key_bytes()[..4], &[0xbb, 0xb7, 0xed, 0x6e]);
        assert_eq!(default_key_bytes().last(), Some(&0x63));
        assert_eq!(Key::BuiltinDefault.token(), "builtinDefault");
        assert_eq!(Key::Configured("/etc/gs.key".to_string()).token(), "configured");

        let cpp = std::fs::read_to_string(format!("{}/../src/PacketRadio/PacketRadioManager.cc", env!("CARGO_MANIFEST_DIR"))).unwrap_or_default();
        assert!(cpp.contains("kRetryIntervalMs"), "this guard reads PacketRadioManager.cc, which still runs the live state machine - without it the two copies of these rules drift in silence");
        let declared = |name: &str| -> Option<u64> { cpp.split(&format!("{name} = ")).nth(1)?.split(';').next()?.trim().parse().ok() };
        assert_eq!(
            (declared("kRetryIntervalMs"), declared("kPollIntervalMs"), declared("kVideoPort")),
            (Some(RETRY_INTERVAL_MS), Some(POLL_INTERVAL_MS), Some(VIDEO_PORT as u64)),
            "the C++ is the one of the two copies that flies, so its literals are the ones these constants have to match"
        );
        let hex: String = cpp.split("kDefaultGsKeyHex").nth(1).unwrap_or_default().split(';').next().unwrap_or_default().split('"').skip(1).step_by(2).collect();
        assert_eq!(hex, DEFAULT_GS_KEY_HEX, "a short or drifted ground station key writes a key file the aircraft will not talk to, and the link simply never opens");
        assert!(cpp.contains(DEFAULT_KEY_FILE), "and the fallback key has to be the same file the C++ creates, or each copy writes its own");
    }
}
