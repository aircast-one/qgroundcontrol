use std::collections::BTreeMap;

use crate::factmeta::{self, MetaData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub name: &'static str,
    pub prefix: &'static str,
    pub json: &'static str,
}

pub const GROUPS: [Group; 22] = [
    Group { name: "ADSBVehicleManager", prefix: "ADSBVehicleManager", json: include_str!("../../src/Settings/ADSBVehicleManager.SettingsGroup.json") },
    Group { name: "APMMavlinkStreamRate", prefix: "APMMavlinkStreamRate", json: include_str!("../../src/Settings/APMMavlinkStreamRate.SettingsGroup.json") },
    Group { name: "App", prefix: "", json: include_str!("../../src/Settings/App.SettingsGroup.json") },
    Group { name: "AutoConnect", prefix: "AutoConnect", json: include_str!("../../src/Settings/AutoConnect.SettingsGroup.json") },
    Group { name: "BatteryIndicator", prefix: "BatteryIndicator", json: include_str!("../../src/Settings/BatteryIndicator.SettingsGroup.json") },
    Group { name: "BrandImage", prefix: "Branding", json: include_str!("../../src/Settings/BrandImage.SettingsGroup.json") },
    Group { name: "FirmwareUpgrade", prefix: "FirmwareUpgrade", json: include_str!("../../src/Settings/FirmwareUpgrade.SettingsGroup.json") },
    Group { name: "FlightMap", prefix: "FlightMap", json: include_str!("../../src/Settings/FlightMap.SettingsGroup.json") },
    Group { name: "FlightMode", prefix: "FlightMode", json: include_str!("../../src/Settings/FlightMode.SettingsGroup.json") },
    Group { name: "FlyView", prefix: "FlyView", json: include_str!("../../src/Settings/FlyView.SettingsGroup.json") },
    Group { name: "GimbalController", prefix: "GimbalController", json: include_str!("../../src/Settings/GimbalController.SettingsGroup.json") },
    Group { name: "Maps", prefix: "Maps", json: include_str!("../../src/Settings/Maps.SettingsGroup.json") },
    Group { name: "Mavlink", prefix: "", json: include_str!("../../src/Settings/Mavlink.SettingsGroup.json") },
    Group { name: "MavlinkActions", prefix: "MavlinkActions", json: include_str!("../../src/Settings/MavlinkActions.SettingsGroup.json") },
    Group { name: "OfflineMaps", prefix: "OfflineMaps", json: include_str!("../../src/Settings/OfflineMaps.SettingsGroup.json") },
    Group { name: "PacketRadio", prefix: "PacketRadio", json: include_str!("../../src/Settings/PacketRadio.SettingsGroup.json") },
    Group { name: "PlanView", prefix: "PlanView", json: include_str!("../../src/Settings/PlanView.SettingsGroup.json") },
    Group { name: "RTK", prefix: "RTK", json: include_str!("../../src/Settings/RTK.SettingsGroup.json") },
    Group { name: "RemoteID", prefix: "RemoteID", json: include_str!("../../src/Settings/RemoteID.SettingsGroup.json") },
    Group { name: "Units", prefix: "Units", json: include_str!("../../src/Settings/Units.SettingsGroup.json") },
    Group { name: "Video", prefix: "Video", json: include_str!("../../src/Settings/Video.SettingsGroup.json") },
    Group { name: "Viewer3D", prefix: "Viewer3D", json: include_str!("../../src/Settings/Viewer3D.SettingsGroup.json") },
];

pub fn group(name: &str) -> Option<&'static Group> {
    GROUPS.iter().find(|g| g.name == name)
}

pub fn metadata(name: &str) -> Result<BTreeMap<String, MetaData>, String> {
    let g = group(name).ok_or(format!("no settings group named {name}"))?;
    factmeta::from_file(g.json)
}

pub fn settings_key(group_name: &str, fact: &str) -> Option<String> {
    group(group_name).map(|g| if g.prefix.is_empty() { fact.to_string() } else { format!("{}/{}", g.prefix, fact) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_group_loads_and_keys_follow_the_qsettings_layout() {
        let loaded: Vec<(&str, Result<_, _>)> = GROUPS.iter().map(|g| (g.name, metadata(g.name))).collect();
        assert!(loaded.iter().all(|(_, r)| r.as_ref().is_ok_and(|m| !m.is_empty())), "{loaded:?}");
        assert_eq!(settings_key("App", "offlineEditingCruiseSpeed").as_deref(), Some("offlineEditingCruiseSpeed"));
        assert_eq!(settings_key("Units", "customUnits").as_deref(), Some("Units/customUnits"));
        assert_eq!(settings_key("BrandImage", "userBrandImageIndoor").as_deref(), Some("Branding/userBrandImageIndoor"));
        assert_eq!(settings_key("Nope", "x"), None);
        assert!(metadata("Units").unwrap().contains_key("customUnits"));
    }
}
