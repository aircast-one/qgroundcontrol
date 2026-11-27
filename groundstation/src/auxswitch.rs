use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;

pub const AUX_SWITCHES_VIEW: &str = "view.auxSwitches";
pub const AUX_SWITCH_SET: &str = "vehicle.auxSwitch";
const SENSOR_SLOTS: std::ops::RangeInclusive<u8> = 1..=9;

pub struct Switch {
    pub id: &'static str,
    pub label: &'static str,
    pub function: u16,
    sensor: &'static str,
}

pub const SWITCHES: [Switch; 2] = [
    Switch {
        id: "rangefinder",
        label: "Range finder",
        function: 10,
        sensor: "RNGFND",
    },
    Switch {
        id: "lidar",
        label: "LiDAR",
        function: 40,
        sensor: "PRX",
    },
];

static TURNED_OFF: Mutex<BTreeSet<(u8, &'static str)>> = Mutex::new(BTreeSet::new());

pub fn fitted(ardupilot: bool, sensor_type: impl Fn(&str) -> Option<f64>, switch: &Switch) -> bool {
    ardupilot
        && SENSOR_SLOTS.clone().any(|slot| {
            sensor_type(&format!("{}{slot}_TYPE", switch.sensor)).is_some_and(|kind| kind != 0.0)
        })
}

pub fn view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let hub = crate::hub::lock();
    let off = TURNED_OFF.lock().unwrap_or_else(PoisonError::into_inner);
    let switches: Vec<Value> = hub
        .active()
        .map(|vehicle| {
            let ardupilot = vehicle.autopilot == crate::modes::AUTOPILOT_ARDUPILOT;
            let sensor_type = |name: &str| vehicle.parameter(vehicle.component, name).map(|p| p.as_f64());
            SWITCHES
                .iter()
                .filter(|switch| fitted(ardupilot, sensor_type, switch))
                .map(|switch| json!({ "id": switch.id, "label": switch.label, "on": !off.contains(&(vehicle.id, switch.id)) }))
                .collect()
        })
        .unwrap_or_default();
    json!({ "switches": switches })
}

pub fn set(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let switch = given
        .get(0)
        .and_then(Value::as_str)
        .and_then(|id| SWITCHES.iter().find(|switch| switch.id == id));
    let (Some(switch), Some(on)) = (switch, given.get(1).and_then(Value::as_bool)) else {
        return json!({ "ok": false, "refusal": "badSwitch", "reason": "A switch is the range finder or the LiDAR, turned on or off." });
    };
    let vehicle = crate::guided::active_id(backend);
    let sent = crate::guided::dispatch(
        backend,
        Some(json!({ "action": "auxSwitch", "function": switch.function, "high": on })),
        vehicle,
        AUX_SWITCH_SET,
        args,
    );
    if let Some(id) = vehicle
        .and_then(|id| u8::try_from(id).ok())
        .filter(|_| crate::read::flag(&sent, "ok"))
    {
        let mut off = TURNED_OFF.lock().unwrap_or_else(PoisonError::into_inner);
        match on {
            true => off.remove(&(id, switch.id)),
            false => off.insert((id, switch.id)),
        };
    }
    sent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switch_shows_on_ardupilot_only_when_one_of_its_sensors_is_set_up() {
        let [rangefinder, lidar] = &SWITCHES;
        let owel = |name: &str| match name {
            "RNGFND1_TYPE" => Some(14.0),
            "PRX1_TYPE" | "PRX2_TYPE" | "RNGFND2_TYPE" => Some(0.0),
            _ => None,
        };
        assert!(
            fitted(true, owel, rangefinder),
            "owel-red-01's TeraRanger is RNGFND1_TYPE 14"
        );
        assert!(
            !fitted(true, owel, lidar),
            "its Lidar360 port has no PRX type set, so there is nothing to switch"
        );
        assert!(
            !fitted(false, owel, rangefinder),
            "PX4 has no ArduPilot aux functions"
        );
        assert!(fitted(
            true,
            |name| (name == "PRX3_TYPE").then_some(5.0),
            lidar
        ));
    }
}
