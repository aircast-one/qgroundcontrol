use serde_json::{Value, json};

use crate::router::Backend;

pub struct Facade<B>(pub B);

static FELL_THROUGH: std::sync::Mutex<std::collections::BTreeMap<String, u64>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

fn shape(path: &str) -> String {
    path.split('.').map(|part| if !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()) { "#" } else { part }).collect::<Vec<_>>().join(".")
}

fn fell_through(kind: &str, path: &str) {
    let mut tally = FELL_THROUGH.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    *tally.entry(format!("{kind} {}", shape(path))).or_default() += 1;
}

fn tally() -> Value {
    json!({ "kind": "value", "value": FELL_THROUGH.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone() })
}

fn switched_on() -> bool {
    std::env::var("QGC_CORE_VEHICLE").map_or(cfg!(not(test)), |v| v == "1")
}

struct Known {
    id: u8,
    parameters_ready: bool,
    lost: bool,
    home: Option<(f64, f64, f64)>,
    sensors: Value,
    supports: Value,
    fields: Value,
}

fn supports(autopilot: u8, vehicle_type: u8) -> Value {
    use crate::cmdinfo::VehicleClass::{FixedWing, MultiRotor, Rover, Sub, Vtol};
    let class = crate::plandoc::vehicle_class(i64::from(vehicle_type));
    let (px4, apm) = (autopilot == crate::modes::AUTOPILOT_PX4, autopilot == crate::modes::AUTOPILOT_ARDUPILOT);
    let sub = apm && class == Sub;
    let capable = |check: fn(bool, bool, crate::cmdinfo::VehicleClass) -> bool| check(px4, apm, class);
    let takeoff = !sub && capable(|px4, apm, c| (apm || px4) && matches!(c, FixedWing | MultiRotor | Vtol));
    let guided_takeoff = !sub && capable(|px4, apm, c| (apm || px4) && matches!(c, MultiRotor | Vtol));
    json!({
        "guidedMode": px4 || apm,
        "pauseVehicle": px4 || apm,
        "roiMode": !sub && (apm || (px4 && class == MultiRotor)),
        "changeHeading": ((apm || px4) && class == MultiRotor) || sub,
        "orbitMode": px4 && matches!(class, MultiRotor | Vtol | FixedWing),
        "takeoffMissionCommand": takeoff,
        "guidedTakeoffWithAltitude": guided_takeoff,
        "guidedTakeoffWithoutAltitude": takeoff && !guided_takeoff,
        "terrainFrame": !px4,
        "radio": !sub,
        "jsButton": sub,
        "motorInterference": !sub,
        "negativeThrust": (apm && matches!(class, Sub | Rover)) || (px4 && matches!(vehicle_type, 10 | 12)),
        "smartRTL": apm && matches!(class, MultiRotor | Rover),
        "throttleModeCenterZero": true,
    })
}

fn carried() -> Option<Known> {
    let hub = crate::hub::lock();
    hub.active().map(|v| {
        let ordered = v.sensors.ordered();
        let sensors = json!({ "sensorNames": ordered.names, "sensorStatus": ordered.status, "sensorEnabled": ordered.enabled, "sensorHealthy": ordered.healthy });
        let class = crate::plandoc::vehicle_class(i64::from(v.vehicle_type));
        use crate::cmdinfo::VehicleClass::{FixedWing, MultiRotor, Rover, Sub, Vtol};
        let fields = json!({
            "id": v.id,
            "armed": v.armed(),
            "flightMode": v.flight_mode(),
            "px4Firmware": v.autopilot == crate::modes::AUTOPILOT_PX4,
            "apmFirmware": v.autopilot == crate::modes::AUTOPILOT_ARDUPILOT,
            "fixedWing": class == FixedWing,
            "multiRotor": class == MultiRotor,
            "vtol": class == Vtol,
            "rover": class == Rover,
            "sub": class == Sub,
            "initialConnectComplete": v.connected,
        });
        Known { id: v.id, parameters_ready: v.parameters_ready(), lost: v.connection_lost, home: v.home, sensors, supports: supports(v.autopilot, v.vehicle_type), fields }
    })
}

fn fields_of(fields: &str) -> Vec<&str> {
    fields.split(',').map(str::trim).filter(|f| !f.is_empty()).collect()
}

fn answer_fields(path: &str, fields: &str, known: &Known) -> Option<Value> {
    let asked = fields_of(fields);
    let field = |name: &str| -> Option<Value> {
        match (path, name) {
            ("vehicles", "activeVehicleAvailable") => Some(json!(true)),
            ("vehicle.parameterManager", "parametersReady") => Some(json!(known.parameters_ready)),
            ("vehicle.vehicleLinkManager", "communicationLost") => Some(json!(known.lost)),
            ("vehicle.supports", capability) => known.supports.get(capability).cloned(),
            ("vehicle", field) => known.fields.get(field).cloned(),
            _ => None,
        }
    };
    let answered: Option<serde_json::Map<String, Value>> = asked.iter().map(|name| field(name).map(|v| (name.to_string(), v))).collect();
    let mut object = Value::Object(answered.filter(|a| !a.is_empty())?);
    object["kind"] = json!("object");
    Some(object)
}

fn answer_get(path: &str, known: &Known) -> Option<Value> {
    let value = match path {
        "vehicle.id" => json!(known.id),
        "vehicle.parameterManager.parametersReady" => json!(known.parameters_ready),
        "vehicle.vehicleLinkManager.communicationLost" => json!(known.lost),
        "vehicle.sysStatusSensorInfo" => {
            let mut object = json!({ "children": [], "class": "SysStatusSensorInfo", "facts": [], "kind": "object", "objectName": "" });
            known.sensors.as_object()?.iter().for_each(|(k, v)| object[k.as_str()] = v.clone());
            return Some(object);
        }
        field if field.starts_with("vehicle.") && !field["vehicle.".len()..].contains('.') && known.fields.get(&field["vehicle.".len()..]).is_some() => known.fields[&field["vehicle.".len()..]].clone(),
        capability if capability.starts_with("vehicle.supports.") => known.supports.get(capability.trim_start_matches("vehicle.supports."))?.clone(),
        sensor if sensor.starts_with("vehicle.sysStatusSensorInfo.") => known.sensors.get(sensor.trim_start_matches("vehicle.sysStatusSensorInfo."))?.clone(),
        "vehicle.homePosition" => {
            let (latitude, longitude, altitude) = known.home?;
            return Some(json!({ "kind": "coordinate", "latitude": latitude, "longitude": longitude, "altitude": altitude, "valid": true }));
        }
        _ => return None,
    };
    Some(json!({ "kind": "value", "value": value }))
}

fn answer_invoke(path: &str, args: &str) -> Option<Value> {
    (path == "vehicle.parameterManager.parameterExists").then_some(())?;
    let given: Value = serde_json::from_str(args).ok()?;
    let name = given.get(1)?.as_str()?;
    let component = given.get(0)?.as_i64()?;
    let hub = crate::hub::lock();
    let vehicle = hub.active()?;
    let component = u8::try_from(component).unwrap_or(vehicle.component);
    Some(json!({ "ok": true, "result": vehicle.parameter(component, name).is_some() }))
}

impl<B: Backend> Backend for Facade<B> {
    fn get(&self, path: &str) -> String {
        if path == "core.qtReads" {
            return tally().to_string();
        }
        switched_on().then(carried).flatten().and_then(|known| answer_get(path, &known)).map_or_else(
            || {
                fell_through("get", path);
                self.0.get(path)
            },
            |v| v.to_string(),
        )
    }
    fn get_fields(&self, path: &str, fields: &str) -> String {
        switched_on().then(carried).flatten().and_then(|known| answer_fields(path, fields, &known)).map_or_else(
            || {
                fell_through("fields", &format!("{path} [{fields}]"));
                self.0.get_fields(path, fields)
            },
            |v| v.to_string(),
        )
    }
    fn set(&self, path: &str, value: &str) -> String {
        fell_through("set", path);
        self.0.set(path, value)
    }
    fn invoke(&self, path: &str, args: &str) -> String {
        switched_on().then(|| answer_invoke(path, args)).flatten().map_or_else(
            || {
                fell_through("invoke", path);
                self.0.invoke(path, args)
            },
            |v| v.to_string(),
        )
    }
    fn watch(&self, paths: &[String]) {
        self.0.watch(paths);
    }
    fn core_guided(&self, action: &Value) -> Option<Result<(), String>> {
        self.0.core_guided(action)
    }
    fn remember_setting(&self, key: &str, value: &Value) {
        self.0.remember_setting(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fields_the_hub_knows_are_answered_and_the_rest_fall_through() {
        let known = Known { id: 1, parameters_ready: true, lost: false, home: None, sensors: json!({ "sensorNames": ["GPS"] }), supports: supports(3, 2), fields: json!({ "armed": false }) };
        assert_eq!(answer_fields("vehicles", "activeVehicleAvailable", &known), Some(json!({ "kind": "object", "activeVehicleAvailable": true })));
        assert_eq!(answer_fields("vehicles", "activeVehicleAvailable,activeVehicle", &known), None, "one unknown field sends the whole read to the host");
        assert_eq!(answer_get("vehicle.id", &known), Some(json!({ "kind": "value", "value": 1 })));
        assert_eq!(answer_get("vehicle.flying", &known), None);
        assert_eq!(answer_get("vehicle.armed", &known), Some(json!({ "kind": "value", "value": false })));
        assert_eq!(answer_fields("vehicle.supports", "guidedTakeoffWithAltitude,orbitMode,smartRTL", &known), Some(json!({ "kind": "object", "guidedTakeoffWithAltitude": true, "orbitMode": false, "smartRTL": true })), "an ArduCopter quad");
        assert_eq!(supports(12, 1)["guidedTakeoffWithoutAltitude"], true, "a PX4 plane takes off without an altitude");
        assert_eq!(answer_get("vehicle.sysStatusSensorInfo.sensorNames", &known), Some(json!({ "kind": "value", "value": ["GPS"] })));
    }
}
