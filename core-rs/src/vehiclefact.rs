use serde_json::{Value, json};

use crate::batteryfacts::BatteryFacts;
use crate::factmeta::{MetaData, ValueType};

const BATTERY_META: &str = include_str!("../../src/Vehicle/FactGroups/BatteryFact.json");
const GPS_META: &str = include_str!("../../src/Vehicle/FactGroups/GPSFact.json");
const GPS_ANSWERED: [&str; 9] = ["lat", "lon", "mgrs", "hdop", "vdop", "courseOverGround", "yaw", "count", "lock"];

fn invalid_text(value_type: &ValueType, decimals: i64) -> String {
    match value_type {
        ValueType::Float | ValueType::Double if decimals > 0 => format!("–.{}", "–".repeat(usize::try_from(decimals).unwrap_or(0))),
        ValueType::ElapsedSeconds => "––:––:––".to_string(),
        _ => "–".to_string(),
    }
}

pub fn fact(meta: &MetaData, raw: &Value, property: Option<&str>) -> Value {
    let mut described = crate::settingsstore::fact_json(meta, raw, crate::units::for_fact(meta, crate::units::cooking));
    described["userVisible"] = Value::Null;
    described["visible"] = Value::Null;
    if meta.default.is_none() {
        described["defaultValue"] = Value::Null;
        described["defaultValueString"] = Value::Null;
    }
    let text = invalid_text(&meta.value_type, described["decimalPlaces"].as_i64().unwrap_or(0));
    let unset_number = matches!(meta.value_type, ValueType::Float | ValueType::Double | ValueType::ElapsedSeconds);
    if unset_number && meta.default == Some(Value::Null) {
        described["defaultValueString"] = json!(text);
    }
    if meta.value_type == ValueType::Bool && raw.is_number() {
        let truth = raw.as_f64().is_some_and(|n| n != 0.0);
        described["valueString"] = json!(truth.to_string());
        described["enumOrValueString"] = json!(truth.to_string());
        described["valueEqualsDefault"] = json!(meta.default.as_ref().and_then(Value::as_bool) == Some(truth));
    }
    if raw.is_null() && unset_number {
        described["valueEqualsDefault"] = json!(false);
        described["valueString"] = json!(text);
        described["enumOrValueString"] = json!(text);
        described["unknownEnumLabel"] = json!("Unknown: nan");
        described["value"] = Value::Null;
    }
    if let Some(name) = property {
        described["property"] = json!(name);
    }
    described
}

const BATTERY_FACT_NAMES: [&str; 12] = ["id", "batteryFunction", "batteryType", "voltage", "current", "mahConsumed", "temperature", "percentRemaining", "timeRemaining", "timeRemainingStr", "chargeState", "instantPower"];

const BATTERY_PROPERTIES: [(&str, &str); 12] = [
    ("id", "id"),
    ("function", "batteryFunction"),
    ("type", "batteryType"),
    ("temperature", "temperature"),
    ("voltage", "voltage"),
    ("current", "current"),
    ("mahConsumed", "mahConsumed"),
    ("percentRemaining", "percentRemaining"),
    ("timeRemaining", "timeRemaining"),
    ("timeRemainingStr", "timeRemainingStr"),
    ("chargeState", "chargeState"),
    ("instantPower", "instantPower"),
];

fn time_remaining_text(seconds: Option<f64>) -> String {
    seconds.map_or_else(
        || "––:––:––".to_string(),
        |s| {
            let total = s as i64;
            format!("{:02}H:{:02}M:{:02}S", total / 3600, (total % 3600) / 60, total % 60)
        },
    )
}

fn battery_raw(id: u8, battery: &BatteryFacts, name: &str) -> Value {
    let number = |v: Option<f64>| v.map_or(Value::Null, |n| json!(n));
    match name {
        "id" => json!(id),
        "batteryFunction" => json!(battery.function),
        "batteryType" => json!(battery.kind),
        "voltage" => number(battery.voltage),
        "current" => number(battery.current),
        "mahConsumed" => number(battery.mah_consumed),
        "temperature" => number(battery.temperature),
        "percentRemaining" => number(battery.percent_remaining),
        "timeRemaining" => number(battery.time_remaining),
        "timeRemainingStr" => json!(time_remaining_text(battery.time_remaining)),
        "chargeState" => json!(battery.charge_state),
        "instantPower" => number(battery.instant_power),
        _ => Value::Null,
    }
}

pub fn battery_fact(id: u8, battery: &BatteryFacts, name: &str, property: Option<&str>) -> Option<Value> {
    let meta = crate::factmeta::from_file(BATTERY_META).ok()?.remove(name)?;
    Some(fact(&meta, &battery_raw(id, battery, name), property))
}

fn group(class: &str, fact_names: &[&str], facts: Vec<Value>, telemetry: bool) -> Value {
    json!({
        "children": [],
        "class": class,
        "factGroupNames": [],
        "factNames": fact_names,
        "facts": facts,
        "kind": "object",
        "objectName": "",
        "telemetryAvailable": telemetry,
    })
}

pub fn battery_group(id: u8, battery: &BatteryFacts) -> Value {
    let facts: Vec<Value> = BATTERY_PROPERTIES.iter().filter_map(|(property, name)| battery_fact(id, battery, name, Some(property))).collect();
    group("BatteryFactGroup", &BATTERY_FACT_NAMES, facts, battery.telemetry)
}

const VIBRATION_META: &str = include_str!("../../src/Vehicle/FactGroups/VibrationFact.json");
const VIBRATION_FACT_NAMES: [&str; 6] = ["xAxis", "yAxis", "zAxis", "clipCount1", "clipCount2", "clipCount3"];

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VibrationFacts {
    pub axes: [Option<f64>; 3],
    pub clipping: [u32; 3],
    pub telemetry: bool,
}

impl VibrationFacts {
    pub fn apply(&mut self, message: &mavlink::dialects::ardupilotmega::MavMessage) {
        if let mavlink::dialects::ardupilotmega::MavMessage::VIBRATION(v) = message {
            *self = VibrationFacts { axes: [Some(f64::from(v.vibration_x)), Some(f64::from(v.vibration_y)), Some(f64::from(v.vibration_z))], clipping: [v.clipping_0, v.clipping_1, v.clipping_2], telemetry: true };
        }
    }

    fn raw(&self, name: &str) -> Option<Value> {
        let index = VIBRATION_FACT_NAMES.iter().position(|n| *n == name)?;
        Some(match index {
            0..=2 => self.axes[index].map_or(Value::Null, |v| json!(v)),
            _ => json!(self.clipping[index - 3]),
        })
    }
}

pub fn vibration_fact(vibration: &VibrationFacts, name: &str, property: Option<&str>) -> Option<Value> {
    let meta = crate::factmeta::from_file(VIBRATION_META).ok()?.remove(name)?;
    Some(fact(&meta, &vibration.raw(name)?, property))
}

pub fn vibration_group(vibration: &VibrationFacts) -> Value {
    let facts = VIBRATION_FACT_NAMES.iter().filter_map(|name| vibration_fact(vibration, name, Some(name))).collect();
    group("VehicleVibrationFactGroup", &VIBRATION_FACT_NAMES, facts, vibration.telemetry)
}

pub fn battery_list(batteries: &[(u8, BatteryFacts)]) -> Value {
    json!({
        "children": [],
        "class": "BatteryFactGroupListModel",
        "count": batteries.len(),
        "dirty": true,
        "elements": batteries.iter().map(|(id, b)| battery_group(*id, b)).collect::<Vec<_>>(),
        "facts": [],
        "kind": "object",
        "objectName": "",
    })
}

pub fn battery_by_property(property: &str) -> Option<&'static str> {
    BATTERY_PROPERTIES.iter().find(|(p, _)| *p == property).map(|(_, n)| *n)
}

pub struct GroupSpec {
    pub class: &'static str,
    pub meta: &'static str,
    pub properties: &'static [(&'static str, &'static str)],
    pub added: &'static [&'static str],
}

pub fn spec_fact(spec: &GroupSpec, name: &str, raw: &Value, property: Option<&str>) -> Option<Value> {
    let meta = crate::factmeta::from_file(spec.meta).ok()?.remove(name)?;
    Some(fact(&meta, raw, property))
}

pub fn spec_group(spec: &GroupSpec, raw: impl Fn(&str) -> Value, telemetry: bool) -> Value {
    let facts = spec.properties.iter().filter_map(|(property, name)| spec_fact(spec, name, &raw(name), Some(property))).collect();
    group(spec.class, spec.added, facts, telemetry)
}

pub fn spec_group_fields(spec: &GroupSpec, fields: &str, raw: impl Fn(&str) -> Value) -> Option<Value> {
    let wanted: Vec<&str> = fields.split(',').map(str::trim).filter(|f| !f.is_empty()).collect();
    wanted.iter().all(|w| spec.properties.iter().any(|(p, _)| p == w)).then_some(())?;
    let facts: Vec<Value> = spec
        .properties
        .iter()
        .filter(|(property, _)| wanted.contains(property))
        .filter_map(|(property, name)| {
            let full = spec_fact(spec, name, &raw(name), None)?;
            Some(json!({ "kind": "fact", "name": full["name"], "value": full["value"], "valueString": full["valueString"], "rawValue": full["rawValue"], "units": full["units"], "property": property }))
        })
        .collect();
    Some(json!({ "kind": "object", "class": spec.class, "facts": facts, "children": [] }))
}

pub fn spec_property(spec: &GroupSpec, property: &str, raw: impl Fn(&str) -> Value) -> Option<Value> {
    let name = spec.properties.iter().find(|(p, _)| *p == property)?.1;
    spec_fact(spec, name, &raw(name), None)
}

pub const ESTIMATOR: GroupSpec = GroupSpec {
    class: "VehicleEstimatorStatusFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/EstimatorStatusFactGroup.json"),
    properties: &[
        ("goodAttitudeEstimate", "goodAttitudeEsimate"),
        ("goodHorizVelEstimate", "goodHorizVelEstimate"),
        ("goodVertVelEstimate", "goodVertVelEstimate"),
        ("goodHorizPosRelEstimate", "goodHorizPosRelEstimate"),
        ("goodHorizPosAbsEstimate", "goodHorizPosAbsEstimate"),
        ("goodVertPosAbsEstimate", "goodVertPosAbsEstimate"),
        ("goodVertPosAGLEstimate", "goodVertPosAGLEstimate"),
        ("goodConstPosModeEstimate", "goodConstPosModeEstimate"),
        ("goodPredHorizPosRelEstimate", "goodPredHorizPosRelEstimate"),
        ("goodPredHorizPosAbsEstimate", "goodPredHorizPosAbsEstimate"),
        ("gpsGlitch", "gpsGlitch"),
        ("accelError", "accelError"),
        ("velRatio", "velRatio"),
        ("horizPosRatio", "horizPosRatio"),
        ("vertPosRatio", "vertPosRatio"),
        ("magRatio", "magRatio"),
        ("haglRatio", "haglRatio"),
        ("tasRatio", "tasRatio"),
        ("horizPosAccuracy", "horizPosAccuracy"),
        ("vertPosAccuracy", "vertPosAccuracy"),
    ],
    added: &[
        "goodAttitudeEsimate", "goodHorizVelEstimate", "goodVertVelEstimate", "goodHorizPosRelEstimate", "goodHorizPosAbsEstimate", "goodVertPosAbsEstimate", "goodVertPosAGLEstimate", "goodConstPosModeEstimate",
        "goodPredHorizPosRelEstimate", "goodPredHorizPosAbsEstimate", "gpsGlitch", "accelError", "velRatio", "horizPosRatio", "vertPosRatio", "magRatio", "haglRatio", "tasRatio", "horizPosAccuracy", "vertPosAccuracy",
    ],
};

pub fn estimator_raw(e: &crate::sensorfacts::EstimatorStatusFacts, name: &str) -> Value {
    let flag = !e.seen && ESTIMATOR.added.iter().position(|n| *n == name).is_some_and(|i| i < 12);
    let ratio = !e.seen && !flag;
    if flag {
        return json!(0);
    }
    if ratio {
        return Value::Null;
    }
    match name {
        "goodAttitudeEsimate" => json!(e.good_attitude),
        "goodHorizVelEstimate" => json!(e.good_horiz_vel),
        "goodVertVelEstimate" => json!(e.good_vert_vel),
        "goodHorizPosRelEstimate" => json!(e.good_horiz_pos_rel),
        "goodHorizPosAbsEstimate" => json!(e.good_horiz_pos_abs),
        "goodVertPosAbsEstimate" => json!(e.good_vert_pos_abs),
        "goodVertPosAGLEstimate" => json!(e.good_vert_pos_agl),
        "goodConstPosModeEstimate" => json!(e.good_const_pos_mode),
        "goodPredHorizPosRelEstimate" => json!(e.good_pred_horiz_pos_rel),
        "goodPredHorizPosAbsEstimate" => json!(e.good_pred_horiz_pos_abs),
        "gpsGlitch" => json!(e.gps_glitch),
        "accelError" => json!(e.accel_error),
        "velRatio" => json!(e.vel_ratio),
        "horizPosRatio" => json!(e.horiz_pos_ratio),
        "vertPosRatio" => json!(e.vert_pos_ratio),
        "magRatio" => json!(e.mag_ratio),
        "haglRatio" => json!(e.hagl_ratio),
        "tasRatio" => json!(e.tas_ratio),
        "horizPosAccuracy" => json!(e.horiz_pos_accuracy),
        "vertPosAccuracy" => json!(e.vert_pos_accuracy),
        _ => Value::Null,
    }
}

const ROTATIONS: [(&str, u32); 10] = [("rotationNone", 0), ("rotationYaw45", 1), ("rotationYaw90", 2), ("rotationYaw135", 3), ("rotationYaw180", 4), ("rotationYaw225", 5), ("rotationYaw270", 6), ("rotationYaw315", 7), ("rotationPitch90", 24), ("rotationPitch270", 25)];

pub const DISTANCE: GroupSpec = GroupSpec {
    class: "VehicleDistanceSensorFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/DistanceSensorFact.json"),
    properties: &[
        ("rotationNone", "rotationNone"),
        ("rotationYaw45", "rotationYaw45"),
        ("rotationYaw90", "rotationYaw90"),
        ("rotationYaw135", "rotationYaw135"),
        ("rotationYaw180", "rotationYaw180"),
        ("rotationYaw225", "rotationYaw225"),
        ("rotationYaw270", "rotationYaw270"),
        ("rotationYaw315", "rotationYaw315"),
        ("rotationPitch90", "rotationPitch90"),
        ("rotationPitch270", "rotationPitch270"),
        ("minDistance", "minDistance"),
        ("maxDistance", "maxDistance"),
    ],
    added: &["rotationNone", "rotationYaw45", "rotationYaw90", "rotationYaw135", "rotationYaw180", "rotationYaw225", "rotationYaw270", "rotationYaw315", "rotationPitch90", "rotationPitch270", "minDistance", "maxDistance"],
};

pub fn distance_raw(d: &crate::sensorfacts::DistanceSensorFacts, name: &str, unset: &Value) -> Value {
    match name {
        "minDistance" if d.seen => json!(d.min_distance),
        "maxDistance" if d.seen => json!(d.max_distance),
        rotation => ROTATIONS.iter().find(|(n, _)| *n == rotation).and_then(|(_, orientation)| d.by_orientation.get(orientation)).map_or_else(|| unset.clone(), |v| json!(v)),
    }
}

pub const RADIO: GroupSpec = GroupSpec {
    class: "RadioStatusFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/RadioStatusFact.json"),
    properties: &[("lrssi", "lrssi"), ("rrssi", "rrssi"), ("rxErrors", "rxErrors"), ("fixed", "fixed"), ("txBuffer", "txBuffer"), ("lNoise", "lNoise"), ("rNoise", "rNoise")],
    added: &["lrssi", "rrssi", "rxErrors", "fixed", "txBuffer", "lNoise", "rNoise"],
};

const SIK_SYSTEM: u8 = b'3';
const SIK_COMPONENT: u8 = b'D';

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RadioStatusFacts {
    pub values: [i64; 7],
    pub telemetry: bool,
}

impl RadioStatusFacts {
    pub fn apply(&mut self, from: (u8, u8), message: &mavlink::dialects::ardupilotmega::MavMessage) {
        let mavlink::dialects::ardupilotmega::MavMessage::RADIO_STATUS(r) = message else { return };
        let sik = |raw: u8| ((f64::from(raw) / 1.9 - 127.0).round() as i64).clamp(-120, 0);
        let signed = |raw: u8| i64::from(raw as i8);
        let (rssi, remote) = match from == (SIK_SYSTEM, SIK_COMPONENT) {
            true => (sik(r.rssi), sik(r.remrssi)),
            false => (signed(r.rssi), signed(r.remrssi)),
        };
        *self = RadioStatusFacts { values: [rssi, remote, i64::from(r.rxerrors), i64::from(r.fixed), i64::from(r.txbuf), signed(r.noise), signed(r.remnoise)], telemetry: true };
    }

    pub fn raw(&self, name: &str) -> Value {
        RADIO.added.iter().position(|n| *n == name).map_or(Value::Null, |i| json!(self.values[i]))
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ObstacleFacts {
    pub distances: Vec<i64>,
    pub increment: f64,
    pub min_distance: i64,
    pub max_distance: i64,
    pub angle_offset: f64,
    pub updated_ms: Option<u64>,
}

impl ObstacleFacts {
    pub fn apply(&mut self, message: &mavlink::dialects::ardupilotmega::MavMessage, now_ms: u64) {
        let mavlink::dialects::ardupilotmega::MavMessage::OBSTACLE_DISTANCE(o) = message else { return };
        let fine = f64::from(o.increment_f);
        *self = ObstacleFacts {
            distances: o.distances.iter().map(|d| i64::from(*d)).collect(),
            increment: if fine.is_finite() && fine > 0.0 { fine } else { f64::from(o.increment) },
            min_distance: i64::from(o.min_distance),
            max_distance: i64::from(o.max_distance),
            angle_offset: f64::from(o.angle_offset),
            updated_ms: Some(now_ms),
        };
    }

    pub fn field(&self, name: &str, enabled: bool, now_ms: u64) -> Option<Value> {
        Some(match name {
            "available" => json!(!self.distances.is_empty()),
            "enabled" => json!(enabled),
            "distances" => json!(self.distances),
            "increment" => json!(self.increment),
            "minDistance" => json!(self.min_distance),
            "maxDistance" => json!(self.max_distance),
            "angleOffset" => json!(self.angle_offset),
            "msSinceUpdate" => json!(self.updated_ms.map_or(-1, |at| i64::try_from(now_ms.saturating_sub(at)).unwrap_or(i64::MAX))),
            _ => return None,
        })
    }
}

pub const TEMPERATURE: GroupSpec = GroupSpec {
    class: "VehicleTemperatureFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/TemperatureFact.json"),
    properties: &[("temperature1", "temperature1"), ("temperature2", "temperature2"), ("temperature3", "temperature3")],
    added: &["temperature1", "temperature2", "temperature3"],
};

pub fn temperature_raw(t: &crate::sensorfacts::TemperatureFacts, name: &str) -> Value {
    let (value, seen) = match name {
        "temperature1" => (t.temperature1, t.seen[0]),
        "temperature2" => (t.temperature2, t.seen[1]),
        "temperature3" => (t.temperature3, t.seen[2]),
        _ => return Value::Null,
    };
    if seen { json!(value) } else { Value::Null }
}

pub const EFI: GroupSpec = GroupSpec {
    class: "VehicleEFIFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/EFIFact.json"),
    properties: &[
        ("health", "health"),
        ("ecuIndex", "ecuIndex"),
        ("rpm", "rpm"),
        ("fuelConsumed", "fuelConsumed"),
        ("fuelFlow", "fuelFlow"),
        ("engineLoad", "engineLoad"),
        ("throttlePos", "throttlePos"),
        ("sparkTime", "sparkTime"),
        ("baroPress", "baroPress"),
        ("intakePress", "intakePress"),
        ("intakeTemp", "intakeTemp"),
        ("cylinderTemp", "cylinderTemp"),
        ("ignTime", "ignTime"),
        ("injTime", "injTime"),
        ("exGasTemp", "exGasTemp"),
        ("throttleOut", "throttleOut"),
        ("ptComp", "ptComp"),
        ("ignVoltage", "ignVoltage"),
        ("fuelPressure", "fuelPressure"),
    ],
    added: &[
        "health", "ecuIndex", "rpm", "fuelConsumed", "fuelFlow", "engineLoad", "sparkTime", "throttlePos", "baroPress", "intakePress", "intakeTemp", "cylinderTemp", "ignTime", "exGasTemp", "injTime", "throttleOut", "ptComp", "ignVoltage", "fuelPressure",
    ],
};

pub fn efi_raw(e: &crate::sensorfacts::EfiFacts, name: &str) -> Value {
    match (name, e.seen) {
        ("health", _) => json!(e.health),
        (_, false) => Value::Null,
        (reading, true) => e.reading(reading).map_or(Value::Null, |v| json!(f64::from(v))),
    }
}

pub const GENERATOR: GroupSpec = GroupSpec {
    class: "VehicleGeneratorFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/GeneratorFact.json"),
    properties: &[
        ("status", "status"),
        ("genSpeed", "genSpeed"),
        ("batteryCurrent", "batteryCurrent"),
        ("loadCurrent", "loadCurrent"),
        ("powerGenerated", "powerGenerated"),
        ("busVoltage", "busVoltage"),
        ("rectifierTemp", "rectifierTemp"),
        ("batCurrentSetpoint", "batCurrentSetpoint"),
        ("genTemp", "genTemp"),
        ("runtime", "runtime"),
        ("timeMaintenance", "timeMaintenance"),
    ],
    added: &["status", "genSpeed", "batteryCurrent", "loadCurrent", "powerGenerated", "busVoltage", "batCurrentSetpoint", "rectifierTemp", "genTemp", "runtime", "timeMaintenance"],
};

pub fn generator_raw(g: &crate::sensorfacts::GeneratorFacts, name: &str) -> Value {
    let measured = |value: f32| if g.seen { json!(f64::from(value)) } else { Value::Null };
    match name {
        "status" => json!(g.status),
        "genSpeed" => json!(g.speed),
        "batteryCurrent" => measured(g.battery_current),
        "loadCurrent" => measured(g.load_current),
        "powerGenerated" => measured(g.power_generated),
        "busVoltage" => measured(g.bus_voltage),
        "batCurrentSetpoint" => measured(g.battery_current_setpoint),
        "rectifierTemp" => json!(g.rectifier_temperature),
        "genTemp" => json!(g.generator_temperature),
        "runtime" => json!(g.runtime),
        "timeMaintenance" => json!(g.time_until_maintenance),
        _ => Value::Null,
    }
}

pub fn generator_flags(g: &crate::sensorfacts::GeneratorFacts) -> Value {
    json!(if g.status_changed { vec![0; 23] } else { Vec::new() })
}

pub const HYGROMETER: GroupSpec = GroupSpec {
    class: "VehicleHygrometerFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/HygrometerFact.json"),
    properties: &[("hygroID", "hygrometerid"), ("hygroTemp", "temperature"), ("hygroHumi", "humidity")],
    added: &["temperature", "humidity", "hygrometerid"],
};

pub fn hygrometer_raw(h: &crate::sensorfacts::HygrometerFacts, name: &str) -> Value {
    match (name, h.seen) {
        ("hygrometerid", _) => json!(h.id),
        ("temperature", true) => json!(h.temperature),
        ("humidity", true) => json!(h.humidity),
        _ => Value::Null,
    }
}

pub const SETPOINT: GroupSpec = GroupSpec {
    class: "VehicleSetpointFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/SetpointFact.json"),
    properties: &[("roll", "roll"), ("pitch", "pitch"), ("yaw", "yaw"), ("rollRate", "rollRate"), ("pitchRate", "pitchRate"), ("yawRate", "yawRate")],
    added: &["roll", "pitch", "yaw", "rollRate", "pitchRate", "yawRate"],
};

pub fn setpoint_raw(s: &crate::sensorfacts::SetpointFacts, name: &str) -> Value {
    let value = match name {
        "roll" => s.roll,
        "pitch" => s.pitch,
        "yaw" => s.yaw,
        "rollRate" => s.roll_rate,
        "pitchRate" => s.pitch_rate,
        "yawRate" => s.yaw_rate,
        _ => return Value::Null,
    };
    if s.seen { json!(value) } else { Value::Null }
}

pub const WIND: GroupSpec = GroupSpec {
    class: "VehicleWindFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/WindFact.json"),
    properties: &[("direction", "direction"), ("speed", "speed"), ("verticalSpeed", "verticalSpeed")],
    added: &["direction", "speed", "verticalSpeed"],
};

pub fn wind_raw(w: &crate::sensorfacts::WindFacts, name: &str) -> Value {
    let (value, seen) = match name {
        "direction" => (w.direction, w.seen[0]),
        "speed" => (w.speed, w.seen[1]),
        "verticalSpeed" => (w.vertical_speed, w.seen[2]),
        _ => return Value::Null,
    };
    if seen { json!(value) } else { Value::Null }
}

pub const LOCAL_POSITION: GroupSpec = GroupSpec {
    class: "VehicleLocalPositionFactGroup",
    meta: include_str!("../../src/Vehicle/FactGroups/LocalPositionFact.json"),
    properties: &[("x", "x"), ("y", "y"), ("z", "z"), ("vx", "vx"), ("vy", "vy"), ("vz", "vz")],
    added: &["x", "y", "z", "vx", "vy", "vz"],
};

pub const LOCAL_POSITION_SETPOINT: GroupSpec = GroupSpec { class: "VehicleLocalPositionSetpointFactGroup", ..LOCAL_POSITION };

pub fn local_position_raw(p: &crate::sensorfacts::LocalPositionFacts, name: &str) -> Value {
    let value = match name {
        "x" => p.x,
        "y" => p.y,
        "z" => p.z,
        "vx" => p.vx,
        "vy" => p.vy,
        "vz" => p.vz,
        _ => return Value::Null,
    };
    if p.seen { json!(value) } else { Value::Null }
}

const VEHICLE_META: &str = include_str!("../../src/Vehicle/FactGroups/VehicleFact.json");

const CIRCLE_META: &str = include_str!("../../src/QmlControls/QGCMapCircle.Facts.json");

fn orbit_center(circle: Option<(f32, i32, i32)>) -> Option<Value> {
    circle.map(|(_, x, y)| json!({ "altitude": null, "latitude": f64::from(x) / 1e7, "longitude": f64::from(y) / 1e7, "valid": true }))
}

fn orbit_radius(circle: Option<(f32, i32, i32)>, property: Option<&str>) -> Option<Value> {
    let meta = crate::factmeta::from_file(CIRCLE_META).ok()?.remove("Radius")?;
    Some(fact(&meta, &json!(circle.map_or(0.0, |(radius, _, _)| f64::from(radius).abs())), property))
}

pub fn orbit_circle(circle: Option<(f32, i32, i32)>) -> Option<Value> {
    Some(json!({
        "center": orbit_center(circle),
        "children": [],
        "class": "QGCMapCircle",
        "clockwiseRotation": circle.is_none_or(|(radius, _, _)| radius > 0.0),
        "dirty": circle.is_some(),
        "facts": [orbit_radius(circle, Some("radius"))?],
        "interactive": false,
        "kind": "object",
        "objectName": "",
        "showRotation": circle.is_some(),
    }))
}

pub fn orbit_circle_part(circle: Option<(f32, i32, i32)>, part: &str) -> Option<Value> {
    match part {
        "radius" => orbit_radius(circle, None),
        "center" => Some(orbit_center(circle).map_or_else(|| json!({ "altitude": null, "kind": "coordinate", "latitude": null, "longitude": null, "valid": false }), |mut c| {
            c["kind"] = json!("coordinate");
            c
        })),
        field => orbit_circle(circle)?.get(field).filter(|v| v.is_boolean()).map(|v| json!({ "kind": "value", "value": v })),
    }
}

pub fn vehicle_fact(name: &str, raw: &Value) -> Option<Value> {
    let meta = crate::factmeta::from_file(VEHICLE_META).ok()?.remove(name)?;
    Some(fact(&meta, raw, None))
}

pub fn compact(full: &Value, property: &str) -> Value {
    json!({ "kind": "fact", "name": full["name"], "value": full["value"], "valueString": full["valueString"], "rawValue": full["rawValue"], "units": full["units"], "property": property })
}

const RSSI_UNKNOWN: u8 = 255;

#[derive(Debug, Clone, PartialEq)]
pub struct RcRssi {
    store: f64,
    pub shown: u8,
}

impl Default for RcRssi {
    fn default() -> Self {
        RcRssi { store: f64::from(RSSI_UNKNOWN), shown: RSSI_UNKNOWN }
    }
}

impl RcRssi {
    pub fn apply(&mut self, message: &mavlink::dialects::ardupilotmega::MavMessage, ardupilot: bool) {
        let mavlink::dialects::ardupilotmega::MavMessage::RC_CHANNELS(c) = message else { return };
        let raw = [c.chan1_raw, c.chan2_raw, c.chan3_raw, c.chan4_raw, c.chan5_raw, c.chan6_raw, c.chan7_raw, c.chan8_raw, c.chan9_raw, c.chan10_raw, c.chan11_raw, c.chan12_raw, c.chan13_raw, c.chan14_raw, c.chan15_raw, c.chan16_raw, c.chan17_raw, c.chan18_raw];
        let valid = raw.iter().filter(|v| **v != u16::MAX).count();
        let first_unused = raw.iter().position(|v| *v == u16::MAX);
        if first_unused.is_some_and(|at| at != valid) {
            return;
        }
        let rssi = match (ardupilot, c.rssi) {
            (true, r) if r != 0 && r != RSSI_UNKNOWN => ((f64::from(r) / 254.0) * 100.0) as u8,
            (_, r) => r,
        };
        if rssi > 100 {
            self.shown = RSSI_UNKNOWN;
            return;
        }
        if self.store == f64::from(RSSI_UNKNOWN) {
            self.store = f64::from(rssi);
        }
        self.store = self.store.mul_add(0.9, f64::from(rssi) * 0.1);
        self.shown = if self.store < 0.1 { 0 } else { self.store.ceil() as u8 };
    }
}

pub fn mgrs(latitude: f64, longitude: f64) -> String {
    let Ok(position) = geoconvert::LatLon::create(latitude, longitude) else { return String::new() };
    let packed = geoconvert::Mgrs::from_latlon(&position, 5).to_string();
    let digits_from = packed.rfind(|c: char| !c.is_ascii_digit()).map_or(0, |i| i + 1);
    let half = (packed.len() - digits_from) / 2;
    format!("{} {} {}", &packed[..digits_from], &packed[digits_from..digits_from + half], &packed[digits_from + half..])
}

fn gps_raw(gps: &crate::gpsfacts::GpsFacts, name: &str) -> Option<Value> {
    let number = |v: Option<f64>| v.map_or(Value::Null, |n| json!(n));
    Some(match name {
        "lat" => number(gps.latitude),
        "lon" => number(gps.longitude),
        "mgrs" => json!(gps.latitude.zip(gps.longitude).map_or_else(String::new, |(lat, lon)| mgrs(lat, lon))),
        "hdop" => number(gps.hdop),
        "vdop" => number(gps.vdop),
        "courseOverGround" => number(gps.course_over_ground),
        "yaw" => number(gps.yaw),
        "count" => json!(gps.count),
        "lock" => json!(gps.lock),
        _ => return None,
    })
}

pub fn gps_fact(gps: &crate::gpsfacts::GpsFacts, name: &str) -> Option<Value> {
    GPS_ANSWERED.contains(&name).then_some(())?;
    let meta = crate::factmeta::from_file(GPS_META).ok()?.remove(name)?;
    Some(fact(&meta, &gps_raw(gps, name)?, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_reading_reads_as_dashes_and_a_known_one_in_its_decimals() {
        let battery = BatteryFacts { voltage: Some(12.6), charge_state: 1, ..BatteryFacts::default() };
        let voltage = battery_fact(0, &battery, "voltage", None).unwrap();
        assert_eq!((voltage["valueString"].as_str(), voltage["units"].as_str(), voltage["visible"].clone()), (Some("12.60"), Some("v"), Value::Null));
        let current = battery_fact(0, &battery, "current", None).unwrap();
        assert_eq!((current["valueString"].as_str(), current["unknownEnumLabel"].as_str(), current["rawValue"].clone()), (Some("–.––"), Some("Unknown: nan"), Value::Null));
        let charge = battery_fact(0, &battery, "chargeState", None).unwrap();
        assert_eq!((charge["valueString"].as_str(), charge["enumOrValueString"].as_str()), (Some("1"), Some("Ok")));
        assert_eq!(time_remaining_text(Some(3725.0)), "01H:02M:05S");
        assert_eq!(battery_group(0, &battery)["facts"][1]["property"], "function", "a group lists its facts by the property that holds them");
    }

    #[test]
    fn the_orbit_circle_latches_the_last_status() {
        let idle = orbit_circle(None).unwrap();
        assert_eq!((idle["center"].clone(), idle["dirty"].clone(), idle["clockwiseRotation"].clone(), idle["facts"][0]["rawValue"].clone()), (Value::Null, json!(false), json!(true), json!(0.0)));
        let heard = orbit_circle(Some((-25.0, -353632000, 1491652000))).unwrap();
        assert_eq!((heard["center"]["latitude"].clone(), heard["clockwiseRotation"].clone(), heard["showRotation"].clone(), heard["facts"][0]["valueString"].clone()), (json!(-35.3632), json!(false), json!(true), json!("25.0")));
        assert_eq!(orbit_circle_part(None, "center").unwrap()["valid"], false);
        assert_eq!(orbit_circle_part(None, "radius").unwrap().get("property"), None, "a direct read of the fact names no holder");
    }

    #[test]
    fn vibration_reads_as_dashes_until_the_first_report() {
        let quiet = VibrationFacts::default();
        assert_eq!(vibration_fact(&quiet, "xAxis", None).unwrap()["valueString"], "–.–");
        assert_eq!(vibration_group(&quiet)["telemetryAvailable"], false);
        let mut heard = VibrationFacts::default();
        heard.apply(&mavlink::dialects::ardupilotmega::MavMessage::VIBRATION(mavlink::dialects::ardupilotmega::VIBRATION_DATA { vibration_x: 0.25, clipping_2: 7, ..Default::default() }));
        assert_eq!((vibration_fact(&heard, "xAxis", None).unwrap()["valueString"].as_str(), vibration_fact(&heard, "clipCount3", None).unwrap()["value"].as_u64()), (Some("0.3"), Some(7)));
    }

    #[test]
    fn estimator_and_distance_groups_read_their_reports() {
        use mavlink::dialects::ardupilotmega::{DISTANCE_SENSOR_DATA, ESTIMATOR_STATUS_DATA, EstimatorStatusFlags, MavMessage, MavSensorOrientation};
        let mut estimator = crate::sensorfacts::EstimatorStatusFacts::default();
        assert_eq!(spec_property(&ESTIMATOR, "goodAttitudeEstimate", |n| estimator_raw(&estimator, n)).unwrap()["rawValue"], 0, "an unreported flag is Qt's integer zero");
        estimator.apply(&MavMessage::ESTIMATOR_STATUS(ESTIMATOR_STATUS_DATA { flags: EstimatorStatusFlags::ESTIMATOR_ATTITUDE, vel_ratio: 0.5, ..Default::default() }));
        let attitude = spec_property(&ESTIMATOR, "goodAttitudeEstimate", |n| estimator_raw(&estimator, n)).unwrap();
        assert_eq!((attitude["name"].as_str(), attitude["valueString"].as_str()), (Some("goodAttitudeEsimate"), Some("true")), "the property and Qt's misspelt fact name both hold");
        assert_eq!(spec_property(&ESTIMATOR, "velRatio", |n| estimator_raw(&estimator, n)).unwrap()["valueString"], "0.50");
        let mut distance = crate::sensorfacts::DistanceSensorFacts::default();
        distance.apply(&MavMessage::DISTANCE_SENSOR(DISTANCE_SENSOR_DATA { current_distance: 250, max_distance: 4000, orientation: MavSensorOrientation::MAV_SENSOR_ROTATION_PITCH_270, ..Default::default() }));
        let raw = |n: &str| distance_raw(&distance, n, &Value::Null);
        assert_eq!((raw("rotationPitch270"), raw("maxDistance"), raw("rotationNone")), (json!(2.5), json!(40.0), Value::Null));
    }

    #[test]
    fn rc_signal_filters_like_the_arm64_qt_build() {
        use mavlink::dialects::ardupilotmega::{MavMessage, RC_CHANNELS_DATA};
        let frame = |rssi: u8| MavMessage::RC_CHANNELS(RC_CHANNELS_DATA { chancount: 8, chan1_raw: 1500, chan2_raw: 1500, chan3_raw: 1500, chan4_raw: 1500, chan5_raw: 1500, chan6_raw: 1500, chan7_raw: 1500, chan8_raw: 1500, rssi, ..Default::default() });
        let mut signal = RcRssi::default();
        assert_eq!(signal.shown, 255, "unknown until a strength is reported");
        (0..5).for_each(|_| signal.apply(&frame(80), true));
        assert_eq!(signal.shown, 31, "ArduPilot's 0-254 becomes 31 percent, and the filter holds it there: unfused, 31 * 0.9 + 3.1 is 31.000000000000004 and would read 32");
        signal.apply(&frame(255), true);
        assert_eq!(signal.shown, 255);
    }

    #[test]
    fn mgrs_is_spelled_as_qgc_spaces_it() {
        assert_eq!(mgrs(-35.3632616, 149.1652372), "55HFA 96719 84519");
        assert_eq!(mgrs(417_189_529.0 * 1e-7, 448_281_746.0 * 1e-7), "38TMM 85707 18586", "both pairs read off one snapshot of Qt's vehicle.gps group");
        let gps = crate::gpsfacts::GpsFacts::default();
        assert_eq!(gps_fact(&gps, "mgrs").unwrap()["valueString"], "", "no position, no grid reference");
        assert_eq!(gps_fact(&gps, "lat").unwrap()["valueString"], "–.–––––––");
        assert!(gps_fact(&gps, "spoofingState").is_none(), "the integrity facts stay with the host until the hub can read GNSS_INTEGRITY");
    }
}
