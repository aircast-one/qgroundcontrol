use serde_json::{Value, json};

use crate::control::decode;
use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady", "vehicle.apmFirmware", "vehicle.sub"];
pub const SET_PRIORITY: &str = "sensorSettings.priority";
const COMPASSES: usize = 3;
const PRIORITY_NOT_SET: usize = 3;
const AUTO_ROT_ENABLED: f64 = 2.0;
pub const HELP_SET: &str = "If mounted in the direction of flight, select None.";
pub const HELP_CAL: &str = "Before calibrating make sure rotation settings are correct. If mounted in the direction of flight, select None.";
const ID_PARAMS: [&str; COMPASSES] = ["COMPASS_DEV_ID", "COMPASS_DEV_ID2", "COMPASS_DEV_ID3"];
const USE_PARAMS: [&str; COMPASSES] = ["COMPASS_USE", "COMPASS_USE2", "COMPASS_USE3"];
const EXTERNAL_PARAMS: [&str; COMPASSES] = ["COMPASS_EXTERNAL", "COMPASS_EXTERN2", "COMPASS_EXTERN3"];
const ORIENT_PARAMS: [&str; COMPASSES] = ["COMPASS_ORIENT", "COMPASS_ORIENT2", "COMPASS_ORIENT3"];
const PRIO_PARAMS: [&str; COMPASSES] = ["COMPASS_PRIO1_ID", "COMPASS_PRIO2_ID", "COMPASS_PRIO3_ID"];
const BUS_TYPES: [&str; 7] = ["-", "I2C", "SPI", "UAVCAN", "SITL", "MSP", "EAHRS"];
const COMPASS_TYPES: [(u32, &str); 23] = [
    (0x01, "HMC5883_OLD"),
    (0x07, "HMC5883"),
    (0x02, "LSM303D"),
    (0x04, "AK8963"),
    (0x05, "BMM150"),
    (0x06, "LSM9DS1"),
    (0x08, "LIS3MDL"),
    (0x09, "AK09916"),
    (0x0A, "IST8310"),
    (0x0B, "ICM20948"),
    (0x0C, "MMC3416"),
    (0x0D, "QMC5883L"),
    (0x0E, "MAG3110"),
    (0x0F, "SITL"),
    (0x10, "IST8308"),
    (0x11, "RM3100"),
    (0x12, "RM3100_2"),
    (0x13, "MMC5983"),
    (0x14, "AK09918"),
    (0x15, "AK09915"),
    (0x16, "QMC5883P"),
    (0x17, "BMM350"),
    (0x18, "IIS2MDC"),
];

pub fn decode_compass_id(devid: u32) -> String {
    if devid == 0 {
        return String::new();
    }
    let bus_type = BUS_TYPES.get((devid & 0x07) as usize).copied().unwrap_or("undefined");
    let bus = (devid >> 3) & 0x1F;
    let devtype = devid >> 16;
    let name = match bus_type {
        "UAVCAN" | "EAHRS" => bus_type,
        _ => COMPASS_TYPES.iter().find(|(id, _)| *id == devtype).map_or("?", |(_, name)| name),
    };
    format!("{name} ({bus_type}{bus})")
}

pub fn compass_label(index: usize, primary: Option<bool>, external: Option<bool>) -> String {
    let base = format!("Compass {} ", index + 1);
    let role = primary.map(|p| if p { "(primary" } else { "(secondary" });
    let place = external.map(|e| if e { "external" } else { "internal" });
    match (role, place) {
        (Some(role), Some(place)) => format!("{base}{role}, {place})"),
        (Some(role), None) => format!("{base}{role})"),
        (None, Some(place)) => format!("{base}({place})"),
        (None, None) => format!("{base})"),
    }
}

pub fn priority_of(id: f64, priorities: &[Option<f64>]) -> usize {
    priorities.iter().position(|p| *p == Some(id)).unwrap_or(PRIORITY_NOT_SET)
}

fn path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter(-1,{name})")
}

fn fact(backend: &dyn Backend, name: &str) -> Option<Value> {
    let fact = object(&backend.get(&path(name)));
    let present = fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
    present.then_some(fact)
}

fn number(backend: &dyn Backend, name: &str) -> Option<f64> {
    fact(backend, name).and_then(|f| f.get("rawValue").or(f.get("value")).and_then(Value::as_f64))
}

fn decoded(backend: &dyn Backend, name: &str) -> Option<Value> {
    fact(backend, name).map(|f| decode(&f, &path(name)))
}

pub fn sensor_settings_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "apmFirmware,sub"));
    let Some(board) = decoded(backend, "AHRS_ORIENTATION").filter(|_| flag(&vehicle, "apmFirmware")) else {
        return json!({ "kind": "object", "class": "SensorSettings", "available": false });
    };
    let primary = number(backend, "COMPASS_PRIMARY");
    let auto_rot = number(backend, "COMPASS_AUTO_ROT") == Some(AUTO_ROT_ENABLED);
    let priorities: Vec<Option<f64>> = PRIO_PARAMS.iter().map(|p| number(backend, p)).collect();
    let has_priority = priorities[0].is_some();
    let compasses: Vec<Value> = (0..COMPASSES)
        .filter_map(|i| {
            let id = number(backend, ID_PARAMS[i]).filter(|id| *id > 0.0)?;
            let is_primary = primary.map(|p| p as usize == i);
            let external = number(backend, EXTERNAL_PARAMS[i]).map(|e| e != 0.0);
            let use_fact = decoded(backend, USE_PARAMS[i]).filter(|_| is_primary != Some(true));
            let in_use = use_fact.is_some() && number(backend, USE_PARAMS[i]).is_some_and(|u| u != 0.0);
            Some(json!({
                "index": i,
                "label": compass_label(i, is_primary, external),
                "device": decode_compass_id(id as u32),
                "use": use_fact,
                "priority": (has_priority && in_use).then(|| priority_of(id, &priorities)),
                "orientation": (!auto_rot && external == Some(true)).then(|| decoded(backend, ORIENT_PARAMS[i])).flatten(),
            }))
        })
        .collect();
    let sub = flag(&vehicle, "sub");
    json!({
        "kind": "object",
        "class": "SensorSettings",
        "available": true,
        "boardRotation": board,
        "compasses": compasses,
        "priorities": ["Priority 1", "Priority 2", "Priority 3", "Not Set"],
        "helpSet": HELP_SET,
        "helpCal": HELP_CAL,
        "simpleAccelHelp": "Simple accelerometer calibration is less precise but allows calibrating without rotating the vehicle. Check this if you have a large/heavy vehicle.",
        "declination": sub.then(|| json!({ "manual": number(backend, "COMPASS_AUTODEC") == Some(0.0), "autoDecPath": path("COMPASS_AUTODEC"), "value": decoded(backend, "COMPASS_DEC") })),
    })
}

pub fn set_priority(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let index = |at: usize| given.get(at).and_then(Value::as_u64).map(|v| v as usize);
    let (Some(compass), Some(priority)) = (index(0).filter(|c| *c < COMPASSES), index(1)) else {
        return json!({ "ok": false, "reason": "sensorSettings.priority takes a compass and a priority slot" });
    };
    if priority >= PRIORITY_NOT_SET {
        return json!({ "ok": true });
    }
    let Some(id) = number(backend, ID_PARAMS[compass]) else { return json!({ "ok": false, "reason": "That compass is not on this vehicle." }) };
    object(&backend.set(&format!("{}.rawValue", path(PRIO_PARAMS[priority])), &json!({ "value": id }).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_ids_decode_like_the_qml_decoder() {
        assert_eq!(decode_compass_id(0), "");
        let ist8310_i2c1_addr12 = (0x0A << 16) | (0x0C << 8) | (1 << 3) | 1;
        assert_eq!(decode_compass_id(ist8310_i2c1_addr12), "IST8310 (I2C1)");
        assert_eq!(decode_compass_id((0x7D << 16) | 3), "UAVCAN (UAVCAN0)");
        assert_eq!(decode_compass_id(0x7F << 16 | 2), "? (SPI0)");
    }

    #[test]
    fn labels_and_priorities_follow_apm_sensors_component() {
        assert_eq!(compass_label(0, Some(true), Some(true)), "Compass 1 (primary, external)");
        assert_eq!(compass_label(1, None, Some(false)), "Compass 2 (internal)");
        assert_eq!(compass_label(2, Some(false), None), "Compass 3 (secondary)");
        assert_eq!(priority_of(97539.0, &[Some(1.0), Some(97539.0), None]), 1);
        assert_eq!(priority_of(5.0, &[Some(1.0), None, None]), 3, "a compass in no slot reads Not Set");
    }
}
