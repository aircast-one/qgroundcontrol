use serde_json::{Value, json};

use crate::router::Backend;

pub struct Facade<B>(pub B);

const VEHICLE_FACTS: [&str; 17] = ["rcRSSI", "heading", "roll", "pitch", "rollRate", "pitchRate", "yawRate", "groundSpeed", "airSpeed", "climbRate", "altitudeRelative", "altitudeAMSL", "throttlePct", "distanceToNextWP", "distanceToHome", "headingToHome", "headingFromHome"];

pub fn qt_azimuth(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (lat1, lat2) = (from.0.to_radians(), to.0.to_radians());
    let dlon = (to.1 - from.1).to_radians();
    let y = dlon.sin() * lat2.cos();
    let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
    let azimuth = y.atan2(x).to_degrees() + 360.0;
    ((azimuth.trunc() as i64 + 360) % 360) as f64 + azimuth.fract()
}

fn home_facts(coordinate: Option<(f64, f64, f64)>, home: Option<(f64, f64, f64)>) -> (f64, f64, f64) {
    match coordinate.zip(home) {
        None => (f64::NAN, f64::NAN, f64::NAN),
        Some((here, home)) => {
            let distance = crate::terrain::qt_distance((here.0, here.1), (home.0, home.1));
            match distance > 1.0 {
                true => (distance, qt_azimuth((here.0, here.1), (home.0, home.1)), qt_azimuth((home.0, home.1), (here.0, here.1))),
                false => (distance, f64::NAN, f64::NAN),
            }
        }
    }
}

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

pub fn switched_on() -> bool {
    std::env::var("QGC_CORE_VEHICLE").map_or(cfg!(not(test)), |v| v == "1")
}

struct Known {
    id: u8,
    parameters_ready: bool,
    parameters_unanswered: bool,
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
    local_setpoint: crate::sensorfacts::LocalPositionFacts,
    wind: crate::sensorfacts::WindFacts,
    setpoint: crate::sensorfacts::SetpointFacts,
    orbit: Option<(f32, i32, i32)>,
    hygrometer: crate::sensorfacts::HygrometerFacts,
    generator: crate::sensorfacts::GeneratorFacts,
    efi: crate::sensorfacts::EfiFacts,
    terrain_blocks: (u16, u16),
    escs: crate::sensorfacts::Escs,
    trigger_points: (Vec<(f64, f64, f64)>, bool),
    mission_indices: (i32, i32, usize),
    links: (Vec<(u32, bool)>, Option<u32>),
    cameras: (Vec<String>, usize),
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

pub fn mav_type_text(mav_type: u8) -> &'static str {
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

pub fn mission_flight_mode(autopilot: u8, vehicle_type: u8, available: &[crate::standardmodes::FlightMode]) -> String {
    mode_fields(autopilot, vehicle_type, available).get("missionFlightMode").and_then(Value::as_str).unwrap_or_default().to_string()
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
    let apm = |rtl: Option<u32>, smart: Option<u32>, mission: Option<u32>, land: (Option<u32>, &str), pause: (Option<u32>, &str), follow: (Option<u32>, &str)| {
        json!({
            "flightModes": names(false),
            "advancedFlightModes": names(true),
            "flightModeSetAvailable": true,
            "rtlFlightMode": named(rtl, "RTL"),
            "smartRTLFlightMode": named(smart, "Smart RTL"),
            "missionFlightMode": named(mission, "Auto"),
            "landFlightMode": named(land.0, land.1),
            "pauseFlightMode": named(pause.0, pause.1),
            "followFlightMode": named(follow.0, follow.1),
        })
    };
    let fields = match (autopilot, crate::modes::vehicle_class(vehicle_type)) {
        (AUTOPILOT_ARDUPILOT, VehicleClass::MultiRotor) => apm(Some(6), Some(21), Some(3), (Some(9), "Land"), (Some(17), "Brake"), (Some(23), "Follow")),
        (AUTOPILOT_ARDUPILOT, VehicleClass::FixedWing) => apm(Some(11), Some(11), Some(10), (None, ""), (Some(12), "Loiter"), (None, "")),
        (AUTOPILOT_ARDUPILOT, VehicleClass::Rover) => apm(Some(11), Some(12), Some(10), (None, ""), (Some(4), "Hold"), (Some(6), "Follow")),
        (AUTOPILOT_ARDUPILOT, VehicleClass::Sub) => apm(None, None, Some(3), (None, ""), (None, ""), (None, "")),
        (AUTOPILOT_PX4, _) => json!({
            "advancedFlightModes": names(true),
            "rtlFlightMode": named(Some(px4(4, 5)), ""),
            "smartRTLFlightMode": "",
            "missionFlightMode": named(Some(px4(4, 4)), ""),
            "landFlightMode": named(Some(px4(4, 6)), ""),
            "pauseFlightMode": named(Some(px4(4, 3)), ""),
            "followFlightMode": named(Some(px4(4, 8)), ""),
        }),
        _ => json!({}),
    };
    fields.as_object().cloned().unwrap_or_default()
}

fn carried() -> Option<Known> {
    crate::hub::lock().active().map(known_of)
}

fn fleet_member(path: &str) -> Option<(usize, &str)> {
    let rest = path.strip_prefix("vehicles.vehicles.")?;
    let (index, tail) = rest.split_once('.').map_or((rest, ""), |(i, t)| (i, t));
    Some((index.parse().ok()?, tail))
}

fn unreported_checks() -> Option<Value> {
    switched_on().then_some(())?;
    let hub = crate::hub::lock();
    let vehicle = hub.active()?;
    (!vehicle.events_heard).then(|| json!({ "supported": false, "canArm": true, "canTakeoff": true, "canStartMission": true, "hasWarningsOrErrors": false }))
}

fn links_field(name: &str) -> Option<Value> {
    match name {
        "mavlinkSupportForwardingEnabled" => Some(json!(crate::forwarding::support_enabled())),
        _ => crate::seriallink::links_field(name),
    }
}

fn video_camera_name(index: i64) -> String {
    let setting = |name: &str| crate::settingsstore::raw_setting(&format!("settings.videoSettings.{name}")).and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    let name = match usize::try_from(index - 1) {
        Err(_) => setting("primaryCameraName"),
        Ok(extra) => serde_json::from_str::<Value>(&setting("extraVideoSources")).ok().and_then(|sources| sources.get(extra)?.get("name")?.as_str().map(str::to_string)).unwrap_or_default(),
    };
    if name.is_empty() { format!("Camera {}", index + 1) } else { name }
}

fn sensors_request(path: &str, args: &str) -> Option<Value> {
    let simple = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_bool()).unwrap_or(false);
    let start = |kind: &str| Some(json!({ "action": "start", "type": kind }));
    match path.strip_prefix("sensorsCal.")? {
        "calibrateAccel" if !simple => start("accelerometer"),
        "calibrateCompass" => start("compass"),
        "levelHorizon" => start("levelHorizon"),
        "calibrateGyro" => start("gyro"),
        "calibratePressure" => start("pressure"),
        "nextClicked" => Some(json!({ "action": "next" })),
        "cancelCalibration" => Some(json!({ "action": "cancel" })),
        _ => None,
    }
}

fn inspector_owned() -> bool {
    switched_on() && crate::hub::lock().active().is_some()
}

fn inspector_get(path: &str) -> Option<Value> {
    inspector_owned().then_some(())?;
    crate::mavinspect::lock().get(path)
}

fn only_fields(answer: Value, wanted: &[&str]) -> Value {
    let keep = |object: &Value| -> Value {
        let mut kept: serde_json::Map<String, Value> = object.as_object().map(|o| o.iter().filter(|(k, _)| wanted.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
        kept.insert("kind".to_string(), json!("object"));
        Value::Object(kept)
    };
    match answer.get("elements").and_then(Value::as_array) {
        Some(elements) => json!({ "kind": "object", "count": elements.len(), "elements": elements.iter().map(keep).collect::<Vec<_>>() }),
        None if answer["kind"] == "object" => keep(&answer),
        None => answer,
    }
}

fn shell_lines() -> Option<Vec<String>> {
    switched_on().then_some(())?;
    crate::hub::lock().active().map(|vehicle| vehicle.shell.lines())
}

fn selected_member(path: &str) -> Option<(usize, &str)> {
    let rest = path.strip_prefix("vehicles.selectedVehicles.")?;
    let (index, tail) = rest.split_once('.').map_or((rest, ""), |(i, t)| (i, t));
    Some((index.parse().ok()?, tail))
}

fn resolved(path: &str) -> Option<(String, Known)> {
    switched_on().then_some(())?;
    if let Some((index, tail)) = selected_member(path) {
        let field = if tail.is_empty() { "vehicle".to_string() } else { format!("vehicle.{tail}") };
        return crate::hub::lock().selected_member(index).map(|v| (field, known_of(v)));
    }
    match fleet_member(path) {
        Some((index, "")) => crate::hub::lock().listed(index).map(|v| ("vehicle".to_string(), known_of(v))),
        Some((index, tail)) => crate::hub::lock().listed(index).map(|v| (format!("vehicle.{tail}"), known_of(v))),
        None => carried().map(|known| (path.to_string(), known)),
    }
}

fn circular_fence(autopilot: u8, parameter: impl Fn(&str) -> Option<f64>) -> f64 {
    match autopilot {
        crate::modes::AUTOPILOT_PX4 => parameter("GF_MAX_HOR_DIST").unwrap_or(0.0),
        crate::modes::AUTOPILOT_ARDUPILOT => match (parameter("FENCE_RADIUS"), parameter("FENCE_ENABLE"), parameter("FENCE_TYPE")) {
            (Some(radius), Some(enabled), Some(kind)) if enabled != 0.0 && (kind as u32) & 2 != 0 => radius,
            _ => 0.0,
        },
        _ => 0.0,
    }
}

fn known_of(v: &crate::hub::Vehicle) -> Known {
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
        "heading": crate::vehiclefact::vehicle_fact("heading", &json!(v.facts.heading)),
        "roll": crate::vehiclefact::vehicle_fact("roll", &json!(v.facts.roll)),
        "pitch": crate::vehiclefact::vehicle_fact("pitch", &json!(v.facts.pitch)),
        "rollRate": crate::vehiclefact::vehicle_fact("rollRate", &json!(v.facts.roll_rate)),
        "pitchRate": crate::vehiclefact::vehicle_fact("pitchRate", &json!(v.facts.pitch_rate)),
        "yawRate": crate::vehiclefact::vehicle_fact("yawRate", &json!(v.facts.yaw_rate)),
        "groundSpeed": crate::vehiclefact::vehicle_fact("groundSpeed", &json!(v.facts.ground_speed)),
        "airSpeed": crate::vehiclefact::vehicle_fact("airSpeed", &json!(v.facts.air_speed)),
        "climbRate": crate::vehiclefact::vehicle_fact("climbRate", &json!(v.facts.climb_rate)),
        "altitudeRelative": crate::vehiclefact::vehicle_fact("altitudeRelative", &json!(v.facts.altitude_relative)),
        "altitudeAMSL": crate::vehiclefact::vehicle_fact("altitudeAMSL", &json!(v.facts.altitude_amsl)),
        "throttlePct": crate::vehiclefact::vehicle_fact("throttlePct", &json!(v.facts.throttle_pct)),
        "distanceToNextWP": crate::vehiclefact::vehicle_fact("distanceToNextWP", &json!(v.facts.distance_to_next_wp)),
        "distanceToHome": crate::vehiclefact::vehicle_fact("distanceToHome", &json!(home_facts(v.facts.coordinate, v.home).0)),
        "headingToHome": crate::vehiclefact::vehicle_fact("headingToHome", &json!(home_facts(v.facts.coordinate, v.home).1)),
        "headingFromHome": crate::vehiclefact::vehicle_fact("headingFromHome", &json!(home_facts(v.facts.coordinate, v.home).2)),
        "orbitActive": v.orbit_active(crate::hub::now_ms()),
        "rcChannelOverrideActive": !v.rc_override.is_empty(),
        "isROIEnabled": v.roi_enabled,
        "communicationLostEnabled": v.comm_lost_enabled,
        "autoDisconnect": v.auto_disconnect,
        "paramCircularFence": circular_fence(v.autopilot, |name| v.parameter(v.component, name).map(|p| p.as_f64())),
        "checkListState": v.check_list_state,
        "haveMRSpeedLimits": v.speed_limits().0,
        "haveFWSpeedLimits": v.speed_limits().1,
        "latitude": v.facts.coordinate.map(|(latitude, _, _)| f64::from(latitude as f32)),
        "longitude": v.facts.coordinate.map(|(_, longitude, _)| f64::from(longitude as f32)),
    });
    let fields = match (fields, v.control.fields()) {
        (Value::Object(mut mine), Value::Object(control)) => {
            mine.extend(control);
            Value::Object(mine)
        }
        (fields, _) => fields,
    };
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
    Known { id: v.id, parameters_ready: v.parameters_ready(), parameters_unanswered: v.parameters_unanswered(), lost: v.connection_lost, home: v.home, coordinate: v.facts.coordinate, batteries: v.batteries.by_id.iter().map(|(id, b)| (*id, b.clone())).collect(), gps: v.gps.clone(), vibration: v.vibration.clone(), estimator: v.estimator.clone(), distance: v.distance.clone(), capabilities: v.capabilities_known.then_some(v.capabilities), radio: v.radio.clone(), obstacle: v.obstacle.clone(), avoidance_enabled: v.parameter(v.component, "CP_DIST").is_some_and(|p| p.as_f64() >= 0.0), temperature: v.temperature.clone(), local: v.local.clone(), local_setpoint: v.local_setpoint.clone(), wind: v.wind.clone(), setpoint: v.setpoint.clone(), orbit: v.orbit_circle, hygrometer: v.hygrometer.clone(), generator: v.generator.clone(), efi: v.efi.clone(), terrain_blocks: v.terrain_blocks, escs: v.escs.clone(), trigger_points: (v.trigger_points.clone(), v.trigger_points_appended), mission_indices: (v.current_mission_index(), v.resume_mission_index(), v.fly_items()), links: (v.link_states.iter().map(|(link, _, lost)| (*link, *lost)).collect(), v.primary_link), cameras: (v.cameras.models(), v.cameras.selected_index().unwrap_or(0)), sensors, supports: supports(v.autopilot, v.vehicle_type), fields }
}

fn fields_of(fields: &str) -> Vec<&str> {
    fields.split(',').map(str::trim).filter(|f| !f.is_empty()).collect()
}

const SIMULATED_CAMERA: &str = "Simulated Camera";
const STORAGE_NOT_SUPPORTED: i64 = 3;
const CAMERA_INSTANCE: &str = "vehicle.cameraManager.currentCameraInstance";

fn onboard_logs<T>(read: impl FnOnce(&crate::onboardlogs::OnboardLogs) -> T) -> Option<T> {
    switched_on().then_some(())?;
    crate::hub::lock().active().map(|v| read(&v.onboard_logs))
}

fn onboard_log_get(path: &str) -> Option<Value> {
    match path {
        "logDownload" => onboard_logs(|logs| logs.controller_json()),
        "logDownload.model" => onboard_logs(|logs| logs.model_json()),
        field => {
            let name = field.strip_prefix("logDownload.")?;
            let value = onboard_logs(|logs| logs.controller_json().get(name).cloned())??;
            Some(json!({ "kind": "value", "value": value }))
        }
    }
}

fn current_camera_fields(recording: impl Fn() -> bool) -> Option<serde_json::Map<String, Value>> {
    switched_on().then_some(())?;
    let recording = recording();
    let hub = crate::hub::lock();
    let vehicle = hub.active()?;
    let camera = vehicle.cameras.selected()?;
    Some(camera_instance(camera, recording, vehicle.camera_tracking_enabled, crate::hub::now_ms()))
}

fn big_size_mb(size_mb: u64) -> String {
    let size = size_mb as f64;
    match size_mb {
        0..1024 => format!("{size_mb} MB"),
        1024..1_048_576 => format!("{:.1} GB", size / 1024.0),
        _ => format!("{:.2} TB", size / 1_048_576.0),
    }
}

fn clock_text(ms: u64) -> String {
    let seconds = (ms / 1000) % 86_400;
    format!("{:02}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

fn stored_number(key: &str) -> Option<f64> {
    crate::settingsstore::stored_text(key).and_then(|text| text.trim().parse().ok())
}

fn camera_instance(camera: &crate::cameraproto::Camera, recording: bool, tracking_enabled: bool, now_ms: u64) -> serde_json::Map<String, Value> {
    use crate::cameraproto::{CAP_CAPTURE_IMAGE, CAP_CAPTURE_VIDEO, CAP_HAS_BASIC_ZOOM, CAP_HAS_MODES, CAP_HAS_TRACKING_POINT, CAP_HAS_TRACKING_RECTANGLE, CAP_HAS_VIDEO_STREAM, CAP_IMAGE_IN_VIDEO_MODE, CAP_VIDEO_IN_IMAGE_MODE};
    let info = &camera.info;
    let flag = |bits: u32| info.flags & bits != 0;
    let photo = camera.photo_status.map_or(0, |s| if s < 4 { s } else { 255 });
    let video = camera.video_status.map_or(0, |s| if s < 2 { s } else { 255 });
    let mode = camera.mode.map_or(-1, i64::from);
    let has_modes = flag(CAP_HAS_MODES);
    let streams_or = |bits: u32| flag(bits | CAP_HAS_VIDEO_STREAM);
    let capture_video = match () {
        _ if video == 1 || recording => 2,
        _ if photo != 0 => 0,
        _ if has_modes && (mode == 0 || mode == 2) => 0,
        _ if streams_or(CAP_CAPTURE_VIDEO) => 1,
        _ => 0,
    };
    let capture_photos = match photo {
        1 => 2,
        2 | 3 => 3,
        0 if streams_or(CAP_CAPTURE_IMAGE) => 1,
        _ => 0,
    };
    let tracking = flag(CAP_HAS_TRACKING_POINT) || flag(CAP_HAS_TRACKING_RECTANGLE);
    let plain = [
        ("modelName", json!(info.model)),
        ("vendor", json!(info.vendor)),
        ("cameraMode", json!(mode)),
        ("capturePhotosState", json!(capture_photos)),
        ("captureVideoState", json!(capture_video)),
        ("recordTimeStr", json!(clock_text(camera.record_time_ms(now_ms).unwrap_or(0)))),
        ("storageStatus", json!(camera.storage_status().map_or(STORAGE_NOT_SUPPORTED, i64::from))),
        ("storageFreeStr", json!(big_size_mb(camera.free_mib.unwrap_or(0.0).max(0.0) as u64))),
        ("capturesPhotos", json!(streams_or(CAP_CAPTURE_IMAGE))),
        ("capturesVideo", json!(streams_or(CAP_CAPTURE_VIDEO))),
        ("hasModes", json!(has_modes)),
        ("photosInVideoMode", json!(flag(CAP_IMAGE_IN_VIDEO_MODE))),
        ("videoInPhotoMode", json!(flag(CAP_VIDEO_IN_IMAGE_MODE))),
        ("photoCaptureMode", json!(stored_number("PhotoCaptureMode").map_or(0, |v| v as i64))),
        ("photoLapse", json!(stored_number("PhotoLapse").unwrap_or(1.0))),
        ("photoLapseCount", json!(stored_number("PhotoLapseCount").map_or(0, |v| v as i64))),
        ("batteryRemaining", json!(camera.battery_percent.filter(|p| *p >= 0).map_or(-1, i64::from))),
        ("hasZoom", json!(flag(CAP_HAS_BASIC_ZOOM))),
        ("zoomLevel", json!(camera.zoom_percent.filter(|z| z.is_finite()).unwrap_or(0.0))),
        ("hasTracking", json!(tracking)),
        ("thermalMode", json!(stored_number("ThermalMode").map_or(1, |v| v as i64))),
        ("thermalOpacity", json!(stored_number("ThermalOpacity").unwrap_or(85.0))),
        ("trackingEnabled", json!(tracking_enabled)),
        ("supportsTrackingRect", json!(flag(CAP_HAS_TRACKING_RECTANGLE))),
        ("supportsTrackingPoint", json!(flag(CAP_HAS_TRACKING_POINT))),
    ];
    let unthermal = camera.thermal_stream().is_none().then(|| ("thermalStreamInstance", Value::Null));
    let untracked = (!tracking).then(|| [("trackingImageIsActive", json!(false)), ("trackingImageRect", Value::Null)]).into_iter().flatten();
    plain.into_iter().chain(unthermal).chain(untracked).map(|(k, v)| (k.to_string(), v)).collect()
}

fn link_fields(known: &Known) -> Option<serde_json::Map<String, Value>> {
    let transports = crate::linkhost::TRANSPORTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let described: Option<Vec<(u32, String, bool)>> = known.links.0.iter().map(|(link, lost)| transports.describe(*link).filter(|(_, kind, high_latency)| matches!(kind.as_str(), "udp" | "tcp") && !high_latency).map(|(name, _, _)| (*link, name, *lost))).collect();
    let described = described.filter(|d| !d.is_empty())?;
    let primary = known.links.1.and_then(|id| described.iter().find(|(link, _, _)| *link == id)).map_or_else(String::new, |(_, name, _)| name.clone());
    Some(serde_json::Map::from_iter([
        ("linkNames".to_string(), json!(described.iter().map(|(_, name, _)| name.clone()).collect::<Vec<_>>())),
        ("linkStatuses".to_string(), json!(described.iter().map(|(_, _, lost)| if *lost { "Comm Lost" } else { "" }).collect::<Vec<_>>())),
        ("primaryLinkName".to_string(), json!(primary)),
    ]))
}

fn answer_fields(path: &str, fields: &str, known: &Known) -> (serde_json::Map<String, Value>, Vec<String>) {
    let field = |name: &str| -> Option<Value> {
        match (path, name) {
            ("vehicles", "activeVehicleAvailable") => Some(json!(true)),
            ("vehicle.parameterManager", "parametersReady") => Some(json!(known.parameters_ready)),
            ("vehicle.parameterManager", "requestUnanswered") => Some(json!(known.parameters_unanswered)),
            ("vehicle.vehicleLinkManager", "communicationLost") => Some(json!(known.lost)),
            ("vehicle.vehicleLinkManager", "communicationLostEnabled") => known.fields.get("communicationLostEnabled").cloned(),
            ("vehicle.vehicleLinkManager", "autoDisconnect") => known.fields.get("autoDisconnect").cloned(),
            ("vehicle.vehicleLinkManager", link @ ("linkNames" | "linkStatuses" | "primaryLinkName")) => link_fields(known)?.remove(link),
            ("vehicle.supports", capability) => known.supports.get(capability).cloned(),
            ("plan.managerVehicle", "capabilitiesKnown") => Some(json!(known.capabilities.is_some())),
            ("plan.geoFenceController", "supported") => Some(json!(known.capabilities.unwrap_or(0) & crate::connect::CAP_MISSION_FENCE != 0)),
            ("plan.rallyPointController", "supported") => Some(json!(known.capabilities.unwrap_or(0) & crate::connect::CAP_MISSION_RALLY != 0)),
            ("vehicle", "coordinate") => Some(known.coordinate.map_or(Value::Null, |(latitude, longitude, altitude)| json!({ "valid": true, "latitude": latitude, "longitude": longitude, "altitude": altitude }))),
            ("vehicle", "homePosition") => Some(known.home.map_or(Value::Null, |(latitude, longitude, altitude)| json!({ "valid": true, "latitude": latitude, "longitude": longitude, "altitude": altitude }))),
            ("planFly.missionController", "currentMissionIndex") => Some(json!(known.mission_indices.0)),
            ("planFly.missionController", "resumeMissionIndex") => Some(json!(known.mission_indices.1)),
            ("planFly.missionController.visualItems", "count") => Some(json!(known.mission_indices.2)),
            ("plan.geoFenceController", "paramCircularFence") => known.fields.get("paramCircularFence").cloned(),
            ("vehicle.cameraManager", "cameraLabels") => Some(match known.cameras.0.is_empty() {
                true => json!([SIMULATED_CAMERA]),
                false => json!(known.cameras.0),
            }),
            ("vehicle.cameraManager", "currentCamera") => Some(json!(known.cameras.1)),
            ("vehicle.cameraTriggerPoints", "count") => Some(json!(known.trigger_points.0.len())),
            ("vehicle", field) => known.fields.get(field).cloned(),
            _ => None,
        }
    };
    let (answered, missing): (Vec<_>, Vec<_>) = fields_of(fields).into_iter().map(|name| (name, field(name))).partition(|(_, v)| v.is_some());
    (answered.into_iter().filter_map(|(name, v)| Some((name.to_string(), v?))).collect(), missing.into_iter().map(|(name, _)| name.to_string()).collect())
}

const CORE_OPTIONS: [(&str, bool); 1] = [("showMissionAbsoluteAltitude", true)];

fn object_of(mut fields: serde_json::Map<String, Value>) -> Value {
    fields.insert("kind".to_string(), json!("object"));
    Value::Object(fields)
}

fn core_option(name: &str) -> Option<bool> {
    CORE_OPTIONS.iter().find(|(held, _)| *held == name).map(|(_, value)| *value)
}

const FIRMWARE_LIMITS: [&str; 4] = ["vehicle.minimumTakeoffAltitudeMeters", "vehicle.maximumHorizontalSpeedMultirotorMetersSecond", "vehicle.maximumEquivalentAirspeed", "vehicle.minimumEquivalentAirspeed"];
const DEFAULT_TAKEOFF_METRES: f64 = 3.048;

fn firmware_limit(path: &str, autopilot: u8, vtol: bool, param: &dyn Fn(&str) -> Option<f64>) -> Option<f64> {
    let first = |names: &[(&str, f64)]| names.iter().find_map(|(name, scale)| param(name).map(|v| v * scale));
    match (autopilot, path) {
        (crate::modes::AUTOPILOT_ARDUPILOT, "vehicle.minimumTakeoffAltitudeMeters") => {
            let names: &[(&str, f64)] = if vtol { &[("Q_PILOT_TKO_ALT_M", 1.0), ("Q_PILOT_TKOFF_ALT", 0.01), ("Q_RTL_ALT", 1.0)] } else { &[("PILOT_TKO_ALT_M", 1.0), ("PILOT_TKOFF_ALT", 0.01)] };
            Some(first(names).filter(|v| *v != 0.0).unwrap_or(DEFAULT_TAKEOFF_METRES))
        }
        (_, "vehicle.minimumTakeoffAltitudeMeters") => Some(DEFAULT_TAKEOFF_METRES),
        (crate::modes::AUTOPILOT_ARDUPILOT, "vehicle.maximumHorizontalSpeedMultirotorMetersSecond") => first(&[("WP_SPD", 1.0), ("WPNAV_SPEED", 0.01)]),
        (crate::modes::AUTOPILOT_PX4, "vehicle.maximumHorizontalSpeedMultirotorMetersSecond") => param("MPC_XY_VEL_MAX"),
        (crate::modes::AUTOPILOT_ARDUPILOT, "vehicle.maximumEquivalentAirspeed") => param("AIRSPEED_MAX"),
        (crate::modes::AUTOPILOT_PX4, "vehicle.maximumEquivalentAirspeed") => param("FW_AIRSPD_MAX"),
        (crate::modes::AUTOPILOT_ARDUPILOT, "vehicle.minimumEquivalentAirspeed") => param("AIRSPEED_MIN"),
        (crate::modes::AUTOPILOT_PX4, "vehicle.minimumEquivalentAirspeed") => param("FW_AIRSPD_MIN"),
        _ => None,
    }
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
        "vehicle.parameterManager.requestUnanswered" => json!(known.parameters_unanswered),
        "vehicle.vehicleLinkManager.communicationLost" => json!(known.lost),
        "vehicle.sysStatusSensorInfo" => {
            let mut object = json!({ "children": [], "class": "SysStatusSensorInfo", "facts": [], "kind": "object", "objectName": "" });
            known.sensors.as_object()?.iter().for_each(|(k, v)| object[k.as_str()] = v.clone());
            return Some(object);
        }
        fact if fact.strip_prefix("vehicle.").is_some_and(|name| VEHICLE_FACTS.contains(&name)) => return known.fields.get(&fact["vehicle.".len()..]).cloned().filter(|f| !f.is_null()),
        property if property.strip_prefix("vehicle.").and_then(|rest| rest.split_once('.')).is_some_and(|(name, _)| VEHICLE_FACTS.contains(&name)) => {
            let (name, field) = property["vehicle.".len()..].split_once('.')?;
            return known.fields.get(name)?.get(field).cloned().map(|v| json!({ "kind": "value", "value": v }));
        }
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
        "vehicle.orbitMapCircle" => return crate::vehiclefact::orbit_circle(known.orbit),
        circle if circle.starts_with("vehicle.orbitMapCircle.") => return crate::vehiclefact::orbit_circle_part(known.orbit, &circle["vehicle.orbitMapCircle.".len()..]),
        "vehicle.cameraTriggerPoints" => return Some(crate::vehiclefact::trigger_points(&known.trigger_points.0, known.trigger_points.1)),
        points if points.starts_with("vehicle.cameraTriggerPoints.") => return crate::vehiclefact::trigger_points_part(&known.trigger_points.0, &points["vehicle.cameraTriggerPoints.".len()..]),
        "vehicle.escs" => return Some(crate::vehiclefact::esc_list(&known.escs)),
        "vehicle.escs.count" => return Some(json!({ "kind": "value", "value": known.escs.by_id.len() })),
        "vehicle.terrain" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::TERRAIN, |n| crate::vehiclefact::terrain_raw(known.terrain_blocks, n), false)),
        terrain if terrain.starts_with("vehicle.terrain.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::TERRAIN, &terrain["vehicle.terrain.".len()..], |n| crate::vehiclefact::terrain_raw(known.terrain_blocks, n)),
        "vehicle.efi" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::EFI, |n| crate::vehiclefact::efi_raw(&known.efi, n), known.efi.seen)),
        efi if efi.starts_with("vehicle.efi.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::EFI, &efi["vehicle.efi.".len()..], |n| crate::vehiclefact::efi_raw(&known.efi, n)),
        "vehicle.generator" => {
            let mut group = crate::vehiclefact::spec_group(&crate::vehiclefact::GENERATOR, |n| crate::vehiclefact::generator_raw(&known.generator, n), known.generator.seen);
            group["flagsListGenerator"] = crate::vehiclefact::generator_flags(&known.generator);
            return Some(group);
        }
        "vehicle.generator.flagsListGenerator" => return Some(json!({ "kind": "value", "value": crate::vehiclefact::generator_flags(&known.generator) })),
        generator if generator.starts_with("vehicle.generator.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::GENERATOR, &generator["vehicle.generator.".len()..], |n| crate::vehiclefact::generator_raw(&known.generator, n)),
        "vehicle.hygrometer" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::HYGROMETER, |n| crate::vehiclefact::hygrometer_raw(&known.hygrometer, n), known.hygrometer.seen)),
        hygrometer if hygrometer.starts_with("vehicle.hygrometer.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::HYGROMETER, &hygrometer["vehicle.hygrometer.".len()..], |n| crate::vehiclefact::hygrometer_raw(&known.hygrometer, n)),
        "vehicle.setpoint" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::SETPOINT, |n| crate::vehiclefact::setpoint_raw(&known.setpoint, n), known.setpoint.seen)),
        setpoint if setpoint.starts_with("vehicle.setpoint.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::SETPOINT, &setpoint["vehicle.setpoint.".len()..], |n| crate::vehiclefact::setpoint_raw(&known.setpoint, n)),
        "vehicle.wind" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::WIND, |n| crate::vehiclefact::wind_raw(&known.wind, n), known.wind.seen.iter().any(|s| *s))),
        wind if wind.starts_with("vehicle.wind.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::WIND, &wind["vehicle.wind.".len()..], |n| crate::vehiclefact::wind_raw(&known.wind, n)),
        "vehicle.localPositionSetpoint" => return Some(crate::vehiclefact::spec_group(&crate::vehiclefact::LOCAL_POSITION_SETPOINT, |n| crate::vehiclefact::local_position_raw(&known.local_setpoint, n), known.local_setpoint.seen)),
        target if target.starts_with("vehicle.localPositionSetpoint.") => return crate::vehiclefact::spec_property(&crate::vehiclefact::LOCAL_POSITION_SETPOINT, &target["vehicle.localPositionSetpoint.".len()..], |n| crate::vehiclefact::local_position_raw(&known.local_setpoint, n)),
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
    let value = answered.get(leaf).filter(|v| v.is_boolean() || v.is_number() || v.is_string() || v.as_array().is_some_and(|a| a.iter().all(Value::is_string)))?;
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

impl<B: Backend> Facade<B> {
    fn shell_invoke(&self, path: &str, args: &str) -> Option<String> {
        (path == "mavlinkConsole.sendCommand" && switched_on()).then_some(())?;
        let vehicle = crate::hub::lock().active_id()?;
        let command = serde_json::from_str::<Value>(args).ok()?.get(0)?.as_str()?.to_string();
        let started = self.0.core_guided(&json!({ "action": "shellCommand", "vehicle": vehicle, "command": command }))?;
        Some(json!({ "ok": started.is_ok() }).to_string())
    }

    fn selection_invoke(&self, path: &str, args: &str) -> Option<String> {
        let name = path.strip_prefix("vehicles.")?;
        switched_on().then_some(())?;
        let id = || serde_json::from_str::<Value>(args).ok()?.get(0)?.as_i64().and_then(|id| u8::try_from(id).ok());
        match name {
            "selectVehicle" => crate::hub::lock().select_vehicle(id()?),
            "deselectVehicle" => crate::hub::lock().deselect_vehicle(id()?),
            "deselectAllVehicles" => crate::hub::lock().set_selected(Vec::new()),
            _ => return None,
        }
        Some(self.0.invoke(path, args))
    }

    fn onboard_log_invoke(&self, path: &str, args: &str) -> Option<String> {
        let name = path.strip_prefix("logDownload.")?;
        switched_on().then_some(())?;
        let vehicle = crate::hub::lock().active_id()?;
        let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
        let first_bool = || given.get(0).and_then(Value::as_bool);
        let dispatch = |action: Value| self.0.core_guided(&action).map(|started| json!({ "ok": started.is_ok() }).to_string());
        match name {
            "refresh" => dispatch(json!({ "action": "logRefresh", "vehicle": vehicle })),
            "cancel" => dispatch(json!({ "action": "logCancel", "vehicle": vehicle })),
            "eraseAll" => dispatch(json!({ "action": "logEraseAll", "vehicle": vehicle })),
            "download" => {
                let folder = given.get(0).and_then(Value::as_str).filter(|f| !f.is_empty()).map(str::to_string).or_else(crate::settingsstore::log_save_path).unwrap_or_default();
                dispatch(json!({ "action": "logDownload", "vehicle": vehicle, "folder": folder }))
            }
            "selectAll" => crate::hub::lock().with_onboard_logs(|logs| logs.select_all(first_bool().unwrap_or(true))).map(|_| json!({ "ok": true }).to_string()),
            "setSortAscending" => crate::hub::lock().with_onboard_logs(|logs| logs.set_sort_ascending(first_bool().unwrap_or(false))).map(|_| json!({ "ok": true }).to_string()),
            "toggleSortByDate" => crate::hub::lock().with_onboard_logs(|logs| logs.set_sort_ascending(!logs.sort_ascending)).map(|_| json!({ "ok": true }).to_string()),
            _ => None,
        }
    }
}

impl<B: Backend> Backend for Facade<B> {
    fn get(&self, path: &str) -> String {
        if path == "core.qtReads" {
            return tally().to_string();
        }
        let count = (path == "vehicles.vehicles.count" && switched_on()).then(|| json!({ "kind": "value", "value": crate::hub::lock().fleet_count() }));
        let count = count.or_else(|| (path == "vehicles.selectedVehicles.count" && switched_on()).then(|| json!({ "kind": "value", "value": crate::hub::lock().selected_count() })));
        if let Some(answer) = path.starts_with("logDownload").then(|| onboard_log_get(path)).flatten() {
            return answer.to_string();
        }
        if let Some(value) = path.strip_prefix("corePlugin.options.").and_then(core_option) {
            return json!({ "kind": "value", "value": value }).to_string();
        }
        if let Some(value) = path.rsplit_once('.').and_then(|(object, field)| crate::coreplan::controller_fields(object)?.get(field).cloned()) {
            return json!({ "kind": "value", "value": value }).to_string();
        }
        if let Some(field) = path.strip_prefix("vehicle.cameraManager.currentCameraInstance.") {
            let recording = || crate::read::flag(&crate::read::object(&self.0.get_fields("video", "recording")), "recording");
            if let Some(value) = current_camera_fields(recording).and_then(|mut fields| fields.remove(field)) {
                return json!({ "kind": "value", "value": value }).to_string();
            }
        }
        if let Some(value) = path.strip_prefix("vehicle.healthAndArmingCheckReport.").and_then(|name| unreported_checks()?.get(name).cloned()) {
            return json!({ "kind": "value", "value": value }).to_string();
        }
        if let Some(answer) = inspector_get(path) {
            return answer.to_string();
        }
        if let Some(sensors) = path.strip_prefix("sensorsCal").filter(|_| switched_on()).and_then(|rest| {
            let sensors = crate::hub::lock().sensors_json()?;
            match rest.strip_prefix('.') {
                None if rest.is_empty() => Some(sensors),
                Some(field) => sensors.get(field).cloned().map(|v| json!({ "kind": "value", "value": v })),
                None => None,
            }
        }) {
            return sensors.to_string();
        }
        if let Some(radio) = path.strip_prefix("radioCal").filter(|_| switched_on()).and_then(|rest| {
            let radio = crate::hub::lock().radio_json()?;
            match rest.strip_prefix('.') {
                None if rest.is_empty() => Some(radio),
                Some(field) => radio.get(field).cloned().map(|v| json!({ "kind": "value", "value": v })),
                None => None,
            }
        }) {
            return radio.to_string();
        }
        if let Some(formatted) = (path == "vehicle.formattedMessages" && switched_on()).then(|| crate::hub::lock().active().map(|v| v.message_log.formatted())).flatten() {
            return json!({ "kind": "value", "value": formatted }).to_string();
        }
        if let Some(lines) = (path == "mavlinkConsole.lines").then(shell_lines).flatten() {
            return json!({ "kind": "value", "value": lines }).to_string();
        }
        if let Some(value) = path.strip_prefix("positionManager.").filter(|_| switched_on()).and_then(|name| crate::gcsposition::lock().property(name)) {
            return match (path, value) {
                ("positionManager.gcsPosition", Value::Null) => json!({ "kind": "coordinate", "valid": false, "latitude": null, "longitude": null, "altitude": null }),
                ("positionManager.gcsPosition", mut point) => {
                    point["kind"] = json!("coordinate");
                    point
                }
                (_, value) => json!({ "kind": "value", "value": value }),
            }
            .to_string();
        }
        if let Some(field) = path.strip_prefix("links.").filter(|_| switched_on()).and_then(links_field) {
            return json!({ "kind": "value", "value": field }).to_string();
        }
        let absent = (switched_on() && (path == "vehicle" || path.starts_with("vehicle.")) && crate::hub::lock().active_id().is_none()).then(|| json!({ "kind": "null" }));
        let absent = absent.or_else(|| (switched_on() && selected_member(path).is_some_and(|(index, _)| crate::hub::lock().selected_member(index).is_none())).then(|| json!({ "kind": "null" })));
        count.or(absent).or_else(|| switched_on().then(|| answer_parameter(path)).flatten()).or_else(|| resolved(path).and_then(|(path, known)| answer_get(&path, &known).or_else(|| answer_scalar(&path, &known)))).map_or_else(
            || {
                fell_through("get", path);
                self.0.get(path)
            },
            |v| v.to_string(),
        )
    }
    fn get_fields(&self, asked_path: &str, fields: &str) -> String {
        if let Some(controller) = (asked_path == "logDownload").then(|| onboard_logs(|logs| logs.controller_json())).flatten() {
            let wanted = fields_of(fields);
            let mut object: serde_json::Map<String, Value> = controller.as_object().cloned().unwrap_or_default().into_iter().filter(|(k, _)| wanted.contains(&k.as_str())).collect();
            object.insert("kind".to_string(), json!("object"));
            return Value::Object(object).to_string();
        }
        if asked_path == CAMERA_INSTANCE {
            let recording = || crate::read::flag(&crate::read::object(&self.0.get_fields("video", "recording")), "recording");
            if let Some(mut answered) = current_camera_fields(recording) {
                let wanted = fields_of(fields);
                let missing: Vec<&str> = wanted.iter().filter(|f| !answered.contains_key(**f)).copied().collect();
                answered.retain(|k, _| wanted.contains(&k.as_str()));
                if missing.is_empty() {
                    let mut object = Value::Object(answered);
                    object["kind"] = json!("object");
                    return object.to_string();
                }
                fell_through("fields", &format!("{asked_path} [{}]", missing.join(",")));
                return merged(answered, self.0.get_fields(asked_path, &missing.join(",")));
            }
        }
        if let Some(options) = (asked_path == "corePlugin.options").then(|| fields_of(fields).into_iter().map(|f| Some((f.to_string(), json!(core_option(f)?)))).collect::<Option<serde_json::Map<String, Value>>>()).flatten() {
            return object_of(options).to_string();
        }
        if let Some(Value::Object(mut answered)) = crate::coreplan::controller_fields(asked_path) {
            let wanted = fields_of(fields);
            let missing: Vec<&str> = wanted.iter().filter(|f| !answered.contains_key(**f)).copied().collect();
            answered.retain(|k, _| wanted.contains(&k.as_str()) || k == "kind");
            if missing.is_empty() {
                return Value::Object(answered).to_string();
            }
            fell_through("fields", &format!("{asked_path} [{}]", missing.join(",")));
            let host: Value = serde_json::from_str(&self.0.get_fields(asked_path, &missing.join(","))).unwrap_or(Value::Null);
            host.as_object().filter(|h| h.get("kind") == Some(&json!("object"))).into_iter().flatten().for_each(|(k, v)| {
                answered.entry(k.clone()).or_insert_with(|| v.clone());
            });
            return Value::Object(answered).to_string();
        }
        if let Some(lines) = (asked_path == "mavlinkConsole" && fields_of(fields) == ["lines"]).then(shell_lines).flatten() {
            return json!({ "kind": "object", "lines": lines }).to_string();
        }
        if let Some(sensors) = (asked_path == "sensorsCal" && switched_on()).then(|| crate::hub::lock().sensors_json()).flatten() {
            return only_fields(sensors, &fields_of(fields)).to_string();
        }
        if let Some(radio) = (asked_path == "radioCal" && switched_on()).then(|| crate::hub::lock().radio_json()).flatten() {
            return only_fields(radio, &fields_of(fields)).to_string();
        }
        if let Some(report) = (asked_path == "vehicle.healthAndArmingCheckReport").then(unreported_checks).flatten() {
            let answered: Option<serde_json::Map<String, Value>> = fields_of(fields).into_iter().map(|f| Some((f.to_string(), report.get(f)?.clone()))).collect();
            if let Some(mut object) = answered.map(Value::Object) {
                object["kind"] = json!("object");
                return object.to_string();
            }
        }
        if let Some(answer) = inspector_get(asked_path) {
            return only_fields(answer, &fields_of(fields)).to_string();
        }
        if asked_path == "positionManager" && switched_on() {
            let position = crate::gcsposition::lock();
            let answered: Option<serde_json::Map<String, Value>> = fields_of(fields).into_iter().map(|f| Some((f.to_string(), position.property(f)?))).collect();
            if let Some(mut object) = answered.map(Value::Object) {
                object["kind"] = json!("object");
                return object.to_string();
            }
        }
        if asked_path == "links" && switched_on() {
            let (answered, missing): (Vec<_>, Vec<_>) = fields_of(fields).into_iter().map(|f| (f, links_field(f))).partition(|(_, v)| v.is_some());
            let answered: serde_json::Map<String, Value> = answered.into_iter().filter_map(|(f, v)| Some((f.to_string(), v?))).collect();
            if missing.is_empty() {
                let mut object = Value::Object(answered);
                object["kind"] = json!("object");
                return object.to_string();
            }
            let asked = missing.iter().map(|(f, _)| *f).collect::<Vec<_>>().join(",");
            fell_through("fields", &format!("links [{asked}]"));
            return merged(answered, self.0.get_fields(asked_path, &asked));
        }
        let resolved = resolved(asked_path);
        let path = resolved.as_ref().map_or(asked_path, |(path, _)| path.as_str());
        let known = resolved.as_ref().map(|(_, known)| known);
        let group = (path == "vehicle.radioStatus").then_some(known).flatten().and_then(|known| crate::vehiclefact::spec_group_fields(&crate::vehiclefact::RADIO, fields, |n| known.radio.raw(n)));
        if let Some(answered) = group {
            return answered.to_string();
        }
        let avoidance = (path == "vehicle.objectAvoidance").then_some(known).flatten().and_then(|known| {
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
        let (mut answered, missing) = known.map_or_else(|| (serde_json::Map::new(), fields_of(fields).into_iter().map(String::from).collect()), |known| answer_fields(path, fields, known));
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
        fell_through("fields", &format!("{asked_path} [{asked}]"));
        let host = merged(answered, self.0.get_fields(asked_path, &asked));
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
        let link_flag = path.strip_prefix("vehicle.vehicleLinkManager.").filter(|f| matches!(*f, "communicationLostEnabled" | "autoDisconnect"));
        if let Some(flag) = link_flag.filter(|_| switched_on()) {
            let on = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").and_then(Value::as_bool).or_else(|| v.as_bool()));
            if let Some(on) = on {
                crate::hub::lock().set_link_flag(flag, on);
            }
            return self.0.set(path, value);
        }
        if let Some(index) = crate::logs::selection_index(path).filter(|_| switched_on()) {
            let on = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").and_then(Value::as_bool).or_else(|| v.as_bool()));
            if let Some(held) = on.and_then(|on| crate::hub::lock().with_onboard_logs(|logs| logs.select(index, on))) {
                return json!({ "ok": held }).to_string();
            }
        }
        if path == "vehicle.cameraManager.currentCameraInstance.trackingEnabled" && switched_on() {
            let on = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").and_then(Value::as_bool).or_else(|| v.as_bool()));
            if let Some(on) = on {
                crate::hub::lock().set_camera_tracking(on);
            }
            return self.0.set(path, value);
        }
        if path == "vehicle.cameraManager.currentCamera" && switched_on() {
            let index = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").and_then(Value::as_u64).or_else(|| v.as_u64()));
            if let Some(index) = index.and_then(|i| usize::try_from(i).ok()) {
                crate::hub::lock().select_camera(index);
            }
            return self.0.set(path, value);
        }
        if path == "mavlinkInspector.activeSystem.selected" && inspector_owned() {
            let index = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").and_then(Value::as_u64).or_else(|| v.as_u64()));
            let fits = index.and_then(|i| usize::try_from(i).ok()).is_some_and(|i| crate::mavinspect::lock().select(i));
            return json!({ "ok": fits }).to_string();
        }
        if path == "radioCal.transmitterMode" && switched_on() {
            let mode = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").and_then(Value::as_i64).or_else(|| v.as_i64()));
            if let Some(held) = mode.map(|m| crate::hub::lock().set_transmitter_mode(m)).filter(|held| *held) {
                return json!({ "ok": held }).to_string();
            }
        }
        if path == "vehicle.checkListState" && switched_on() {
            let state = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").and_then(Value::as_i64).or_else(|| v.as_i64()));
            if let Some(state) = state {
                crate::hub::lock().set_check_list_state(state);
            }
            return self.0.set(path, value);
        }
        fell_through("set", path);
        self.0.set(path, value)
    }
    fn invoke(&self, path: &str, args: &str) -> String {
        if let Some(answer) = self.onboard_log_invoke(path, args) {
            return answer;
        }
        if let Some(answer) = self.selection_invoke(path, args) {
            return answer;
        }
        if let Some(answer) = self.shell_invoke(path, args) {
            return answer;
        }
        if let Some(request) = sensors_request(path, args).filter(|_| switched_on() && crate::hub::lock().sensors_json().is_some()) {
            let vehicle = crate::hub::lock().active_id();
            let started = self.0.core_guided(&json!({ "action": "calibrate", "vehicle": vehicle, "request": request }));
            return json!({ "ok": started.is_some_and(|s| s.is_ok()) }).to_string();
        }
        if let Some(op) = (switched_on() && crate::hub::lock().active().is_some()).then(|| match path {
            "radioCal.nextButtonClicked" => Some("next"),
            "radioCal.cancelButtonClicked" => Some("cancel"),
            _ => None,
        }).flatten() {
            let vehicle = crate::hub::lock().active_id();
            let started = self.0.core_guided(&json!({ "action": "rcCal", "vehicle": vehicle, "op": op }));
            return json!({ "ok": started.is_some_and(|s| s.is_ok()) }).to_string();
        }
        if let Some(limit) = FIRMWARE_LIMITS.contains(&path).then(|| {
            let hub = crate::hub::lock();
            hub.active().map(|v| firmware_limit(path, v.autopilot, (19..=25).contains(&v.vehicle_type), &|name| v.parameter(v.component, name).map(|p| p.as_f64())))
        }).flatten().filter(|_| switched_on()) {
            return json!({ "ok": true, "result": limit }).to_string();
        }
        if path == "vehicle.requestOperatorControl" && inspector_owned() {
            let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
            let allow = given.get(0).and_then(Value::as_bool).unwrap_or(false);
            let timeout = given.get(1).and_then(Value::as_i64).unwrap_or(0);
            let meta = crate::settingsstore::metadata("FlyView", "requestControlTimeout");
            let limit = |value: Option<&Value>, fallback: i64| value.and_then(Value::as_i64).unwrap_or(fallback);
            let safe = crate::operatorcontrol::safe_timeout(timeout, (limit(meta.as_ref().and_then(|m| m.min.as_ref()), 3), limit(meta.as_ref().and_then(|m| m.max.as_ref()), 60)), limit(meta.as_ref().and_then(|m| m.default.as_ref()), 10));
            let vehicle = crate::hub::lock().active_id();
            let started = self.0.core_guided(&json!({ "action": "requestControl", "vehicle": vehicle, "allowTakeover": allow, "timeout": timeout, "safeTimeout": safe }));
            return json!({ "ok": started.is_some_and(|s| s.is_ok()) }).to_string();
        }
        if path == "mavlinkInspector.setMessageInterval" && inspector_owned() {
            let rate = serde_json::from_str::<Value>(args).ok().and_then(|v| v.get(0)?.as_i64());
            let target = crate::mavinspect::lock().selected_target();
            let started = rate.zip(target).and_then(|(rate, (vehicle, component, message))| self.0.core_guided(&json!({ "action": "messageInterval", "vehicle": vehicle, "component": component, "message": message, "rate": rate })));
            return json!({ "ok": started.is_some_and(|s| s.is_ok()) }).to_string();
        }
        if let Some(index) = (path == "video.cameraName" && switched_on()).then(|| serde_json::from_str::<Value>(args).ok()?.get(0)?.as_i64()).flatten() {
            return json!({ "ok": true, "result": video_camera_name(index) }).to_string();
        }
        if path == "vehicle.clearMessages" && switched_on() {
            crate::hub::lock().clear_message_log();
        }
        if switched_on() {
            match path {
                "links.createMavlinkForwardingSupportLink" => return json!({ "ok": crate::forwarding::start_support() }).to_string(),
                "links.endMavlinkForwardingSupportLink" => {
                    crate::forwarding::end_support();
                    return json!({ "ok": true }).to_string();
                }
                _ => {}
            }
        }
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
        assert_eq!(copter["followFlightMode"], "Follow");
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
    fn a_circular_fence_is_the_radius_only_while_enabled_as_a_circle() {
        let params = |list: &'static [(&'static str, f64)]| move |name: &str| list.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
        assert_eq!(circular_fence(3, params(&[("FENCE_RADIUS", 150.0), ("FENCE_ENABLE", 1.0), ("FENCE_TYPE", 7.0)])), 150.0);
        assert_eq!(circular_fence(3, params(&[("FENCE_RADIUS", 150.0), ("FENCE_ENABLE", 0.0), ("FENCE_TYPE", 7.0)])), 0.0);
        assert_eq!(circular_fence(3, params(&[("FENCE_RADIUS", 150.0), ("FENCE_ENABLE", 1.0), ("FENCE_TYPE", 5.0)])), 0.0, "bit 1 is the circle");
        assert_eq!(circular_fence(12, params(&[("GF_MAX_HOR_DIST", 80.0)])), 80.0);
        assert_eq!(circular_fence(12, params(&[])), 0.0);
    }

    #[test]
    fn a_fleet_path_names_a_list_position_and_what_follows_it() {
        assert_eq!(fleet_member("vehicles.vehicles.0"), Some((0, "")));
        assert_eq!(fleet_member("vehicles.vehicles.12.gps.lat"), Some((12, "gps.lat")));
        assert_eq!(fleet_member("vehicles.vehicles.#.supports"), None, "a wildcard is the host's to expand");
        assert_eq!(fleet_member("vehicles.vehicles.count"), None);
    }

    #[test]
    fn an_unnamed_camera_is_called_by_its_slot() {
        assert_eq!((video_camera_name(0), video_camera_name(2)), ("Camera 1".to_string(), "Camera 3".to_string()));
    }

    #[test]
    fn firmware_limits_read_the_parameter_each_firmware_names_with_its_units() {
        let params = |held: &'static [(&'static str, f64)]| move |name: &str| held.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
        let apm = crate::modes::AUTOPILOT_ARDUPILOT;
        assert_eq!(firmware_limit("vehicle.minimumTakeoffAltitudeMeters", apm, false, &params(&[("PILOT_TKOFF_ALT", 250.0)])), Some(2.5), "pre-4.7 copters give centimetres");
        assert_eq!(firmware_limit("vehicle.minimumTakeoffAltitudeMeters", apm, true, &params(&[("PILOT_TKOFF_ALT", 250.0), ("Q_RTL_ALT", 15.0)])), Some(15.0), "a VTOL reads the Q_ parameters");
        assert_eq!(firmware_limit("vehicle.minimumTakeoffAltitudeMeters", apm, false, &params(&[("PILOT_TKOFF_ALT", 0.0)])), Some(3.048), "zero means the firmware default");
        assert_eq!(firmware_limit("vehicle.minimumTakeoffAltitudeMeters", crate::modes::AUTOPILOT_PX4, false, &params(&[])), Some(3.048));
        assert_eq!(firmware_limit("vehicle.maximumHorizontalSpeedMultirotorMetersSecond", apm, false, &params(&[("WPNAV_SPEED", 1000.0)])), Some(10.0));
        assert_eq!(firmware_limit("vehicle.maximumHorizontalSpeedMultirotorMetersSecond", apm, false, &params(&[("WP_SPD", 7.0), ("WPNAV_SPEED", 1000.0)])), Some(7.0), "4.7 names win");
        assert_eq!(firmware_limit("vehicle.maximumEquivalentAirspeed", apm, false, &params(&[])), None, "no parameter is no limit");
    }

    #[test]
    fn only_fields_the_hub_knows_are_answered_and_the_rest_fall_through() {
        let known = Known { id: 1, parameters_ready: true, parameters_unanswered: false, lost: false, home: None, coordinate: None, batteries: Vec::new(), gps: crate::gpsfacts::GpsFacts::default(), vibration: crate::vehiclefact::VibrationFacts::default(), estimator: Default::default(), distance: Default::default(), capabilities: None, radio: Default::default(), obstacle: Default::default(), avoidance_enabled: false, temperature: Default::default(), local: Default::default(), local_setpoint: Default::default(), wind: Default::default(), setpoint: Default::default(), orbit: None, hygrometer: Default::default(), generator: Default::default(), efi: Default::default(), terrain_blocks: (0, 0), escs: Default::default(), trigger_points: Default::default(), mission_indices: (-1, 0, 1), links: (Vec::new(), None), cameras: (Vec::new(), 0), sensors: json!({ "sensorNames": ["GPS"] }), supports: supports(3, 2), fields: json!({ "armed": false }) };
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
