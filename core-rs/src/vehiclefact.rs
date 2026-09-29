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
    fn mgrs_is_spelled_as_qgc_spaces_it() {
        assert_eq!(mgrs(-35.3632616, 149.1652372), "55HFA 96719 84519");
        assert_eq!(mgrs(417_189_529.0 * 1e-7, 448_281_746.0 * 1e-7), "38TMM 85707 18586", "both pairs read off one snapshot of Qt's vehicle.gps group");
        let gps = crate::gpsfacts::GpsFacts::default();
        assert_eq!(gps_fact(&gps, "mgrs").unwrap()["valueString"], "", "no position, no grid reference");
        assert_eq!(gps_fact(&gps, "lat").unwrap()["valueString"], "–.–––––––");
        assert!(gps_fact(&gps, "spoofingState").is_none(), "the integrity facts stay with the host until the hub can read GNSS_INTEGRITY");
    }
}
