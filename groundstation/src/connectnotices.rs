use serde_json::Value;

use crate::router::Backend;

pub const SETUP_INCOMPLETE: &str = "Configuration tasks remain before this vehicle is ready to fly. See Vehicle Configuration for details.";
pub const HITL_ENABLED: &str = "Warning: Hardware In The Loop (HITL) simulation is enabled for this vehicle.";

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
    fn parameters_ready_warns_like_the_autopilot_plugins_prechecks() {
        assert_eq!(parameters_ready_notices(false, false, false), [SETUP_INCOMPLETE]);
        assert_eq!(parameters_ready_notices(true, true, true), [HITL_ENABLED], "PX4AutoPilotPlugin warns about SYS_HITL");
        assert!(parameters_ready_notices(true, false, true).is_empty(), "only PX4 reads SYS_HITL");
        assert!(parameters_ready_notices(true, true, false).is_empty());
    }
}
