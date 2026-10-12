use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "settings.appSettings.virtualJoystick",
    "settings.appSettings.virtualJoystickAutoCenterThrottle",
    "settings.appSettings.virtualJoystickLeftHandedMode",
    "vehicles.activeVehicleAvailable",
    "vehicle.rover",
    "vehicle.initialConnectComplete",
];

pub const JOYSTICK_VALUE: &str = "vehicle.virtualTabletJoystickValue";
pub const SEND_PERIOD_MS: u64 = 40;
pub const TAKEOVER_DEFLECTION: f64 = 0.3;
pub const TAKEOVER_HOLD_MS: u64 = 300;
pub const TAKEOVER_RETRY_MS: u64 = 5000;
pub const YAW_EXPO_PCT: f64 = 50.0;
const THROTTLE_CENTRE: f64 = 0.5;

pub fn stick_deflected(roll: f64, pitch: f64, yaw: f64, thrust: f64, flying: bool) -> bool {
    [roll, pitch, yaw].iter().any(|axis| axis.abs() > TAKEOVER_DEFLECTION)
        || thrust - THROTTLE_CENTRE > TAKEOVER_DEFLECTION
        || (flying && THROTTLE_CENTRE - thrust > TAKEOVER_DEFLECTION)
}

fn setting(backend: &dyn Backend, name: &str) -> bool {
    backend.value(&format!("settings.appSettings.{name}")).get("value").and_then(Value::as_bool).unwrap_or(false)
}

fn sending(backend: &dyn Backend) -> bool {
    let vehicles = backend.value_fields("vehicles", "activeVehicleAvailable");
    flag(&vehicles, "activeVehicleAvailable") && flag(&backend.value_fields("vehicle", "initialConnectComplete"), "initialConnectComplete")
}

pub fn virtual_joystick_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = backend.value_fields("vehicle", "rover");
    let connected = flag(&backend.value_fields("vehicles", "activeVehicleAvailable"), "activeVehicleAvailable");
    let rover = connected && flag(&vehicle, "rover");
    let left_handed = setting(backend, "virtualJoystickLeftHandedMode");
    json!({
        "kind": "object",
        "class": "VirtualJoystick",
        "show": setting(backend, "virtualJoystick"),
        "sending": sending(backend),
        "autoCenterThrottle": setting(backend, "virtualJoystickAutoCenterThrottle"),
        "leftHandedMode": left_handed,
        "leftPositiveOnly": connected && !rover && !left_handed,
        "rightPositiveOnly": connected && !rover && left_handed,
        "periodMs": SEND_PERIOD_MS,
    })
}

pub fn send(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let axis = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.is_finite()).map(|v| v.clamp(-1.0, 1.0));
    let (Some(roll), Some(pitch), Some(yaw), Some(thrust)) = (axis(0), axis(1), axis(2), axis(3)) else {
        return json!({ "ok": false, "reason": "The virtual joystick sends roll, pitch, yaw and thrust, each from -1 to 1." });
    };
    if !sending(backend) {
        return json!({ "ok": false, "reason": "No vehicle has finished connecting." });
    }
    let active = crate::hub::lock().active_id();
    if active.is_some_and(crate::joystickhost::drives) {
        return json!({ "ok": false, "reason": "A joystick is enabled for this vehicle, so the on-screen sticks do not send." });
    }
    let action = json!({ "action": "virtualJoystick", "roll": roll, "pitch": pitch, "yaw": yaw, "thrust": thrust });
    let dispatched = crate::rcoverride::on_core(backend, action).unwrap_or_else(|| flag(&object(&backend.invoke(JOYSTICK_VALUE, &json!([roll, pitch, yaw, thrust]).to_string())), "ok"));
    json!({ "ok": dispatched })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stick_pushed_well_off_centre_is_the_pilot_taking_over() {
        assert!(!stick_deflected(0.0, 0.0, 0.0, 0.5, true), "sticks at rest");
        assert!(!stick_deflected(0.2, -0.25, 0.1, 0.7, true), "small corrections and a brush of the throttle do not count");
        assert!(stick_deflected(0.0, -0.6, 0.0, 0.5, true), "pitch forward");
        assert!(stick_deflected(0.0, 0.0, 0.5, 0.5, true), "a turn");
        assert!(stick_deflected(0.0, 0.0, 0.0, 0.95, false), "climbing");
        assert!(stick_deflected(0.0, 0.0, 0.0, 0.0, true), "pulling the throttle down to descend");
        assert!(!stick_deflected(0.0, 0.0, 0.0, 0.0, false), "on the ground the throttle rests at the bottom");
    }
    use std::cell::RefCell;

    struct Rig {
        rover: bool,
        left_handed: bool,
        connected: bool,
        invoked: RefCell<Vec<String>>,
    }

    impl Backend for Rig {
        fn get(&self, path: &str) -> String {
            let value = match path {
                "settings.appSettings.virtualJoystick" | "settings.appSettings.virtualJoystickAutoCenterThrottle" => true,
                "settings.appSettings.virtualJoystickLeftHandedMode" => self.left_handed,
                _ => false,
            };
            json!({ "kind": "fact", "value": value }).to_string()
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicles" => json!({ "activeVehicleAvailable": self.connected }),
                _ => json!({ "rover": self.rover, "initialConnectComplete": self.connected }),
            }
            .to_string()
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }
        fn invoke(&self, path: &str, args: &str) -> String {
            self.invoked.borrow_mut().push(format!("{path} {args}"));
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _paths: &[String]) {}
    }

    fn rig(rover: bool, left_handed: bool, connected: bool) -> Rig {
        Rig { rover, left_handed, connected, invoked: RefCell::new(Vec::new()) }
    }

    #[test]
    fn the_throttle_stick_runs_zero_to_one_except_on_a_rover() {
        let right_handed = virtual_joystick_view(&rig(false, false, true), &[]);
        assert_eq!((right_handed["leftPositiveOnly"].as_bool(), right_handed["rightPositiveOnly"].as_bool()), (Some(true), Some(false)));
        let left_handed = virtual_joystick_view(&rig(false, true, true), &[]);
        assert_eq!((left_handed["leftPositiveOnly"].as_bool(), left_handed["rightPositiveOnly"].as_bool()), (Some(false), Some(true)));
        let rover = virtual_joystick_view(&rig(true, false, true), &[]);
        assert_eq!(rover["leftPositiveOnly"], json!(false));
        assert_eq!(right_handed["show"], json!(true));
    }

    #[test]
    fn values_go_to_the_vehicle_only_once_it_has_connected() {
        let up = rig(false, false, true);
        assert_eq!(send(&up, "[0.1, 0.2, 0.3, 0.9]")["ok"], json!(true));
        assert_eq!(up.invoked.borrow().as_slice(), ["vehicle.virtualTabletJoystickValue [0.1,0.2,0.3,0.9]"]);
        let down = rig(false, false, false);
        assert_eq!(send(&down, "[0, 0, 0, 0]")["ok"], json!(false));
        assert!(down.invoked.borrow().is_empty());
        assert_eq!(send(&up, "[0, 0]")["ok"], json!(false));
    }
}
