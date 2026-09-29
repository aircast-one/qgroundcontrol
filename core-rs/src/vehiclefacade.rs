use serde_json::{Value, json};

use crate::router::Backend;

pub struct Facade<B>(pub B);

const VEHICLE_FACTS: [&str; 1] = ["rcRSSI"];

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
    coordinate: Option<(f64, f64, f64)>,
    batteries: Vec<(u8, crate::batteryfacts::BatteryFacts)>,
    gps: crate::gpsfacts::GpsFacts,
    vibration: crate::vehiclefact::VibrationFacts,
    estimator: crate::sensorfacts::EstimatorStatusFacts,
    distance: crate::sensorfacts::DistanceSensorFacts,
    capabilities: Option<u64>,
    radio: crate::vehiclefact::RadioStatusFacts,
    obstacle: crate::vehiclefact::ObstacleFacts,
    avoidance_enabled: bool,
    temperature: crate::sensorfacts::TemperatureFacts,
    local: crate::sensorfacts::LocalPositionFacts,
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

fn mav_type_text(mav_type: u8) -> &'static str {
    match mav_type {
        0 => "Generic micro air vehicle",
        1 => "Fixed wing aircraft",
        2 => "Quadrotor",
        3 => "Coaxial helicopter",
        4 => "Normal helicopter with tail rotor.",
        5 => "Ground installation",
        6 => "Operator control unit / ground control station",
        7 => "Airship, controlled",
        8 => "Free balloon, uncontrolled",
        9 => "Rocket",
        10 => "Ground rover",
        11 => "Surface vessel, boat, ship",
        12 => "Submarine",
        13 => "Hexarotor",
        14 => "Octorotor",
        15 => "trirotor",
        16 => "Flapping wing",
        17 => "Kite",
        18 => "Onboard companion controller",
        19 => "Two-rotor VTOL using control surfaces in vertical operation in addition. Tailsitter",
        20 => "Quad-rotor VTOL using a V-shaped quad config in vertical operation. Tailsitter",
        21 => "Tiltrotor VTOL",
        22 => "VTOL Fixedrotor",
        23 => "VTOL Tailsitter",
        24 => "VTOL Tiltwing",
        25 => "VTOL reserved 5",
        26 => "Onboard gimbal",
        27 => "Onboard ADSB peripheral",
        45 => "Spacecraft, orbiter",
        _ => "MAV_TYPE_UNKNOWN",
    }
}

fn motor_count(mav_type: u8, sub_frame: Option<f64>) -> Option<i64> {
    Some(match mav_type {
        4 => 1,
        19 => 2,
        15 => 3,
        2 | 20 => 4,
        13 => 6,
        14 | 45 => 8,
        12 => match sub_frame? as i64 {
            0 | 1 => 6,
            4 => 3,
            5 => 4,
            6 => 5,
            2 | 3 | 7 => 8,
            _ => -1,
        },
        _ => -1,
    })
}

fn firmware_fields(autopilot: u8, firmware: Option<crate::connect::Firmware>) -> Value {
    let version = firmware.and_then(|f| f.version);
    let part = |pick: fn((u8, u8, u8, u8)) -> u8| version.map_or(-1, |v| i64::from(pick(v)));
    json!({
        "firmwareTypeString": match autopilot {
            crate::modes::AUTOPILOT_PX4 => "PX4 Pro",
            crate::modes::AUTOPILOT_ARDUPILOT => "ArduPilot",
            _ => "Generic",
        },
        "firmwareMajorVersion": part(|v| v.0),
        "firmwareMinorVersion": part(|v| v.1),
        "firmwarePatchVersion": part(|v| v.2),
        "firmwareVersionTypeString": match version.map(|v| v.3) {
            Some(0) => "dev",
            Some(64) => "alpha",
            Some(128) => "beta",
            Some(192) => "rc",
            _ => "",
        },
    })
}

fn has_gripper(autopilot: u8, parameter: impl Fn(&str) -> Option<f64>) -> bool {
    match autopilot {
        crate::modes::AUTOPILOT_ARDUPILOT => parameter("GRIP_ENABLE") == Some(1.0),
        crate::modes::AUTOPILOT_PX4 => parameter("PD_GRIPPER_EN").map(|v| v != 0.0).or_else(|| parameter("PD_GRIPPER_TYPE").map(|v| v >= 0.0)).unwrap_or(false),
        _ => false,
    }
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
            "vtolInFwdFlight": v.vtol_in_forward_flight,
            "hasGripper": has_gripper(v.autopilot, |name| v.parameter(v.component, name).map(|p| p.as_f64())),
            "readyToFlyAvailable": v.status_bits.ready_to_fly_available,
            "readyToFly": v.status_bits.ready_to_fly,
            "allSensorsHealthy": v.status_bits.all_healthy,
            "requiresGpsFix": v.status_bits.requires_gps_fix(),
            "sensorsUnhealthyBits": v.status_bits.unhealthy(),
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
            "rcRSSI": crate::vehiclefact::vehicle_fact("rcRSSI", &json!(v.rc_rssi.shown)),
            "latitude": v.facts.coordinate.map(|(latitude, _, _)| f64::from(latitude as f32)),
            "longitude": v.facts.coordinate.map(|(_, longitude, _)| f64::from(longitude as f32)),
        });
        let described = json!({
            "vehicleTypeString": mav_type_text(v.vehicle_type),
            "airship": v.vehicle_type == 7,
        });
        let motors = motor_count(v.vehicle_type, v.parameter(v.component, "FRAME_CONFIG").map(|p| p.as_f64())).map(|count| ("motorCount".to_string(), json!(count)));
        let object = |value: Value| value.as_object().cloned().unwrap_or_default();
        let prearm = (v.autopilot == crate::modes::AUTOPILOT_ARDUPILOT).then(|| ("prearmError".to_string(), json!(v.prearm_error(crate::hub::now_ms()))));
        let fields = Value::Object(
            object(fields)
                .into_iter()
                .chain(prearm)
                .into_iter()
                .chain(object(described))
                .chain(object(firmware_fields(v.autopilot, v.firmware())))
                .chain(motors)
                .chain(mode_fields(v.autopilot, v.vehicle_type, &v.flight_modes))
                .collect(),
        );
        Known { id: v.id, parameters_ready: v.parameters_ready(), lost: v.connection_lost, home: v.home, coordinate: v.facts.coordinate, batteries: v.batteries.by_id.iter().map(|(id, b)| (*id, b.clone())).collect(), gps: v.gps.clone(), vibration: v.vibration.clone(), estimator: v.estimator.clone(), distance: v.distance.clone(), capabilities: v.capabilities_known.then_some(v.capabilities), radio: v.radio.clone(), obstacle: v.obstacle.clone(), avoidance_enabled: v.parameter(v.component, "CP_DIST").is_some_and(|p| p.as_f64() >= 0.0), temperature: v.temperature.clone(), local: v.local.clone(), sensors, supports: supports(v.autopilot, v.vehicle_type), fields }
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
            ("plan.managerVehicle", "capabilitiesKnown") => Some(json!(known.capabilities.is_some())),
            ("plan.geoFenceController", "supported") => Some(json!(known.capabilities.unwrap_or(0) & crate::connect::CAP_MISSION_FENCE != 0)),
            ("plan.rallyPointController", "supported") => Some(json!(known.capabilities.unwrap_or(0) & crate::connect::CAP_MISSION_RALLY != 0)),
            ("vehicle", "coordinate") => Some(known.coordinate.map_or(Value::Null, |(latitude, longitude, altitude)| json!({ "valid": true, "latitude": latitude, "longitude": longitude, "altitude": altitude }))),
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
        "vehicle.rcRSSI" => return known.fields.get("rcRSSI").cloned(),
        field if field.starts_with("vehicle.") && !field["vehicle.".len()..].contains('.') && known.fields.get(&field["vehicle.".len()..]).is_some() => known.fields[&field["vehicle.".len()..]].clone(),
        capability if capability.starts_with("vehicle.supports.") => known.supports.get(capability.trim_start_matches("vehicle.supports."))?.clone(),
        sensor if sensor.starts_with("vehicle.sysStatusSensorInfo.") => known.sensors.get(sensor.trim_start_matches("vehicle.sysStatusSensorInfo."))?.clone(),
        "vehicle.estimatorStatus" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::ESTIMATOR, |n| crate::vehiclefact::estimator_raw(&known.estimator, n), known.estimator.seen)),
        estimator if estimator.starts_with("vehicle.estimatorStatus.") => {
            return crate::vehiclefact::spec_property(&crate::vehiclefact::ESTIMATOR, &estimator["vehicle.estimatorStatus.".len()..], |n| crate::vehiclefact::estimator_raw(&known.estimator, n));
        }
        "vehicle.distanceSensors" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::DISTANCE, |n| crate::vehiclefact::distance_raw(&known.distance, n, &Value::Null), known.distance.seen)),
        distance if distance.starts_with("vehicle.distanceSensors.") => {
            return crate::vehiclefact::spec_property(&crate::vehiclefact::DISTANCE, &distance["vehicle.distanceSensors.".len()..], |n| crate::vehiclefact::distance_raw(&known.distance, n, &Value::Null));
        }
        "vehicle.radioStatus" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::RADIO, |n| known.radio.raw(n), known.radio.telemetry)),
        radio if radio.starts_with("vehicle.radioStatus.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::RADIO, &radio["vehicle.radioStatus.".len()..], |n| known.radio.raw(n)),
        "vehicle.temperature" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::TEMPERATURE, |n| crate::vehiclefact::temperature_raw(&known.temperature, n), known.temperature.seen.iter().any(|s| *s))),
        temperature if temperature.starts_with("vehicle.temperature.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::TEMPERATURE, &temperature["vehicle.temperature.".len()..], |n| crate::vehiclefact::temperature_raw(&known.temperature, n)),
        "vehicle.localPosition" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::LOCAL_POSITION, |n| crate::vehiclefact::local_position_raw(&known.local, n), known.local.seen)),
        local if local.starts_with("vehicle.localPosition.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::LOCAL_POSITION, &local["vehicle.localPosition.".len()..], |n| crate::vehiclefact::local_position_raw(&known.local, n)),
        "vehicle.vibration" => return Some(crate::vehiclefact::vibration_group(&known.vibration)),
        vibration if vibration.starts_with("vehicle.vibration.") => return crate::vehiclefact::vibration_fact(&known.vibration, &vibration["vehicle.vibration.".len()..], None),
        gps if gps.starts_with("vehicle.gps.") => return crate::vehiclefact::gps_fact(&known.gps, &gps["vehicle.gps.".len()..]),
        "vehicle.batteries" => return Some(crate::vehiclefact::battery_list(&known.batteries)),
        "vehicle.batteries.count" => json!(known.batteries.len()),
        battery if battery.starts_with("vehicle.batteries.") => {
            let mut parts = battery["vehicle.batteries.".len()..].splitn(2, '.');
            let (id, facts) = known.batteries.get(parts.next()?.parse::<usize>().ok()?)?;
            return match parts.next() {
                None => Some(crate::vehiclefact::battery_group(*id, facts)),
                Some(name) => crate::vehiclefact::battery_fact(*id, facts, crate::vehiclefact::battery_by_property(name)?, None),
            };
        }
        "vehicle.coordinate" => {
            let (latitude, longitude, altitude) = known.coordinate?;
            return Some(json!({ "kind": "coordinate", "latitude": latitude, "longitude": longitude, "altitude": altitude, "valid": true }));
        }
        "vehicle.homePosition" => {
            let (latitude, longitude, altitude) = known.home?;
            return Some(json!({ "kind": "coordinate", "latitude": latitude, "longitude": longitude, "altitude": altitude, "valid": true }));
        }
        _ => return None,
    };
    Some(json!({ "kind": "value", "value": value }))
}

fn answer_parameter(path: &str) -> Option<Value> {
    let (call, rest) = path.strip_prefix("vehicle.parameterManager.getParameter(")?.split_once(')')?;
    let (component, name) = call.split_once(',')?;
    let hub = crate::hub::lock();
    let vehicle = hub.active()?;
    (vehicle.autopilot == crate::modes::AUTOPILOT_ARDUPILOT).then_some(())?;
    let component = match component.trim().parse::<i64>().ok()? {
        -1 => vehicle.component,
        id => u8::try_from(id).ok()?,
    };
    let value = vehicle.parameter(component, name.trim())?;
    let value_type = crate::factmeta::ValueType::from_param_type(value.param_type())?;
    let version = vehicle.firmware().and_then(|f| f.version).map_or((-1, -1), |(major, minor, _, _)| (i64::from(major), i64::from(minor)));
    let definitions = crate::apmmeta::load(crate::apmmeta::vehicle_file_name(vehicle.vehicle_type)?, version.0, version.1)?;
    let number = |value: crate::params::ParamValue| match value {
        crate::params::ParamValue::F32(v) => json!(f64::from(v)),
        other => json!(other.as_f64() as i64),
    };
    let raw = number(value);
    let firmware_default = (component == vehicle.component).then(|| vehicle.parameter_defaults.get(name.trim()).copied()).flatten();
    let meta = crate::factmeta::MetaData { default: firmware_default.map(number), ..crate::apmmeta::json_metadata(&definitions, name.trim(), value_type) };
    let mut described = crate::vehiclefact::fact(&meta, &raw, None);
    if described["typeIsInteger"] == true {
        [("minString", &meta.min), ("maxString", &meta.max)].into_iter().filter_map(|(key, bound)| Some((key, bound.as_ref()?.as_f64()?))).for_each(|(key, bound)| described[key] = json!(crate::control::qt_shortest(bound)));
    }
    match rest.strip_prefix('.') {
        None if rest.is_empty() => Some(described),
        Some(field) => Some(json!({ "kind": "value", "value": described.get(field)?.clone() })),
        None => None,
    }
}

fn answer_scalar(path: &str, known: &Known) -> Option<Value> {
    let (parent, leaf) = path.rsplit_once('.')?;
    let (answered, _) = answer_fields(parent, leaf, known);
    let value = answered.get(leaf).filter(|v| v.is_boolean() || v.is_number() || v.is_string())?;
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
        switched_on().then(|| answer_parameter(path)).flatten().or_else(|| switched_on().then(carried).flatten().and_then(|known| answer_get(path, &known).or_else(|| answer_scalar(path, &known)))).map_or_else(
            || {
                fell_through("get", path);
                self.0.get(path)
            },
            |v| v.to_string(),
        )
    }
    fn get_fields(&self, path: &str, fields: &str) -> String {
        let group = (path == "vehicle.radioStatus").then(|| switched_on().then(carried).flatten()).flatten().and_then(|known| crate::vehiclefact::spec_group_fields(&crate::vehiclefact::RADIO, fields, |n| known.radio.raw(n)));
        if let Some(answered) = group {
            return answered.to_string();
        }
        let avoidance = (path == "vehicle.objectAvoidance").then(|| switched_on().then(carried).flatten()).flatten().and_then(|known| {
            let now = crate::hub::now_ms();
            let answered: Option<serde_json::Map<String, Value>> = fields_of(fields).into_iter().map(|f| Some((f.to_string(), known.obstacle.field(f, known.avoidance_enabled, now)?))).collect();
            let mut object = Value::Object(answered?);
            object["kind"] = json!("object");
            object["class"] = json!("VehicleObjectAvoidance");
            object["facts"] = json!([]);
            object["children"] = json!([]);
            Some(object)
        });
        if let Some(answered) = avoidance {
            return answered.to_string();
        }
        let (mut answered, missing) = switched_on().then(carried).flatten().map_or_else(|| (serde_json::Map::new(), fields_of(fields).into_iter().map(String::from).collect()), |known| answer_fields(path, fields, &known));
        let facts: Vec<Value> = match path {
            "vehicle" => VEHICLE_FACTS.iter().filter_map(|name| Some(crate::vehiclefact::compact(&answered.remove(*name)?, name))).collect(),
            _ => Vec::new(),
        };
        if missing.is_empty() && (!answered.is_empty() || !facts.is_empty()) {
            let mut object = Value::Object(answered);
            object["kind"] = json!("object");
            if !facts.is_empty() {
                object["facts"] = json!(facts);
            }
            return object.to_string();
        }
        let asked = if answered.is_empty() && facts.is_empty() { fields.to_string() } else { missing.join(",") };
        fell_through("fields", &format!("{path} [{asked}]"));
        let host = merged(answered, self.0.get_fields(path, &asked));
        match facts.is_empty() {
            true => host,
            false => serde_json::from_str::<Value>(&host).ok().filter(|h| h["kind"] == "object").map_or(host, |mut h| {
                let theirs = h.get("facts").and_then(Value::as_array).cloned().unwrap_or_default();
                h["facts"] = json!(theirs.into_iter().chain(facts).collect::<Vec<_>>());
                h.to_string()
            }),
        }
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
    fn airframe_and_firmware_text_follow_the_heartbeat_and_autopilot_version() {
        assert_eq!((mav_type_text(2), mav_type_text(45), mav_type_text(99)), ("Quadrotor", "Spacecraft, orbiter", "MAV_TYPE_UNKNOWN"));
        assert_eq!((motor_count(2, None), motor_count(10, None), motor_count(12, Some(4.0)), motor_count(12, Some(9.0))), (Some(4), Some(-1), Some(3), Some(-1)));
        assert_eq!(motor_count(12, None), None, "a sub's count waits for FRAME_CONFIG");
        let unknown = firmware_fields(3, None);
        assert_eq!((unknown["firmwareMajorVersion"].as_i64(), unknown["firmwareVersionTypeString"].as_str(), unknown["firmwareTypeString"].as_str()), (Some(-1), Some(""), Some("ArduPilot")));
        let beta = firmware_fields(12, Some(crate::connect::Firmware { version: Some((1, 15, 2, 128)), custom: None, git_hash: String::new() }));
        assert_eq!((beta["firmwareMinorVersion"].as_i64(), beta["firmwareVersionTypeString"].as_str(), beta["firmwareTypeString"].as_str()), (Some(15), Some("beta"), Some("PX4 Pro")));
    }

    #[test]
    fn a_gripper_is_read_from_each_firmware_parameter() {
        let with = |pairs: &'static [(&'static str, f64)]| move |name: &str| pairs.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
        assert!(has_gripper(3, with(&[("GRIP_ENABLE", 1.0)])));
        assert!(!has_gripper(3, with(&[("GRIP_ENABLE", 2.0)])));
        assert!(has_gripper(12, with(&[("PD_GRIPPER_TYPE", 0.0)])), "PX4 1.17 names a gripper type instead of enabling one");
        assert!(!has_gripper(12, with(&[("PD_GRIPPER_EN", 0.0), ("PD_GRIPPER_TYPE", 1.0)])), "the older switch wins where it exists");
        assert!(!has_gripper(0, with(&[("GRIP_ENABLE", 1.0)])));
    }

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
        let known = Known { id: 1, parameters_ready: true, lost: false, home: None, coordinate: None, batteries: Vec::new(), gps: crate::gpsfacts::GpsFacts::default(), vibration: crate::vehiclefact::VibrationFacts::default(), estimator: Default::default(), distance: Default::default(), capabilities: None, radio: Default::default(), obstacle: Default::default(), avoidance_enabled: false, temperature: Default::default(), local: Default::default(), sensors: json!({ "sensorNames": ["GPS"] }), supports: supports(3, 2), fields: json!({ "armed": false }) };
        assert_eq!(answer_fields("vehicles", "activeVehicleAvailable,activeVehicle", &known), (json!({ "activeVehicleAvailable": true }).as_object().unwrap().clone(), vec!["activeVehicle".to_string()]), "only the unknown field goes to the host");
        assert_eq!(merged(json!({ "armed": false }).as_object().unwrap().clone(), json!({ "kind": "object", "rcRSSI": 255 }).to_string()), json!({ "kind": "object", "rcRSSI": 255, "armed": false }).to_string());
        assert_eq!(merged(json!({ "armed": false }).as_object().unwrap().clone(), json!({ "kind": "null" }).to_string()), json!({ "kind": "null" }).to_string(), "a host with no such object keeps its answer");
        assert_eq!(answer_get("vehicle.id", &known), Some(json!({ "kind": "value", "value": 1 })));
        assert_eq!(answer_fields("vehicle", "coordinate", &known).0.get("coordinate"), Some(&Value::Null), "an unknown position reads as null nested, as the bridge spells it");
        assert_eq!(answer_get("vehicle.coordinate", &known), None, "a direct read of an unknown position stays with the host");
        let placed = Known { coordinate: Some((1.0, 2.0, 3.0)), ..known };
        assert_eq!(answer_get("vehicle.coordinate", &placed).unwrap()["kind"], "coordinate");
        let known = placed;
        assert_eq!(answer_scalar("plan.managerVehicle.capabilitiesKnown", &known), Some(json!({ "kind": "value", "value": false })), "capabilities are unknown until AUTOPILOT_VERSION answers or is given up on");
        let rally_only = Known { capabilities: Some(crate::connect::CAP_MISSION_RALLY), ..known };
        assert_eq!((answer_scalar("plan.geoFenceController.supported", &rally_only), answer_scalar("plan.rallyPointController.supported", &rally_only)), (Some(json!({ "kind": "value", "value": false })), Some(json!({ "kind": "value", "value": true }))));
        let known = rally_only;
        assert_eq!(answer_get("vehicle.flying", &known), None);
        assert_eq!(answer_get("vehicle.armed", &known), Some(json!({ "kind": "value", "value": false })));
        assert_eq!(answer_fields("vehicle.supports", "guidedTakeoffWithAltitude,orbitMode,smartRTL", &known).0, json!({ "guidedTakeoffWithAltitude": true, "orbitMode": false, "smartRTL": true }).as_object().unwrap().clone(), "an ArduCopter quad");
        assert_eq!(supports(12, 1)["guidedTakeoffWithoutAltitude"], true, "a PX4 plane takes off without an altitude");
        assert_eq!(answer_get("vehicle.sysStatusSensorInfo.sensorNames", &known), Some(json!({ "kind": "value", "value": ["GPS"] })));
    }
}
