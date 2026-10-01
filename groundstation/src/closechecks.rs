use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicles.vehicles.count", "vehicle.parameterManager.pendingWrites", "plan.dirtyForSave", "plan.dirtyForUpload", crate::coreplan::CHANGED];
static VEHICLES_SEEN: AtomicU64 = AtomicU64::new(0);

pub fn deps() -> Vec<String> {
    DEPS.iter().map(|d| d.to_string()).chain((0..VEHICLES_SEEN.load(Ordering::Relaxed)).map(|index| format!("vehicles.vehicles.{index}.parameterManager.pendingWrites"))).collect()
}

const UNSAVED_MISSION: &str = "You have a mission edit in progress which has not been saved/sent. If you close you will lose changes. Are you sure you want to close?";
const PENDING_WRITES: &str = "You have pending parameter updates to a vehicle. If you close you will lose changes. Are you sure you want to close?";
const ACTIVE_CONNECTIONS: &str = "There are still active connections to vehicles. Are you sure you want to exit?";

pub struct State {
    pub vehicle: bool,
    pub dirty_for_save: bool,
    pub dirty_for_upload: bool,
    pub pending_writes: bool,
}

pub fn prompts(state: &State) -> Vec<(&'static str, &'static str)> {
    [
        (state.dirty_for_save && (state.dirty_for_upload || !state.vehicle)).then_some(("unsavedMission", UNSAVED_MISSION)),
        state.pending_writes.then_some(("pendingWrites", PENDING_WRITES)),
        state.vehicle.then_some(("activeConnections", ACTIVE_CONNECTIONS)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub fn close_checks_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let (dirty_for_save, dirty_for_upload) = match crate::coreplan::plan_state() {
        Some(core) => (core.dirty, core.dirty),
        None => {
            let plan = object(&backend.get_fields("plan", "dirtyForSave,dirtyForUpload"));
            (flag(&plan, "dirtyForSave"), flag(&plan, "dirtyForUpload"))
        }
    };
    let pending_writes = match crate::vehiclefacade::switched_on() {
        true => crate::hub::lock().any_pending_parameter_writes(),
        false => {
            let count = object(&backend.get("vehicles.vehicles.count")).get("value").and_then(Value::as_u64).unwrap_or(0);
            VEHICLES_SEEN.store(count, Ordering::Relaxed);
            (0..count).any(|index| flag(&object(&backend.get_fields(&format!("vehicles.vehicles.{index}.parameterManager"), "pendingWrites")), "pendingWrites"))
        }
    };
    let state = State { vehicle, dirty_for_save, dirty_for_upload, pending_writes };
    json!({
        "kind": "object",
        "class": "CloseChecks",
        "prompts": prompts(&state).iter().map(|(id, message)| json!({ "id": id, "message": message })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(state: &State) -> Vec<&'static str> {
        prompts(state).iter().map(|(id, _)| *id).collect()
    }

    #[test]
    fn the_checks_run_in_main_window_order() {
        let all = State { vehicle: true, dirty_for_save: true, dirty_for_upload: true, pending_writes: true };
        assert_eq!(ids(&all), ["unsavedMission", "pendingWrites", "activeConnections"]);
        assert_eq!(ids(&State { dirty_for_upload: false, ..all }), ["pendingWrites", "activeConnections"], "a plan already sent to the vehicle is not lost by closing");
        assert_eq!(ids(&State { vehicle: false, dirty_for_save: true, dirty_for_upload: false, pending_writes: false }), ["unsavedMission"], "with no vehicle an unsaved plan has nowhere else to live");
        assert!(ids(&State { vehicle: false, dirty_for_save: false, dirty_for_upload: false, pending_writes: false }).is_empty());
    }
}
