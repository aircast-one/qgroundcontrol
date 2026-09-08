use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Firmware {
    Generic,
    Px4,
    ArduPilot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleClass {
    Generic,
    FixedWing,
    MultiRotor,
    Vtol,
    Sub,
    Rover,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub id: i64,
    pub raw_name: String,
    pub friendly_name: String,
    pub description: String,
    pub category: String,
    pub specifies_coordinate: bool,
    pub specifies_altitude_only: bool,
    pub standalone_coordinate: bool,
    pub friendly_edit: bool,
    pub is_takeoff: bool,
    pub is_land: bool,
    pub is_loiter: bool,
    pub params: BTreeMap<u8, Value>,
    pub hidden: BTreeSet<u8>,
}

const COMMON: &str = include_str!("../../src/MissionManager/MavCmdInfoCommon.json");
const FIXED_WING: &str = include_str!("../../src/MissionManager/MavCmdInfoFixedWing.json");
const MULTI_ROTOR: &str = include_str!("../../src/MissionManager/MavCmdInfoMultiRotor.json");
const VTOL: &str = include_str!("../../src/MissionManager/MavCmdInfoVTOL.json");
const SUB: &str = include_str!("../../src/MissionManager/MavCmdInfoSub.json");
const ROVER: &str = include_str!("../../src/MissionManager/MavCmdInfoRover.json");
const PX4_COMMON: &str = include_str!("../../src/FirmwarePlugin/PX4/PX4-MavCmdInfoCommon.json");
const PX4_FIXED_WING: &str = include_str!("../../src/FirmwarePlugin/PX4/PX4-MavCmdInfoFixedWing.json");
const PX4_MULTI_ROTOR: &str = include_str!("../../src/FirmwarePlugin/PX4/PX4-MavCmdInfoMultiRotor.json");
const PX4_VTOL: &str = include_str!("../../src/FirmwarePlugin/PX4/PX4-MavCmdInfoVTOL.json");
const PX4_SUB: &str = include_str!("../../src/FirmwarePlugin/PX4/PX4-MavCmdInfoSub.json");
const PX4_ROVER: &str = include_str!("../../src/FirmwarePlugin/PX4/PX4-MavCmdInfoRover.json");
const APM_COMMON: &str = include_str!("../../src/FirmwarePlugin/APM/APM-MavCmdInfoCommon.json");
const APM_FIXED_WING: &str = include_str!("../../src/FirmwarePlugin/APM/APM-MavCmdInfoFixedWing.json");
const APM_MULTI_ROTOR: &str = include_str!("../../src/FirmwarePlugin/APM/APM-MavCmdInfoMultiRotor.json");
const APM_VTOL: &str = include_str!("../../src/FirmwarePlugin/APM/APM-MavCmdInfoVTOL.json");
const APM_SUB: &str = include_str!("../../src/FirmwarePlugin/APM/APM-MavCmdInfoSub.json");
const APM_ROVER: &str = include_str!("../../src/FirmwarePlugin/APM/APM-MavCmdInfoRover.json");

fn file(firmware: Firmware, vehicle: VehicleClass) -> &'static str {
    match (firmware, vehicle) {
        (Firmware::Generic, VehicleClass::Generic) => COMMON,
        (Firmware::Generic, VehicleClass::FixedWing) => FIXED_WING,
        (Firmware::Generic, VehicleClass::MultiRotor) => MULTI_ROTOR,
        (Firmware::Generic, VehicleClass::Vtol) => VTOL,
        (Firmware::Generic, VehicleClass::Sub) => SUB,
        (Firmware::Generic, VehicleClass::Rover) => ROVER,
        (Firmware::Px4, VehicleClass::Generic) => PX4_COMMON,
        (Firmware::Px4, VehicleClass::FixedWing) => PX4_FIXED_WING,
        (Firmware::Px4, VehicleClass::MultiRotor) => PX4_MULTI_ROTOR,
        (Firmware::Px4, VehicleClass::Vtol) => PX4_VTOL,
        (Firmware::Px4, VehicleClass::Sub) => PX4_SUB,
        (Firmware::Px4, VehicleClass::Rover) => PX4_ROVER,
        (Firmware::ArduPilot, VehicleClass::Generic) => APM_COMMON,
        (Firmware::ArduPilot, VehicleClass::FixedWing) => APM_FIXED_WING,
        (Firmware::ArduPilot, VehicleClass::MultiRotor) => APM_MULTI_ROTOR,
        (Firmware::ArduPilot, VehicleClass::Vtol) => APM_VTOL,
        (Firmware::ArduPilot, VehicleClass::Sub) => APM_SUB,
        (Firmware::ArduPilot, VehicleClass::Rover) => APM_ROVER,
    }
}

fn entries(text: &str) -> Result<Vec<Map<String, Value>>, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let list = root.get("mavCmdInfo").and_then(Value::as_array).ok_or("no mavCmdInfo array")?;
    Ok(list.iter().filter_map(|entry| entry.as_object().cloned()).collect())
}

#[derive(Default, Clone)]
struct Layered {
    info: Map<String, Value>,
    params: BTreeMap<u8, Value>,
    removed: BTreeSet<u8>,
}

fn param_index(key: &str) -> Option<u8> {
    key.strip_prefix("param").and_then(|n| n.parse().ok()).filter(|n| (1..=7).contains(n))
}

fn overlay(base: Layered, entry: &Map<String, Value>) -> Layered {
    let removed_now: BTreeSet<u8> = entry
        .get("paramRemove")
        .and_then(Value::as_str)
        .map(|list| list.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_default();
    let params_now: BTreeMap<u8, Value> = entry.iter().filter_map(|(k, v)| param_index(k).map(|i| (i, v.clone()))).collect();
    let info = base
        .info
        .into_iter()
        .chain(entry.iter().filter(|(k, _)| param_index(k).is_none() && *k != "paramRemove").map(|(k, v)| (k.clone(), v.clone())))
        .collect();
    let removed = base.removed.union(&removed_now).copied().filter(|i| !params_now.contains_key(i)).collect();
    let params = base.params.into_iter().chain(params_now).collect();
    Layered { info, params, removed }
}

fn collapse(commands: BTreeMap<i64, Layered>, text: &str) -> BTreeMap<i64, Layered> {
    let last_wins: BTreeMap<i64, Map<String, Value>> = entries(text)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| entry.get("id").and_then(Value::as_i64).map(|id| (id, entry)))
        .collect();
    last_wins.iter().fold(commands, |mut acc, (id, entry)| {
        let base = acc.remove(id).unwrap_or_default();
        acc.insert(*id, overlay(base, entry));
        acc
    })
}

fn command(id: i64, layered: Layered) -> Command {
    let text = |key: &str| layered.info.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let flag = |key: &str| layered.info.get(key).and_then(Value::as_bool).unwrap_or(false);
    let or = |value: String, fallback: &str| if value.is_empty() { fallback.to_string() } else { value };
    let raw_name = text("rawName");
    Command {
        id,
        friendly_name: or(text("friendlyName"), &raw_name),
        category: or(text("category"), "Advanced"),
        raw_name,
        description: text("description"),
        specifies_coordinate: flag("specifiesCoordinate"),
        specifies_altitude_only: flag("specifiesAltitudeOnly"),
        standalone_coordinate: flag("standaloneCoordinate"),
        friendly_edit: flag("friendlyEdit"),
        is_takeoff: flag("isTakeoffCommand"),
        is_land: flag("isLandCommand"),
        is_loiter: flag("isLoiterCommand"),
        params: layered.params,
        hidden: layered.removed,
    }
}

pub fn tree(firmware: Firmware, vehicle: VehicleClass) -> BTreeMap<i64, Command> {
    let layers = [
        Some(file(Firmware::Generic, VehicleClass::Generic)),
        (vehicle != VehicleClass::Generic).then(|| file(Firmware::Generic, vehicle)),
        (firmware != Firmware::Generic).then(|| file(firmware, VehicleClass::Generic)),
        (firmware != Firmware::Generic && vehicle != VehicleClass::Generic).then(|| file(firmware, vehicle)),
    ];
    layers
        .into_iter()
        .flatten()
        .fold(BTreeMap::new(), collapse)
        .into_iter()
        .map(|(id, layered)| (id, command(id, layered)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_file_parses_and_the_base_tree_has_the_common_commands() {
        let firmwares = [Firmware::Generic, Firmware::Px4, Firmware::ArduPilot];
        let vehicles = [VehicleClass::Generic, VehicleClass::FixedWing, VehicleClass::MultiRotor, VehicleClass::Vtol, VehicleClass::Sub, VehicleClass::Rover];
        assert!(firmwares.iter().all(|f| vehicles.iter().all(|v| entries(file(*f, *v)).is_ok())));
        let base = tree(Firmware::Generic, VehicleClass::Generic);
        assert_eq!(base.len(), 90);
        assert_eq!(base[&176].friendly_name, "Set flight mode");
        assert_eq!(base[&95].friendly_name, "Home Position");
        assert!(base[&95].specifies_coordinate && base.values().all(|c| !c.raw_name.is_empty()));
        assert!(base[&22].is_takeoff && base[&21].is_land && base[&17].is_loiter);
        assert_eq!(base[&220].friendly_name, base[&220].raw_name);
        assert_eq!(base[&23].category, "Advanced");
    }

    #[test]
    fn a_vehicle_override_removes_parameters_without_touching_the_rest() {
        let generic = tree(Firmware::Generic, VehicleClass::Generic);
        let multi = tree(Firmware::Generic, VehicleClass::MultiRotor);
        assert!(generic[&17].hidden.is_empty() && generic[&17].params.contains_key(&3));
        assert_eq!(multi[&17].hidden, BTreeSet::from([3]));
        assert_eq!(multi[&17].params[&3]["default"], 50.0);
        assert!(multi[&17].params.contains_key(&4) && !multi[&17].hidden.contains(&4));
        assert_eq!(multi[&17].friendly_name, generic[&17].friendly_name);
        assert_eq!(multi[&18].hidden, BTreeSet::from([1, 2, 3, 4]));
    }

    #[test]
    fn firmware_layers_stack_on_top_of_the_vehicle_layer() {
        let px4 = tree(Firmware::Px4, VehicleClass::MultiRotor);
        let apm = tree(Firmware::ArduPilot, VehicleClass::MultiRotor);
        assert!(px4.len() >= 90 && apm.len() >= 90);
        assert_ne!(px4, apm);
        assert!(px4[&17].hidden.contains(&3));
    }
}
