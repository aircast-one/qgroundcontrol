use std::collections::{BTreeMap, BTreeSet};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mavlink::dialects::ardupilotmega::{MavMessage, MavSeverity, SETUP_SIGNING_DATA};
use mavlink::{MAVLinkV2MessageRaw, MavHeader, MavlinkVersion, SigningConfig, SigningData};

use crate::signingkeys::Key;
use crate::transport::LinkId;

const SIGNING_EPOCH_UNIX_SECONDS: u64 = 1_420_070_400;
const SIGNED_FLAG: u8 = 0x01;
const V2_MAGIC: u8 = 0xFD;
const RAW_FRAME_BYTES: usize = 1 + 9 + 255 + 2 + 13;
const CONFIRM_TIMEOUT_MS: u64 = 5000;
const RETRANSMIT_MS: u64 = 1500;
pub const SETUP_COPIES: usize = 2;
const BAD_SIGNATURE_ALERT: u8 = 3;
const DETECT_COOLDOWN_MS: u64 = 2000;
const MSG_RADIO_STATUS: u32 = 109;
const MSG_HEARTBEAT: u32 = 0;
const MSG_STATUSTEXT: u32 = 253;

pub fn signing_timestamp(now: SystemTime) -> u64 {
    let since_epoch = now.duration_since(UNIX_EPOCH + Duration::from_secs(SIGNING_EPOCH_UNIX_SECONDS)).unwrap_or_default();
    since_epoch.as_millis() as u64 * 100
}

const PERSISTED_TIMESTAMP_SAFETY_BUMP_TICKS: u64 = 6_000_000;

pub fn setup_signing(key: Option<Key>, target: (u8, u8), now: SystemTime, floor: u64) -> SETUP_SIGNING_DATA {
    SETUP_SIGNING_DATA {
        initial_timestamp: key.map_or(0, |_| signing_timestamp(now).max(floor)),
        target_system: target.0,
        target_component: target.1,
        secret_key: key.unwrap_or([0; 32]),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Policy {
    Strict,
    Pending,
}

impl Policy {
    fn accepts_unsigned(self, msgid: u32) -> bool {
        match self {
            Policy::Strict => msgid == MSG_RADIO_STATUS,
            Policy::Pending => [MSG_RADIO_STATUS, MSG_HEARTBEAT, MSG_STATUSTEXT].contains(&msgid),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Op {
    None,
    Enable { system: u8, target: (u8, u8), due_ms: u64, retry_ms: u64 },
    Disable { system: u8, target: (u8, u8), due_ms: u64, retry_ms: u64, unsigned_seen: bool },
}

struct Channel {
    name: String,
    key: Key,
    data: SigningData,
    sign_outgoing: bool,
    policy: Policy,
    op: Op,
    bad_signatures: u8,
    link_id: u8,
    timestamp: u64,
    streams: BTreeSet<(u8, u8, u8)>,
}

impl Channel {
    fn new(name: &str, key: Key, link: LinkId, sign_outgoing: bool, policy: Policy, op: Op, seed: u64) -> Channel {
        let link_id = (link & 0xFF) as u8;
        let data = SigningData::from_config(SigningConfig::new(key, link_id, true, false));
        Channel { name: name.to_string(), key, data, sign_outgoing, policy, op, bad_signatures: 0, link_id, timestamp: seed.saturating_add(PERSISTED_TIMESTAMP_SAFETY_BUMP_TICKS), streams: BTreeSet::new() }
    }

    fn sign(&mut self, bytes: &[u8]) -> Option<Vec<u8>> {
        let (header, message): (MavHeader, MavMessage) = crate::tlog::decode_frame(bytes, MavlinkVersion::V2)?;
        let mut raw = MAVLinkV2MessageRaw::new();
        raw.serialize_message_for_signing(header, &message);
        self.timestamp = self.timestamp.max(signing_timestamp(SystemTime::now()));
        raw.signature_timestamp_bytes_mut().copy_from_slice(&self.timestamp.to_le_bytes()[..6]);
        *raw.signature_link_id_mut() = self.link_id;
        let mut signature = [0u8; 6];
        raw.calculate_signature(&self.key, &mut signature);
        raw.signature_value_mut().copy_from_slice(&signature);
        self.timestamp += 1;
        Some(raw.raw_bytes().to_vec())
    }
}

#[derive(Default)]
pub struct Signing {
    channels: BTreeMap<LinkId, Channel>,
    detect_cooldown: BTreeMap<LinkId, u64>,
    retired: Vec<(String, u64)>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Inbound {
    pub accept: bool,
    pub notices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Status {
    pub state: &'static str,
    pub key_name: String,
    pub stream_count: usize,
}

fn raw_frame(bytes: &[u8]) -> Option<MAVLinkV2MessageRaw> {
    (bytes.first() == Some(&V2_MAGIC) && bytes.len() <= RAW_FRAME_BYTES).then(|| {
        let mut buffer = [0u8; RAW_FRAME_BYTES];
        buffer[..bytes.len()].copy_from_slice(bytes);
        MAVLinkV2MessageRaw::from_bytes_unparsed(buffer)
    })
}

pub fn is_signed(bytes: &[u8]) -> bool {
    bytes.first() == Some(&V2_MAGIC) && bytes.get(2).is_some_and(|flags| flags & SIGNED_FLAG != 0)
}

fn msgid(bytes: &[u8]) -> u32 {
    bytes.get(7..10).map_or(u32::MAX, |b| u32::from(b[0]) | (u32::from(b[1]) << 8) | (u32::from(b[2]) << 16))
}

pub fn verifies(key: Key, bytes: &[u8]) -> bool {
    let data = SigningData::from_config(SigningConfig::new(key, 0, false, false));
    raw_frame(bytes).is_some_and(|raw| data.verify_signature(&raw))
}

impl Signing {
    pub fn status(&self, link: LinkId) -> Status {
        match self.channels.get(&link) {
            None => Status { state: "off", key_name: String::new(), stream_count: 0 },
            Some(channel) => Status {
                state: match channel.op {
                    Op::Enable { .. } => "enabling",
                    Op::Disable { .. } => "disabling",
                    Op::None => "on",
                },
                key_name: channel.name.clone(),
                stream_count: channel.streams.len(),
            },
        }
    }

    pub fn key_in_use(&self, name: &str) -> bool {
        self.channels.values().any(|c| c.name == name && !matches!(c.op, Op::Enable { .. }))
    }

    pub fn begin_enable(&mut self, link: LinkId, target: (u8, u8), name: &str, key: Key, seed: u64, now_ms: u64) -> Result<SETUP_SIGNING_DATA, String> {
        if self.channels.get(&link).is_some_and(|c| c.op != Op::None) {
            return Err("Signing operation already pending".to_string());
        }
        let op = Op::Enable { system: target.0, target, due_ms: now_ms + CONFIRM_TIMEOUT_MS, retry_ms: now_ms + RETRANSMIT_MS };
        self.channels.insert(link, Channel::new(name, key, link, false, Policy::Pending, op, seed));
        Ok(setup_signing(Some(key), target, SystemTime::now(), seed.saturating_add(PERSISTED_TIMESTAMP_SAFETY_BUMP_TICKS)))
    }

    pub fn begin_disable(&mut self, link: LinkId, target: (u8, u8), now_ms: u64) -> Result<SETUP_SIGNING_DATA, String> {
        let channel = self.channels.get_mut(&link).ok_or("Channel not signing — cannot disable")?;
        if channel.op != Op::None {
            return Err("Signing operation already pending".to_string());
        }
        channel.policy = Policy::Pending;
        channel.op = Op::Disable { system: target.0, target, due_ms: now_ms + CONFIRM_TIMEOUT_MS, retry_ms: now_ms + RETRANSMIT_MS, unsigned_seen: false };
        Ok(setup_signing(None, target, SystemTime::now(), 0))
    }

    pub fn inbound(&mut self, link: LinkId, system: u8, bytes: &[u8], message: &MavMessage, stored: &dyn Fn() -> Vec<(String, Key, u64)>, now_ms: u64) -> Inbound {
        let signed = is_signed(bytes);
        let id = msgid(bytes);
        let Some(channel) = self.channels.get_mut(&link) else {
            if signed && self.detect_cooldown.get(&link).is_none_or(|until| now_ms >= *until) {
                match stored().into_iter().find(|(_, key, _)| verifies(*key, bytes)) {
                    Some((name, key, seed)) => {
                        self.channels.insert(link, Channel::new(&name, key, link, true, Policy::Strict, Op::None, seed));
                        self.detect_cooldown.remove(&link);
                    }
                    None => {
                        self.detect_cooldown.insert(link, now_ms + DETECT_COOLDOWN_MS);
                    }
                }
            }
            return Inbound { accept: true, notices: Vec::new() };
        };
        let pending_enable = matches!(channel.op, Op::Enable { .. });
        let verified = signed && raw_frame(bytes).filter(|raw| channel.data.verify_signature(raw)).map(|raw| channel.streams.insert((raw.system_id(), raw.component_id(), raw.signature_link_id()))).is_some();
        let valid = verified || channel.policy.accepts_unsigned(id);
        if !valid && !signed {
            return Inbound { accept: false, notices: Vec::new() };
        }
        if !valid {
            channel.bad_signatures = channel.bad_signatures.saturating_add(1);
            let notice = (channel.bad_signatures == BAD_SIGNATURE_ALERT).then(|| match pending_enable {
                true => format!("Vehicle {system}: MAVLink signing: {BAD_SIGNATURE_ALERT} consecutive bad signatures while enabling — the chosen key likely does not match the vehicle's stored key. Verify the key on the vehicle, then retry."),
                false => format!("Vehicle {system}: MAVLink signing: {BAD_SIGNATURE_ALERT} consecutive bad signatures on this link — wrong key or vehicle clock drift"),
            });
            return Inbound { accept: false, notices: notice.into_iter().collect() };
        }
        if valid {
            channel.bad_signatures = 0;
        }
        let notices = self.advance(link, system, verified, signed, message);
        Inbound { accept: valid, notices }
    }

    fn advance(&mut self, link: LinkId, system: u8, verified: bool, signed: bool, message: &MavMessage) -> Vec<String> {
        let Some(channel) = self.channels.get_mut(&link) else { return Vec::new() };
        let expected = match channel.op {
            Op::Enable { system, .. } | Op::Disable { system, .. } => system,
            Op::None => return Vec::new(),
        };
        if system != expected {
            return Vec::new();
        }
        if let MavMessage::STATUSTEXT(text) = message {
            let body = text.text.to_str().unwrap_or_default().to_string();
            if (text.severity as u8) <= (MavSeverity::MAV_SEVERITY_ERROR as u8) && body.to_lowercase().contains("signing") {
                return self.fail(link, &format!("Vehicle rejected signing change: {body}"));
            }
        }
        let heartbeat = matches!(message, MavMessage::HEARTBEAT(_));
        match (&channel.op, heartbeat) {
            (Op::Enable { .. }, true) if verified => {
                channel.sign_outgoing = true;
                channel.policy = Policy::Strict;
                channel.op = Op::None;
                Vec::new()
            }
            (Op::Disable { .. }, true) if !signed => {
                self.retire(link);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn fail(&mut self, link: LinkId, detail: &str) -> Vec<String> {
        let Some(channel) = self.channels.get_mut(&link) else { return Vec::new() };
        match channel.op {
            Op::Enable { .. } => {
                self.retire(link);
                vec![detail.to_string()]
            }
            Op::Disable { unsigned_seen: true, .. } => {
                self.retire(link);
                Vec::new()
            }
            Op::Disable { .. } => {
                channel.policy = Policy::Strict;
                channel.op = Op::None;
                vec!["Signing disable not confirmed — vehicle is unreachable or still requires signed messages. Local signing remains enabled.".to_string()]
            }
            Op::None => Vec::new(),
        }
    }

    pub fn tick(&mut self, now_ms: u64) -> (Vec<(LinkId, SETUP_SIGNING_DATA)>, Vec<String>) {
        let expired: Vec<(LinkId, bool)> = self
            .channels
            .iter()
            .filter_map(|(link, c)| match c.op {
                Op::Enable { due_ms, .. } if now_ms >= due_ms => Some((*link, true)),
                Op::Disable { due_ms, .. } if now_ms >= due_ms => Some((*link, false)),
                _ => None,
            })
            .collect();
        let notices: Vec<String> = expired
            .iter()
            .flat_map(|(link, enabling)| self.fail(*link, if *enabling { "Signing setup not confirmed by vehicle (timeout)" } else { "Signing disable not confirmed by vehicle (timeout)" }))
            .collect();
        let resend: Vec<(LinkId, SETUP_SIGNING_DATA)> = self
            .channels
            .iter_mut()
            .filter_map(|(link, c)| match &mut c.op {
                Op::Enable { target, retry_ms, .. } if now_ms >= *retry_ms => {
                    *retry_ms = now_ms + RETRANSMIT_MS;
                    Some((*link, setup_signing(Some(c.key), *target, SystemTime::now(), c.timestamp)))
                }
                Op::Disable { target, retry_ms, .. } if now_ms >= *retry_ms => {
                    *retry_ms = now_ms + RETRANSMIT_MS;
                    Some((*link, setup_signing(None, *target, SystemTime::now(), 0)))
                }
                _ => None,
            })
            .collect();
        (resend, notices)
    }

    pub fn outbound(&mut self, link: LinkId, bytes: &[u8]) -> Vec<u8> {
        match self.channels.get_mut(&link).filter(|c| c.sign_outgoing) {
            Some(channel) if bytes.first() == Some(&V2_MAGIC) => channel.sign(bytes).unwrap_or_else(|| bytes.to_vec()),
            _ => bytes.to_vec(),
        }
    }

    fn retire(&mut self, link: LinkId) {
        if let Some(channel) = self.channels.remove(&link).filter(|c| c.timestamp > 0) {
            self.retired.push((channel.name, channel.timestamp));
        }
    }

    pub fn take_timestamps(&mut self) -> Vec<(String, u64)> {
        let live: Vec<(String, u64)> = self.channels.values().filter(|c| c.timestamp > 0).map(|c| (c.name.clone(), c.timestamp)).collect();
        std::mem::take(&mut self.retired).into_iter().chain(live).collect()
    }

    pub fn closed(&mut self, link: LinkId) {
        self.retire(link);
        self.detect_cooldown.remove(&link);
    }
}

pub static SIGNING: LazyLock<Mutex<Signing>> = LazyLock::new(|| Mutex::new(Signing::default()));

pub fn lock() -> MutexGuard<'static, Signing> {
    SIGNING.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, STATUSTEXT_DATA};

    const KEY: Key = [7; 32];

    fn heartbeat() -> MavMessage {
        MavMessage::HEARTBEAT(HEARTBEAT_DATA::default())
    }

    fn frame(message: &MavMessage, key: Option<Key>) -> Vec<u8> {
        let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let mut raw = MAVLinkV2MessageRaw::new();
        match key {
            Some(key) => {
                raw.serialize_message_for_signing(header, message);
                SigningData::from_config(SigningConfig::new(key, 0, true, false)).sign_message(&mut raw);
            }
            None => raw.serialize_message(header, message),
        }
        raw.raw_bytes().to_vec()
    }

    fn no_keys() -> Vec<(String, Key, u64)> {
        Vec::new()
    }

    #[test]
    fn the_timestamp_counts_ten_microsecond_ticks_since_2015() {
        let epoch = UNIX_EPOCH + Duration::from_secs(SIGNING_EPOCH_UNIX_SECONDS);
        assert_eq!(signing_timestamp(epoch), 0);
        assert_eq!(signing_timestamp(epoch + Duration::from_millis(1)), 100);
        assert_eq!(signing_timestamp(UNIX_EPOCH), 0);
        let disable = setup_signing(None, (1, 1), SystemTime::now(), 0);
        assert_eq!((disable.initial_timestamp, disable.secret_key), (0, [0; 32]), "a zero key and timestamp turn signing off");
    }

    #[test]
    fn enabling_waits_for_a_heartbeat_signed_with_the_key_then_signs_everything() {
        let mut signing = Signing::default();
        let setup = signing.begin_enable(3, (1, 1), "field", KEY, 0, 0).unwrap();
        assert_eq!((setup.secret_key, setup.target_system), (KEY, 1));
        assert_eq!(signing.status(3).state, "enabling");
        assert_eq!(signing.outbound(3, &frame(&heartbeat(), None)), frame(&heartbeat(), None), "nothing is signed until the vehicle confirms");
        assert!(signing.inbound(3, 1, &frame(&heartbeat(), None), &heartbeat(), &no_keys, 10).accept, "an unsigned heartbeat still passes while pending");
        assert!(signing.inbound(3, 1, &frame(&heartbeat(), Some(KEY)), &heartbeat(), &no_keys, 20).accept);
        assert_eq!(signing.status(3), Status { state: "on", key_name: "field".into(), stream_count: 1 });
        signing.inbound(3, 1, &frame(&heartbeat(), Some(KEY)), &heartbeat(), &no_keys, 25);
        assert_eq!(signing.status(3).stream_count, 1, "a stream is one system, component and link id, counted once");
        let out = signing.outbound(3, &frame(&heartbeat(), None));
        assert!(is_signed(&out) && verifies(KEY, &out));
        assert!(!signing.inbound(3, 1, &frame(&heartbeat(), None), &heartbeat(), &no_keys, 30).accept, "once on, unsigned traffic is refused");
        let alerts: Vec<String> = (31..40).flat_map(|t| signing.inbound(3, 1, &frame(&heartbeat(), None), &heartbeat(), &no_keys, t).notices).collect();
        assert!(alerts.is_empty(), "unsigned telemetry is dropped quietly: the pinned mavlink_parse_char returns nothing for it, so QGC's bad-signature burst never counts it and enabling raises no false key-mismatch alert");
    }

    #[test]
    fn a_wrong_key_alerts_after_three_bad_signatures_and_the_timeout_gives_up() {
        let mut signing = Signing::default();
        signing.begin_enable(3, (1, 1), "field", KEY, 0, 0).unwrap();
        let attitude = MavMessage::ATTITUDE(mavlink::dialects::ardupilotmega::ATTITUDE_DATA::default());
        let wrong = frame(&attitude, Some([9; 32]));
        assert!(signing.inbound(3, 1, &frame(&heartbeat(), Some([9; 32])), &heartbeat(), &no_keys, 0).accept, "a badly signed heartbeat passes while pending, as accept_unsigned_callback overrides a failed signature check");
        let notices: Vec<String> = (0..3).flat_map(|i| signing.inbound(3, 1, &wrong, &attitude, &no_keys, i).notices).collect();
        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("while enabling"));
        let (resend, _) = signing.tick(1500);
        assert_eq!(resend.len(), 1, "SETUP_SIGNING is retransmitted every 1.5 s");
        let (_, timeout) = signing.tick(5000);
        assert_eq!(timeout, ["Signing setup not confirmed by vehicle (timeout)"]);
        assert_eq!(signing.status(3).state, "off");
    }

    #[test]
    fn a_signing_statustext_from_the_vehicle_aborts_the_change() {
        let mut signing = Signing::default();
        signing.begin_enable(3, (1, 1), "field", KEY, 0, 0).unwrap();
        let text = MavMessage::STATUSTEXT(STATUSTEXT_DATA { severity: MavSeverity::MAV_SEVERITY_ERROR, text: "Signing refused while armed".into(), ..Default::default() });
        let notices = signing.inbound(3, 1, &frame(&text, None), &text, &no_keys, 10).notices;
        assert_eq!(notices, ["Vehicle rejected signing change: Signing refused while armed"]);
    }

    #[test]
    fn disabling_needs_an_unsigned_heartbeat_and_a_timeout_keeps_signing_on() {
        let mut signing = Signing::default();
        assert!(signing.begin_disable(3, (1, 1), 0).is_err());
        signing.begin_enable(3, (1, 1), "field", KEY, 0, 0).unwrap();
        signing.inbound(3, 1, &frame(&heartbeat(), Some(KEY)), &heartbeat(), &no_keys, 10);
        signing.begin_disable(3, (1, 1), 100).unwrap();
        assert!(signing.key_in_use("field"));
        let (_, notices) = signing.tick(5100);
        assert!(notices[0].starts_with("Signing disable not confirmed"));
        assert_eq!(signing.status(3).state, "on");
        signing.begin_disable(3, (1, 1), 6000).unwrap();
        signing.inbound(3, 1, &frame(&heartbeat(), None), &heartbeat(), &no_keys, 6100);
        assert_eq!(signing.status(3).state, "off");
    }

    #[test]
    fn a_signed_vehicle_is_recognised_by_a_stored_key() {
        let mut signing = Signing::default();
        let stored = || vec![("bench".to_string(), [1; 32], 0), ("field".to_string(), KEY, 0)];
        assert!(signing.inbound(3, 1, &frame(&heartbeat(), Some(KEY)), &heartbeat(), &stored, 0).accept);
        assert_eq!((signing.status(3).state, signing.status(3).key_name.as_str()), ("on", "field"));
        let mut unknown = Signing::default();
        unknown.inbound(4, 1, &frame(&heartbeat(), Some([5; 32])), &heartbeat(), &stored, 0);
        assert_eq!(unknown.status(4).state, "off");
        let calls = std::cell::Cell::new(0);
        let counted = || {
            calls.set(calls.get() + 1);
            stored()
        };
        unknown.inbound(4, 1, &frame(&heartbeat(), Some([5; 32])), &heartbeat(), &counted, 1000);
        assert_eq!(calls.get(), 0, "a miss waits two seconds before trying the keys again");
    }

    #[test]
    fn the_outgoing_timestamp_resumes_from_the_persisted_one_and_is_reported_when_the_link_goes() {
        let mut signing = Signing::default();
        let ahead = signing_timestamp(SystemTime::now()) + 10_000_000;
        let setup = signing.begin_enable(3, (1, 1), "field", KEY, ahead, 0).unwrap();
        assert_eq!(setup.initial_timestamp, ahead + PERSISTED_TIMESTAMP_SAFETY_BUMP_TICKS, "a stored timestamp ahead of the clock seeds SETUP_SIGNING with the same bump the channel signs from");
        signing.inbound(3, 1, &frame(&heartbeat(), Some(KEY)), &heartbeat(), &no_keys, 10);
        let out = signing.outbound(3, &frame(&heartbeat(), None));
        let bumped = ahead + PERSISTED_TIMESTAMP_SAFETY_BUMP_TICKS;
        assert_eq!(raw_frame(&out).unwrap().signature_timestamp(), bumped, "signing continues 60 s past the persisted counter rather than from the wall clock, as SigningChannel::init bumps it");
        assert!(verifies(KEY, &out));
        assert_eq!(signing.take_timestamps(), [("field".to_string(), bumped + 1)]);
        signing.closed(3);
        assert_eq!(signing.take_timestamps(), [("field".to_string(), bumped + 1)], "a closed link hands its last timestamp over once");
        assert!(signing.take_timestamps().is_empty());
    }
}
