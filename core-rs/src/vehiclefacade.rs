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

fn mode_fields(autopilot: u8, vehicle_type: u8, available: &[crate::standardmodes::FlightMode]) -> serde_json::Map<String, Value> {
    use crate::modes::{AUTOPILOT_ARDUPILOT, AUTOPILOT_PX4, VehicleClass, px4};
    let listed: Vec<(String, u32, bool, bool)> = if available.is_empty() {
        crate::modes::table(autopilot, vehicle_type).iter().map(|m| (m.name.to_string(), m.custom_mode, m.can_be_set, m.advanced)).collect()
    } else {
        available.iter().map(|m| (m.name.clone(), m.custom_mode, m.can_be_set, m.advanced)).collect()
    };
    let named = |custom: Option<u32>, fallback: &str| custom.and_then(|c| listed.iter().find(|m| m.1 == c)).map_or_else(|| fallback.to_string(), |m| m.0.clone());
    let names = |advanced_only: bool| listed.iter().filter(|m| m.2 && (!advanced_only || m.3)).map(|m| m.0.clone()).collect::<Vec<_>>();
    let apm = |rtl: Option<u32>, smart: Option<u32>, mission: Option<u32>, land: (Option<u32>, &str), pause: (Option<u32>, &str)| {
        json!({
            "flightModes": names(false),
            "advancedFlightModes": names(true),
            "flightModeSetAvailable": true,
            "rtlFlightMode": named(rtl, "RTL"),
            "smartRTLFlightMode": named(smart, "Smart RTL"),
            "missionFlightMode": named(mission, "Auto"),
            "landFlightMode": named(land.0, land.1),
            "pauseFlightMode": named(pause.0, pause.1),
        })
    };
    let fields = match (autopilot, crate::modes::vehicle_class(vehicle_type)) {
        (AUTOPILOT_ARDUPILOT, VehicleClass::MultiRotor) => apm(Some(6), Some(21), Some(3), (Some(9), "Land"), (Some(17), "Brake")),
        (AUTOPILOT_ARDUPILOT, VehicleClass::FixedWing) => apm(Some(11), Some(11), Some(10), (None, ""), (Some(12), "Loiter")),
        (AUTOPILOT_ARDUPILOT, VehicleClass::Rover) => apm(Some(11), Some(12), Some(10), (None, ""), (Some(4), "Hold")),
        (AUTOPILOT_ARDUPILOT, VehicleClass::Sub) => apm(None, None, Some(3), (None, ""), (None, "")),
        (AUTOPILOT_PX4, _) => json!({
            "advancedFlightModes": names(true),
            "rtlFlightMode": named(Some(px4(4, 5)), ""),
            "smartRTLFlightMode": "",
            "missionFlightMode": named(Some(px4(4, 4)), ""),
            "landFlightMode": named(Some(px4(4, 6)), ""),
            "pauseFlightMode": named(Some(px4(4, 3)), ""),
        }),
        _ => json!({}),
    };
    fields.as_object().cloned().unwrap_or_default()
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
            "flying": v.flying,
            "landing": v.landing,
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
        let fields = Value::Object(fields.as_object().cloned().unwrap_or_default().into_iter().chain(mode_fields(v.autopilot, v.vehicle_type, &v.flight_modes)).collect());
        Known { id: v.id, parameters_ready: v.parameters_ready(), lost: v.connection_lost, home: v.home, sensors, supports: supports(v.autopilot, v.vehicle_type), fields }
    })
}

fn fields_of(fields: &str) -> Vec<&str> {
    fields.split(',').map(str::trim).filter(|f| !f.is_empty()).collect()
}

fn answer_fields(path: &str, fields: &str, known: &Known) -> (serde_json::Map<String, Value>, Vec<String>) {
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
    let (answered, missing): (Vec<_>, Vec<_>) = fields_of(fields).into_iter().map(|name| (name, field(name))).partition(|(_, v)| v.is_some());
    (answered.into_iter().filter_map(|(name, v)| Some((name.to_string(), v?))).collect(), missing.into_iter().map(|(name, _)| name.to_string()).collect())
}

fn merged(answered: serde_json::Map<String, Value>, host: String) -> String {
    serde_json::from_str::<Value>(&host).ok().filter(|h| h["kind"] == "object").map_or(host, |mut h| {
        answered.into_iter().for_each(|(k, v)| h[k.as_str()] = v);
        h.to_string()
    })
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
        let (answered, missing) = switched_on().then(carried).flatten().map_or_else(|| (serde_json::Map::new(), fields_of(fields).into_iter().map(String::from).collect()), |known| answer_fields(path, fields, &known));
        if missing.is_empty() && !answered.is_empty() {
            let mut object = Value::Object(answered);
            object["kind"] = json!("object");
            return object.to_string();
        }
        let asked = if answered.is_empty() { fields.to_string() } else { missing.join(",") };
        fell_through("fields", &format!("{path} [{asked}]"));
        merged(answered, self.0.get_fields(path, &asked))
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
    fn mode_names_come_from_the_firmware_class_and_what_the_vehicle_announced() {
        let copter = mode_fields(3, 2, &[]);
        assert_eq!((copter["pauseFlightMode"].as_str(), copter["landFlightMode"].as_str(), copter["missionFlightMode"].as_str()), (Some("Brake"), Some("Land"), Some("Auto")));
        assert!(copter["advancedFlightModes"].as_array().unwrap().contains(&json!("Brake")));
        assert!(!copter["advancedFlightModes"].as_array().unwrap().contains(&json!("Loiter")));
        let plane = mode_fields(3, 1, &[]);
        assert!(!plane["flightModes"].as_array().unwrap().contains(&json!("Initializing")), "a mode the vehicle only reports is never offered");
        assert_eq!((plane["smartRTLFlightMode"].as_str(), plane["landFlightMode"].as_str()), (Some("RTL"), Some("")), "a plane's smart RTL is its RTL and it has no land mode");
        assert_eq!(mode_fields(3, 12, &[])["rtlFlightMode"], "RTL", "a sub has no RTL number, so the name falls back");
        let renamed = crate::standardmodes::FlightMode { name: "Return Home".into(), standard_mode: 0, custom_mode: 6, can_be_set: true, advanced: false, fixed_wing: false, multi_rotor: true };
        let announced = mode_fields(3, 2, &[renamed]);
        assert_eq!((announced["rtlFlightMode"].as_str(), announced["pauseFlightMode"].as_str()), (Some("Return Home"), Some("Brake")), "an announced list replaces the table and a missing mode falls back");
        assert!(mode_fields(12, 2, &[]).get("flightModes").is_none(), "PX4's list filters by airframe flags the core does not keep yet");
        assert!(mode_fields(0, 2, &[]).is_empty());
    }

    #[test]
    fn only_fields_the_hub_knows_are_answered_and_the_rest_fall_through() {
        let known = Known { id: 1, parameters_ready: true, lost: false, home: None, sensors: json!({ "sensorNames": ["GPS"] }), supports: supports(3, 2), fields: json!({ "armed": false }) };
        assert_eq!(answer_fields("vehicles", "activeVehicleAvailable,activeVehicle", &known), (json!({ "activeVehicleAvailable": true }).as_object().unwrap().clone(), vec!["activeVehicle".to_string()]), "only the unknown field goes to the host");
        assert_eq!(merged(json!({ "armed": false }).as_object().unwrap().clone(), json!({ "kind": "object", "rcRSSI": 255 }).to_string()), json!({ "kind": "object", "rcRSSI": 255, "armed": false }).to_string());
        assert_eq!(merged(json!({ "armed": false }).as_object().unwrap().clone(), json!({ "kind": "null" }).to_string()), json!({ "kind": "null" }).to_string(), "a host with no such object keeps its answer");
        assert_eq!(answer_get("vehicle.id", &known), Some(json!({ "kind": "value", "value": 1 })));
        assert_eq!(answer_get("vehicle.flying", &known), None);
        assert_eq!(answer_get("vehicle.armed", &known), Some(json!({ "kind": "value", "value": false })));
        assert_eq!(answer_fields("vehicle.supports", "guidedTakeoffWithAltitude,orbitMode,smartRTL", &known).0, json!({ "guidedTakeoffWithAltitude": true, "orbitMode": false, "smartRTL": true }).as_object().unwrap().clone(), "an ArduCopter quad");
        assert_eq!(supports(12, 1)["guidedTakeoffWithoutAltitude"], true, "a PX4 plane takes off without an altitude");
        assert_eq!(answer_get("vehicle.sysStatusSensorInfo.sensorNames", &known), Some(json!({ "kind": "value", "value": ["GPS"] })));
    }
}
