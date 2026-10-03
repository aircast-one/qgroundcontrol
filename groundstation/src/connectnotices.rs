use serde_json::Value;

use crate::router::Backend;

pub const SETUP_INCOMPLETE: &str = "Configuration tasks remain before this vehicle is ready to fly. Setup shows what is left.";
pub const HITL_ENABLED: &str = "Warning: Hardware In The Loop (HITL) simulation is enabled for this vehicle.";

pub const OUTDATED_PX4: &str = "QGroundControl supports PX4 Pro firmware Version 1.4.1 and above. You are using a version prior to that which will lead to unpredictable results. Please upgrade your firmware.";
const SUPPORTED_PX4: (u32, u32, u32) = (1, 4, 1);

pub fn outdated_px4(flight_sw_version: u32) -> Option<&'static str> {
    let byte = |shift: u32| (flight_sw_version >> shift) & 0xFF;
    (flight_sw_version == 0 || (byte(24), byte(16), byte(8)) < SUPPORTED_PX4).then_some(OUTDATED_PX4)
}

pub const BAD_CUBE_BLACK: &str = "WARNING: The flight board you are using has a critical service bulletin against it which advises against flying. For details see: https://discuss.cubepilot.org/t/sb-0000002-critical-service-bulletin-for-cubes-purchased-between-january-2019-to-present-do-not-fly/406";

pub struct ParametersAnnounce {
    pub px4: bool,
    pub hitl: bool,
    pub cube_black_link: Option<crate::transport::LinkId>,
}

pub fn bad_cube_black_params(acc3: Option<i64>, gyr3: Option<i64>, enable_mask: Option<i64>) -> bool {
    acc3 == Some(0) && gyr3 == Some(0) && enable_mask.is_some_and(|mask| mask >= 7)
}

pub fn is_cube_black(port_description: &str) -> bool {
    port_description.contains("CubeBlack")
}

fn serial_port_description(link: crate::transport::LinkId) -> Option<String> {
    let config = crate::linkhost::TRANSPORTS.lock().unwrap().config(link)?;
    let crate::linkconfig::Kind::Serial { port_name, .. } = config.kind else { return None };
    crate::corelinks::port_infos().into_iter().find(|port| port.system_location == port_name).map(|port| port.description)
}

pub fn parameters_ready_notices(setup_ready: bool, px4: bool, hitl: bool) -> Vec<&'static str> {
    [(!setup_ready, SETUP_INCOMPLETE), (px4 && hitl, HITL_ENABLED)].into_iter().filter(|(due, _)| *due).map(|(_, text)| text).collect()
}

pub fn announce(backend: &dyn Backend) {
    if crate::qthost::present() {
        return;
    }
    let Some(announce) = crate::hub::lock().take_parameters_announce() else { return };
    let setup_ready = crate::setup::setup_view(backend, &[]).get("ready").and_then(Value::as_bool).unwrap_or(true);
    let bad_cube = announce.cube_black_link.and_then(serial_port_description).is_some_and(|description| is_cube_black(&description));
    parameters_ready_notices(setup_ready, announce.px4, announce.hitl).into_iter().chain(bad_cube.then_some(BAD_CUBE_BLACK)).for_each(|text| {
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
    fn a_cube_black_with_the_bulletin_imu_setup_is_warned_like_check_for_bad_cube_black() {
        assert!(bad_cube_black_params(Some(0), Some(0), Some(7)));
        assert!(bad_cube_black_params(Some(0), Some(0), Some(127)));
        assert!(!bad_cube_black_params(Some(0), Some(0), Some(6)), "INS_ENABLE_MASK below 7 has the third IMU off");
        assert!(!bad_cube_black_params(Some(1), Some(0), Some(7)), "a detected third accelerometer is a fixed board");
        assert!(!bad_cube_black_params(None, Some(0), Some(7)), "every parameter must exist");
        assert!(is_cube_black("CubeBlack"));
        assert!(is_cube_black("CubeBlack+"));
        assert!(!is_cube_black("CubeOrange"));
    }

    #[test]
    fn parameters_ready_warns_like_the_autopilot_plugins_prechecks() {
        assert_eq!(parameters_ready_notices(false, false, false), [SETUP_INCOMPLETE]);
        assert_eq!(parameters_ready_notices(true, true, true), [HITL_ENABLED], "PX4AutoPilotPlugin warns about SYS_HITL");
        assert!(parameters_ready_notices(true, false, true).is_empty(), "only PX4 reads SYS_HITL");
        assert!(parameters_ready_notices(true, true, false).is_empty());
    }
}
