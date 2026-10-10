use std::sync::OnceLock;

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltInRadio {
    pub port: &'static str,
    pub baud: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub radio: BuiltInRadio,
    pub udp_auto_connect: bool,
}

pub const BUILT_IN_RADIO_NAME: &str = "Built-in radio";
const UDP_AUTO_CONNECT: &str = "settings.autoConnectSettings.autoConnectUDP";
const PROFILES: [(&str, Profile); 1] = [("radiomaster-ax12", Profile { radio: BuiltInRadio { port: "/dev/ttyS1", baud: 460_800 }, udp_auto_connect: true })];

static PROFILE: OnceLock<Option<Profile>> = OnceLock::new();

pub fn profile_for(device: &str) -> Option<Profile> {
    PROFILES.iter().find(|(id, _)| *id == device).map(|(_, profile)| *profile)
}

pub fn adopt(device: Option<&str>) {
    let profile = device.and_then(profile_for);
    let _ = PROFILE.set(profile);
    if profile.is_some_and(|p| p.udp_auto_connect) && !crate::settingsstore::is_stored(UDP_AUTO_CONNECT) {
        crate::settingsstore::set_raw(UDP_AUTO_CONNECT, &Value::Bool(true));
    }
}

pub fn built_in_radio() -> Option<BuiltInRadio> {
    PROFILE.get().copied().flatten().map(|profile| profile.radio).filter(|radio| usable(radio.port))
}

#[cfg(target_os = "android")]
fn usable(port: &str) -> bool {
    std::ffi::CString::new(port).is_ok_and(|path| unsafe { libc::access(path.as_ptr(), libc::R_OK | libc::W_OK) } == 0)
}

#[cfg(not(target_os = "android"))]
fn usable(_port: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_radiomaster_ax12_brings_its_elrs_uart_and_wifi_auto_connect() {
        let ax12 = profile_for("radiomaster-ax12").unwrap();
        assert_eq!(ax12.radio, BuiltInRadio { port: "/dev/ttyS1", baud: 460_800 }, "RadioMaster's QGC release notes: Serial, ttyS1, 460800");
        assert!(ax12.udp_auto_connect, "the ELRS backpack sends MAVLink to UDP 14550 over Wi-Fi");
        assert_eq!(profile_for("pixel-8"), None, "an unknown device gets no built-in radio, so no UART is ever opened on it");
    }
}
