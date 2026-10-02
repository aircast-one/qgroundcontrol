use serde_json::Value;

use crate::router::Backend;

pub const SETUP_INCOMPLETE: &str = "Configuration tasks remain before this vehicle is ready to fly. See Vehicle Configuration for details.";
pub const HITL_ENABLED: &str = "Warning: Hardware In The Loop (HITL) simulation is enabled for this vehicle.";

pub const OUTDATED_PX4: &str = "QGroundControl supports PX4 Pro firmware Version 1.4.1 and above. You are using a version prior to that which will lead to unpredictable results. Please upgrade your firmware.";
const SUPPORTED_PX4: (u32, u32, u32) = (1, 4, 1);

pub fn outdated_px4(flight_sw_version: u32) -> Option<&'static str> {
    let byte = |shift: u32| (flight_sw_version >> shift) & 0xFF;
    (flight_sw_version == 0 || (byte(24), byte(16), byte(8)) < SUPPORTED_PX4).then_some(OUTDATED_PX4)
}

pub fn parameters_ready_notices(setup_ready: bool, px4: bool, hitl: bool) -> Vec<&'static str> {
    [(!setup_ready, SETUP_INCOMPLETE), (px4 && hitl, HITL_ENABLED)].into_iter().filter(|(due, _)| *due).map(|(_, text)| text).collect()
}

pub fn announce(backend: &dyn Backend) {
    if crate::qthost::present() {
        return;
    }
    let Some((px4, hitl)) = crate::hub::lock().take_parameters_announce() else { return };
    let setup_ready = crate::setup::setup_view(backend, &[]).get("ready").and_then(Value::as_bool).unwrap_or(true);
    parameters_ready_notices(setup_ready, px4, hitl).into_iter().for_each(|text| {
        crate::noticeboard::post_from_vehicle(crate::noticeboard::MESSAGE, text);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px4_older_than_1_4_1_is_warned_about_once_by_the_plugin() {
        assert_eq!(outdated_px4(0x01_04_00_00), Some(OUTDATED_PX4));
        assert_eq!(outdated_px4(0x01_04_01_00), None);
        assert_eq!(outdated_px4(0x01_0F_00_FF), None);
        assert_eq!(outdated_px4(0), Some(OUTDATED_PX4), "a vehicle that reports no version is warned too");
    }

    #[test]
    fn parameters_ready_warns_like_the_autopilot_plugins_prechecks() {
        assert_eq!(parameters_ready_notices(false, false, false), [SETUP_INCOMPLETE]);
        assert_eq!(parameters_ready_notices(true, true, true), [HITL_ENABLED], "PX4AutoPilotPlugin warns about SYS_HITL");
        assert!(parameters_ready_notices(true, false, true).is_empty(), "only PX4 reads SYS_HITL");
        assert!(parameters_ready_notices(true, true, false).is_empty());
    }
}
