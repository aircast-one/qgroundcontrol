use groundstation::hub::Origin;
use groundstation::router::{Backend, Core};
use groundstation::settingsstore::Owner;
use groundstation::vehiclefacade::Facade;
use mavlink::MavHeader;
use mavlink::dialects::ardupilotmega::{
    HEARTBEAT_DATA, HOME_POSITION_DATA, MavAutopilot, MavMessage, MavType,
};
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

const HOME: (f64, f64) = (47.397, 8.546);

fn arrives(message: &MavMessage) {
    groundstation::hub::lock().on_frame(
        Origin {
            link: 1,
            replay: false,
            v2: true,
        },
        &MavHeader {
            system_id: 1,
            component_id: 1,
            sequence: 0,
        },
        message,
        0,
        0,
    );
}

fn coordinate(item: &Value) -> (f64, f64) {
    (
        item["coordinate"]["latitude"].as_f64().unwrap_or(f64::NAN),
        item["coordinate"]["longitude"].as_f64().unwrap_or(f64::NAN),
    )
}

#[test]
fn the_first_tap_on_an_empty_plan_launches_from_the_connected_vehicle_home() {
    let scratch = std::env::temp_dir().join(format!("plan-vehicle-home-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    groundstation::settingsstore::open(&scratch.join("settings.ini"));
    let core = Core::new(Owner(Facade(NoHost)));

    arrives(&MavMessage::HEARTBEAT(HEARTBEAT_DATA {
        mavtype: MavType::MAV_TYPE_QUADROTOR,
        autopilot: MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
        ..HEARTBEAT_DATA::default()
    }));
    arrives(&MavMessage::HOME_POSITION(HOME_POSITION_DATA {
        latitude: (HOME.0 * 1e7) as i32,
        longitude: (HOME.1 * 1e7) as i32,
        altitude: 488_000,
        ..HOME_POSITION_DATA::default()
    }));

    let tapped: Value = serde_json::from_str(&core.invoke(
        "mission.insert",
        &json!(["waypoint", HOME.0, HOME.1, -1]).to_string(),
    ))
    .unwrap();
    assert_eq!(tapped["ok"], true, "{tapped}");

    let items: Value =
        serde_json::from_str(&core.get("view.missionItems(geometry)")).unwrap_or(Value::Null);
    let (latitude, longitude) = coordinate(&items["items"][0]);
    assert!(
        (latitude - HOME.0).abs() < 1e-6 && (longitude - HOME.1).abs() < 1e-6,
        "TakeoffMissionItem copies the active vehicle's home into an unset planned home, so H sits under the vehicle, not 30 m north of the tap: {:?}",
        (latitude, longitude)
    );
    let _ = std::fs::remove_dir_all(scratch);
}
