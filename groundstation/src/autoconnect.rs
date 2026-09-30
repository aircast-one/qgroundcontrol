use std::collections::{BTreeMap, BTreeSet};

use crate::boards::{BoardTable, BoardType, PortInfo};

pub const UPDATE_INTERVAL_MS: u32 = 1000;
pub const DEFAULT_UDP_LINK_NAME: &str = "UDP Link (AutoConnect)";
pub const FORWARDING_LINK_NAME: &str = "MAVLink Forwarding Link";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub pixhawk: bool,
    pub sik_radio: bool,
    pub libre_pilot: bool,
    pub rtk_gps: bool,
    pub udp: bool,
    pub forward_mavlink: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    pub android: bool,
    pub windows: bool,
}

impl Host {
    pub fn connect_delay_ms(&self) -> u32 {
        if self.windows { 6000 } else { 1000 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    OpenUdp { name: String },
    OpenForwarding { name: String },
    OpenSerial { name: String, port: String, baud: u32, usb_direct: bool },
    ConnectRtk { port: String, name: String },
    DisconnectRtk,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AutoConnect {
    wait_list: BTreeMap<String, u32>,
    rtk_port: Option<String>,
}

pub fn filter_composite(ports: Vec<PortInfo>) -> Vec<PortInfo> {
    let (kept, _) = ports.into_iter().fold((Vec::new(), BTreeSet::new()), |(mut kept, mut seen), port| {
        let identity = port.vendor_id.zip(port.product_id).filter(|_| !port.serial_number.is_empty() && port.serial_number != "0").map(|(v, p)| (v, p, port.serial_number.clone()));
        let duplicate = identity.as_ref().is_some_and(|id| seen.contains(id)) && !port.description.contains("NMEA");
        if let Some(id) = identity {
            seen.insert(id);
        }
        if !duplicate {
            kept.push(port);
        }
        (kept, seen)
    });
    kept
}

fn allowed(settings: &Settings, board: BoardType, rtk_connected: bool) -> bool {
    match board {
        BoardType::Pixhawk => settings.pixhawk,
        BoardType::SiKRadio => settings.sik_radio,
        BoardType::OpenPilot => settings.libre_pilot,
        BoardType::RtkGps => settings.rtk_gps && !rtk_connected,
    }
}

impl AutoConnect {
    pub fn network(&self, settings: &Settings, open_udp_names: &[String]) -> Vec<Action> {
        let missing = |name: &str| !open_udp_names.iter().any(|n| n == name);
        [
            (settings.udp && missing(DEFAULT_UDP_LINK_NAME)).then(|| Action::OpenUdp { name: DEFAULT_UDP_LINK_NAME.to_string() }),
            (settings.forward_mavlink && missing(FORWARDING_LINK_NAME)).then(|| Action::OpenForwarding { name: FORWARDING_LINK_NAME.to_string() }),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    pub fn serial(&mut self, table: &BoardTable, settings: &Settings, host: &Host, ports: Vec<PortInfo>, connected_ports: &[String], nmea_port: &str) -> Vec<Action> {
        let ports = filter_composite(ports);
        let current: BTreeSet<String> = ports.iter().map(|p| p.system_location.clone()).collect();
        let mut actions = Vec::new();
        for port in &ports {
            let location = port.system_location.trim().to_string();
            if location == nmea_port.trim() {
                continue;
            }
            let Some((board, name)) = table.classify(port, host.android) else { continue };
            if !allowed(settings, board, self.rtk_port.is_some()) || table.is_bootloader(port, host.android) {
                continue;
            }
            if connected_ports.iter().any(|c| c.trim() == location) || self.rtk_port.as_deref() == Some(&location) {
                continue;
            }
            let Some(seen) = self.wait_list.get_mut(&location) else {
                self.wait_list.insert(location, 1);
                continue;
            };
            *seen += 1;
            if *seen * UPDATE_INTERVAL_MS <= host.connect_delay_ms() {
                continue;
            }
            self.wait_list.remove(&location);
            let link_name = format!("{name} on {} (AutoConnect)", port.port_name.trim());
            match board {
                BoardType::RtkGps => {
                    self.rtk_port = Some(location.clone());
                    actions.push(Action::ConnectRtk { port: location, name });
                }
                BoardType::Pixhawk => actions.push(Action::OpenSerial { name: link_name, port: location, baud: 115200, usb_direct: true }),
                BoardType::SiKRadio => actions.push(Action::OpenSerial { name: link_name, port: location, baud: 57600, usb_direct: false }),
                BoardType::OpenPilot => actions.push(Action::OpenSerial { name: link_name, port: location, baud: 115200, usb_direct: false }),
            }
        }
        if self.rtk_port.as_ref().is_some_and(|p| !current.contains(p)) {
            self.rtk_port = None;
            actions.push(Action::DisconnectRtk);
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings { pixhawk: true, sik_radio: true, libre_pilot: false, rtk_gps: true, udp: true, forward_mavlink: false }
    }

    fn port(location: &str, vid: u16, pid: u16, serial: &str, description: &str) -> PortInfo {
        PortInfo { system_location: location.into(), port_name: location.trim_start_matches("/dev/").into(), description: description.into(), manufacturer: String::new(), serial_number: serial.into(), vendor_id: Some(vid), product_id: Some(pid) }
    }

    #[test]
    fn a_pixhawk_connects_on_the_pass_after_the_delay_and_a_radio_gets_its_baud() {
        let table = BoardTable::bundled().unwrap();
        let host = Host { android: false, windows: false };
        let mut auto = AutoConnect::default();
        let pixhawk = || vec![port("/dev/cu.usbmodem1", 9900, 17, "A1", "PX4 FMU v2.x")];
        assert!(auto.serial(&table, &settings(), &host, pixhawk(), &[], "").is_empty());
        let second = auto.serial(&table, &settings(), &host, pixhawk(), &[], "");
        assert_eq!(second, vec![Action::OpenSerial { name: "PX4 FMU V2 on cu.usbmodem1 (AutoConnect)".into(), port: "/dev/cu.usbmodem1".into(), baud: 115200, usb_direct: true }]);
        assert!(auto.serial(&table, &settings(), &host, pixhawk(), &["/dev/cu.usbmodem1".into()], "").is_empty());
        let windows = Host { android: false, windows: true };
        let mut slow = AutoConnect::default();
        let passes: Vec<usize> = (0..7).map(|_| slow.serial(&table, &settings(), &windows, pixhawk(), &[], "").len()).collect();
        assert_eq!(passes, vec![0, 0, 0, 0, 0, 0, 1]);
        let sik = table.boards.iter().find(|(_, _, b, _)| *b == BoardType::SiKRadio).unwrap().clone();
        let radio = || vec![port("/dev/cu.usbserial", sik.0, sik.1.max(1), "R1", "")];
        let mut auto = AutoConnect::default();
        auto.serial(&table, &settings(), &host, radio(), &[], "");
        assert!(matches!(auto.serial(&table, &settings(), &host, radio(), &[], "").as_slice(), [Action::OpenSerial { baud: 57600, usb_direct: false, .. }]));
    }

    #[test]
    fn bootloaders_disabled_boards_and_the_nmea_port_are_skipped() {
        let table = BoardTable::bundled().unwrap();
        let host = Host { android: false, windows: false };
        let mut auto = AutoConnect::default();
        let bl = vec![port("/dev/cu.usbmodem1", 9900, 17, "A1", "PX4 BL FMU v2.x")];
        assert!(auto.serial(&table, &settings(), &host, bl.clone(), &[], "").is_empty() && auto.serial(&table, &settings(), &host, bl, &[], "").is_empty());
        let off = Settings { pixhawk: false, ..settings() };
        let pixhawk = vec![port("/dev/cu.usbmodem1", 9900, 17, "A1", "PX4 FMU v2.x")];
        assert!(auto.serial(&table, &off, &host, pixhawk.clone(), &[], "").is_empty() && auto.serial(&table, &off, &host, pixhawk.clone(), &[], "").is_empty());
        assert!(auto.serial(&table, &settings(), &host, pixhawk.clone(), &[], "/dev/cu.usbmodem1").is_empty() && auto.serial(&table, &settings(), &host, pixhawk, &[], "/dev/cu.usbmodem1 ").is_empty());
    }

    #[test]
    fn composite_devices_keep_their_first_port_and_rtk_follows_the_port() {
        let ports = vec![port("/dev/a", 9900, 17, "S", "PX4 FMU"), port("/dev/b", 9900, 17, "S", "PX4 FMU"), port("/dev/c", 9900, 17, "S", "NMEA GPS"), port("/dev/d", 9900, 17, "0", ""), port("/dev/e", 9900, 17, "0", "")];
        let kept: Vec<String> = filter_composite(ports).into_iter().map(|p| p.system_location).collect();
        assert_eq!(kept, vec!["/dev/a", "/dev/c", "/dev/d", "/dev/e"]);
        let table = BoardTable::bundled().unwrap();
        let rtk = table.boards.iter().find(|(_, _, b, _)| *b == BoardType::RtkGps).unwrap().clone();
        let gps = || vec![port("/dev/cu.gps", rtk.0, rtk.1.max(1), "G", "")];
        let host = Host { android: false, windows: false };
        let mut auto = AutoConnect::default();
        auto.serial(&table, &settings(), &host, gps(), &[], "");
        assert_eq!(auto.serial(&table, &settings(), &host, gps(), &[], ""), vec![Action::ConnectRtk { port: "/dev/cu.gps".into(), name: rtk.3.clone() }]);
        assert!(auto.serial(&table, &settings(), &host, gps(), &[], "").is_empty());
        assert_eq!(auto.serial(&table, &settings(), &host, Vec::new(), &[], ""), vec![Action::DisconnectRtk]);
        assert_eq!(auto.network(&settings(), &[]), vec![Action::OpenUdp { name: DEFAULT_UDP_LINK_NAME.into() }]);
        assert!(auto.network(&settings(), &[DEFAULT_UDP_LINK_NAME.to_string()]).is_empty());
    }
}
