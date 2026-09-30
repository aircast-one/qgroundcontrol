use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use mavlink::dialects::ardupilotmega::MavMessage;
use serde_json::Value;

use crate::linkconfig::{Kind, LinkConfig};
use crate::transport::{Frame, LinkId};

pub const GENERAL_NAME: &str = "MAVLink Forwarding Link";
pub const SUPPORT_NAME: &str = "MAVLink Support Forwarding Link";
const SIGNED: u8 = 0x01;

#[derive(Debug, Default)]
pub struct Forwards {
    general: Option<LinkId>,
    support: Option<LinkId>,
}

static FORWARDS: LazyLock<Mutex<Forwards>> = LazyLock::new(|| Mutex::new(Forwards::default()));

fn lock() -> MutexGuard<'static, Forwards> {
    FORWARDS.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn host_port(text: &str) -> Option<(String, u16)> {
    let (host, port) = text.trim().rsplit_once(':')?;
    Some((host.to_string(), port.parse().ok()?)).filter(|(host, _)| !host.is_empty())
}

pub fn outgoing(frame: &Frame) -> Option<Vec<u8>> {
    if matches!(frame.message, MavMessage::SETUP_SIGNING(_)) {
        return None;
    }
    if !frame.v2 {
        return matches!(frame.message, MavMessage::HEARTBEAT(_) | MavMessage::RADIO_STATUS(_)).then(|| frame.raw.clone());
    }
    if frame.raw.get(2).is_none_or(|flags| flags & SIGNED == 0) {
        return Some(frame.raw.clone());
    }
    let mut unsigned = Vec::new();
    mavlink::write_v2_msg(&mut unsigned, frame.header, &frame.message).ok()?;
    Some(unsigned)
}

fn setting(name: &str) -> Option<Value> {
    crate::settingsstore::raw_setting(&format!("settings.mavlinkSettings.{name}"))
}

fn open_to(name: &str, host: &str) -> Option<LinkId> {
    let (host, port) = host_port(host)?;
    let config = LinkConfig { name: name.to_string(), auto_connect: false, high_latency: false, kind: Kind::Udp { local_port: 0, hosts: vec![(host, port)] } };
    crate::linkhost::open(&crate::linkhost::TRANSPORTS, config, &[]).ok()
}

fn live(id: Option<LinkId>, open: &[LinkId]) -> Option<LinkId> {
    id.filter(|id| open.contains(id))
}

pub fn maintain() {
    let open = crate::linkhost::TRANSPORTS.lock().unwrap_or_else(PoisonError::into_inner).open_ids();
    let wanted = setting("forwardMavlink").and_then(|v| v.as_bool()).unwrap_or(false);
    let general = live(lock().general, &open);
    let general = match (wanted, general) {
        (true, None) => setting("forwardMavlinkHostName").and_then(|v| v.as_str().map(str::to_string)).and_then(|host| open_to(GENERAL_NAME, &host)),
        (_, held) => held,
    };
    let mut forwards = lock();
    forwards.general = general;
    forwards.support = live(forwards.support, &open);
}

pub fn start_support() -> bool {
    let host = setting("forwardMavlinkAPMSupportHostName").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    let opened = open_to(SUPPORT_NAME, &host);
    lock().support = opened;
    opened.is_some()
}

pub fn end_support() {
    if let Some(id) = lock().support.take() {
        crate::linkhost::close(&crate::linkhost::TRANSPORTS, id, "support forwarding ended");
    }
}

pub fn support_enabled() -> bool {
    let open = crate::linkhost::TRANSPORTS.lock().unwrap_or_else(PoisonError::into_inner).open_ids();
    live(lock().support, &open).is_some()
}

pub fn forward(frame: &Frame) {
    let (general, support) = {
        let forwards = lock();
        (forwards.general, forwards.support)
    };
    if [general, support].contains(&Some(frame.link)) {
        return;
    }
    let Some(bytes) = outgoing(frame) else { return };
    let general = general.filter(|_| setting("forwardMavlink").and_then(|v| v.as_bool()).unwrap_or(false));
    [general, support].into_iter().flatten().for_each(|id| {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, id, &bytes);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIGNATURE_LEN: usize = 13;
    use mavlink::MavHeader;
    use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, SETUP_SIGNING_DATA, SYS_STATUS_DATA};

    fn frame(message: MavMessage, v2: bool, raw: Vec<u8>) -> Frame {
        Frame { link: 1, replay: false, v2, header: MavHeader { system_id: 1, component_id: 1, sequence: 7 }, message, raw }
    }

    fn v2(message: &MavMessage) -> Vec<u8> {
        let mut bytes = Vec::new();
        mavlink::write_v2_msg(&mut bytes, MavHeader { system_id: 1, component_id: 1, sequence: 7 }, message).unwrap();
        bytes
    }

    #[test]
    fn a_host_names_its_port_after_the_last_colon() {
        assert_eq!(host_port("localhost:14445"), Some(("localhost".to_string(), 14445)));
        assert_eq!(host_port("support.ardupilot.org:xxxx"), None, "the placeholder default names no port, so nothing is opened");
        assert_eq!(host_port(":14445"), None);
    }

    #[test]
    fn frames_go_out_unsigned_and_signing_setup_and_v1_chatter_stay_home() {
        let status = MavMessage::SYS_STATUS(SYS_STATUS_DATA::default());
        let plain = v2(&status);
        assert_eq!(outgoing(&frame(status.clone(), true, plain.clone())), Some(plain.clone()), "an unsigned frame is forwarded byte for byte");
        let mut signed = plain.clone();
        signed[2] |= SIGNED;
        signed.extend([0u8; SIGNATURE_LEN]);
        assert_eq!(outgoing(&frame(status.clone(), true, signed)), Some(plain), "a signature from our key would fail on the far side, so it is stripped and the checksum redone");
        assert_eq!(outgoing(&frame(MavMessage::SETUP_SIGNING(SETUP_SIGNING_DATA::default()), true, vec![0xFD])), None, "the signing key never leaves");
        assert_eq!(outgoing(&frame(status, false, vec![0xFE])), None, "MAVLinkProtocol drops v1 traffic other than HEARTBEAT and RADIO_STATUS before forwarding");
        assert_eq!(outgoing(&frame(MavMessage::HEARTBEAT(HEARTBEAT_DATA::default()), false, vec![0xFE, 9])), Some(vec![0xFE, 9]));
    }
}
