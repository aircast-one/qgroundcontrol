use groundstation::hub::Origin;
use groundstation::router::{Backend, Core};
use groundstation::settingsstore::Owner;
use groundstation::vehiclefacade::Facade;
use mavlink::MavHeader;
use mavlink::dialects::ardupilotmega::{HEARTBEAT_DATA, MavAutopilot, MavMessage, MavType};
use serde_json::{Value, json};

struct NoHost;

impl Backend for NoHost {
    fn get(&self, _path: &str) -> String {
        json!({ "kind": "null" }).to_string()
    }
    fn get_fields(&self, _path: &str, _fields: &str) -> String {
        json!({ "kind": "null" }).to_string()
    }
    fn set(&self, _path: &str, _value: &str) -> String {
        json!({ "ok": false, "reason": "no host" }).to_string()
    }
    fn invoke(&self, _path: &str, _args: &str) -> String {
        json!({ "ok": false, "reason": "no host" }).to_string()
    }
    fn watch(&self, _paths: &[String]) {}
}

fn heartbeat(autopilot: MavAutopilot) -> MavMessage {
    MavMessage::HEARTBEAT(HEARTBEAT_DATA { mavtype: MavType::MAV_TYPE_QUADROTOR, autopilot, ..HEARTBEAT_DATA::default() })
}

fn arrives(link: u32, system_id: u8, autopilot: MavAutopilot) {
    groundstation::hub::lock().on_frame(Origin { link, replay: false, v2: true }, &MavHeader { system_id, component_id: 1, sequence: 0 }, &heartbeat(autopilot), 0, 0);
}

fn read(core: &Core<Owner<Facade<NoHost>>>, path: &str) -> Value {
    serde_json::from_str(&core.get(path)).unwrap_or(Value::Null)
}

fn waypoint(core: &Core<Owner<Facade<NoHost>>>, latitude: f64) -> Value {
    serde_json::from_str(&core.invoke("mission.insert", &json!(["waypoint", latitude, 8.546, -1]).to_string())).unwrap_or(Value::Null)
}

const PX4_PLAN: &str = r#"{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{"version":2,"firmwareType":12,"vehicleType":2,"cruiseSpeed":15,"hoverSpeed":5,"plannedHomePosition":[47.0,8.0,500],"items":[{"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":1,"frame":3,"params":[0,0,0,null,47.001,8.0,50]}]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;

#[test]
fn a_new_plan_takes_the_firmware_of_the_vehicle_it_is_made_on() {
    let scratch = std::env::temp_dir().join(format!("plan-vehicle-switch-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let (settings, px4_plan) = (scratch.join("settings.ini"), scratch.join("px4.plan"));
    std::fs::write(&px4_plan, PX4_PLAN).unwrap();
    groundstation::settingsstore::open(&settings);
    let core = Core::new(Owner(Facade(NoHost)));

    arrives(1, 128, MavAutopilot::MAV_AUTOPILOT_PX4);
    assert_eq!(read(&core, "view.plan")["planningFor"]["firmware"], "PX4 Pro");
    groundstation::hub::lock().link_closed(1);
    assert_eq!(read(&core, "view.plan")["upload"]["state"], 1, "no vehicle is left to upload to");

    arrives(2, 129, MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA);
    assert_eq!(waypoint(&core, 47.397)["ok"], true);
    assert_eq!(waypoint(&core, 47.398)["ok"], true);
    let view = read(&core, "view.plan");
    assert_eq!(view["planningFor"]["firmware"], "ArduPilot");
    assert_eq!(view["upload"]["state"], 0, "PlanMasterController::_activeVehicleChanged moves the offline classes to the new vehicle at once, and the empty plan's controller vehicle follows them until its first item locks them: {}", view["upload"]);

    assert_eq!(serde_json::from_str::<Value>(&core.invoke("plan.loadFromFile", &json!([px4_plan]).to_string())).unwrap()["ok"], true);
    assert_eq!(read(&core, "view.plan")["upload"]["state"], 2, "a loaded plan keeps the firmware it was made for");
    assert_eq!(serde_json::from_str::<Value>(&core.invoke("plan.removeAll", "[]")).unwrap()["ok"], true);
    assert_eq!(waypoint(&core, 47.399)["ok"], true);
    let view = read(&core, "view.plan");
    assert_eq!(view["upload"]["state"], 0, "MissionController::removeAll calls _allItemsRemoved, so the blank plan tracks the offline classes again: {}", view["upload"]);
    let _ = std::fs::remove_dir_all(scratch);
}
