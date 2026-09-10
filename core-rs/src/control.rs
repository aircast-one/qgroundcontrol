use serde_json::{Value, json};

use crate::label::humanise;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const UNKNOWN_ENUM_PREFIX: &str = "Unknown: ";

pub fn control_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return json!({ "kind": "null" }) };
    let fact = object(&backend.get(path));
    match fact.get("kind").and_then(Value::as_str) {
        Some("fact") => decode(&fact, path),
        _ => json!({ "kind": "null" }),
    }
}

pub fn raw_text(value: &Value) -> String {
    match value.as_f64() {
        Some(n) if n == n.round() && n.abs() < 1e15 => format!("{}", n as i64),
        Some(n) => format!("{n}"),
        None => value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string()),
    }
}

pub fn decode(fact: &Value, path: &str) -> Value {
    let text = |key: &str| fact.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let flag = |key: &str| fact.get(key).and_then(Value::as_bool).unwrap_or(false);
    let name = text("name");
    let labels: Vec<String> = fact.get("enumStrings").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    let raws: Vec<Value> = fact
        .get("enumValues")
        .and_then(Value::as_array)
        .filter(|r| r.len() == labels.len())
        .cloned()
        .unwrap_or_else(|| (0..labels.len()).map(|i| json!(i)).collect());
    let options: Vec<Value> = labels
        .iter()
        .zip(raws.iter())
        .filter(|(label, _)| !label.starts_with(UNKNOWN_ENUM_PREFIX))
        .map(|(label, raw)| json!({ "label": label, "raw": raw_text(raw) }))
        .collect();
    let bit_labels: Vec<String> = fact.get("bitmaskStrings").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    let bit_values: Vec<Value> = fact.get("bitmaskValues").and_then(Value::as_array).filter(|v| v.len() == bit_labels.len()).cloned().unwrap_or_default();
    let value_bits = fact.get("value").and_then(Value::as_i64).unwrap_or(0);
    let bits: Vec<Value> = bit_labels
        .iter()
        .zip(bit_values.iter())
        .filter_map(|(label, raw)| raw.as_i64().map(|bit| json!({ "label": label, "raw": raw_text(raw), "set": bit != 0 && value_bits & bit != 0 })))
        .collect();
    let control = match (flag("typeIsBool"), labels.is_empty(), bits.is_empty(), flag("typeIsString")) {
        (true, ..) => "toggle",
        (false, false, ..) => "choice",
        (false, true, false, _) => "bitmask",
        (false, true, true, true) => "text",
        _ => "number",
    };
    let enum_index = fact.get("enumIndex").and_then(Value::as_i64).unwrap_or(-1);
    let display = labels
        .get(usize::try_from(enum_index).unwrap_or(usize::MAX))
        .filter(|l| !l.starts_with(UNKNOWN_ENUM_PREFIX))
        .cloned()
        .unwrap_or_else(|| text("valueString"));
    let bound = |key: &str, default_flag: &str| (!flag(default_flag)).then(|| fact.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())).flatten();
    let described = text("shortDescription");
    json!({
        "kind": "object",
        "class": "Control",
        "path": path,
        "name": name,
        "label": if described.is_empty() { humanise(&name) } else { described },
        "control": control,
        "value": fact.get("value").cloned().unwrap_or(Value::Null),
        "valueString": text("valueString"),
        "display": display,
        "units": text("units"),
        "readOnly": flag("readOnly"),
        "options": options,
        "bits": bits,
        "decimalPlaces": fact.get("decimalPlaces").and_then(Value::as_i64).unwrap_or(0),
        "minimum": bound("min", "minIsDefaultForType"),
        "maximum": bound("max", "maxIsDefaultForType"),
        "rebootRequired": flag("vehicleRebootRequired") || flag("qgcRebootRequired"),
        "vehicleRebootRequired": flag("vehicleRebootRequired"),
        "applicationRestartRequired": flag("qgcRebootRequired"),
        "restartNotices": restart_notices(flag("vehicleRebootRequired"), flag("qgcRebootRequired")),
    })
}

pub const VEHICLE_REBOOT_NOTICE: &str = "Vehicle reboot required after change";
pub const APPLICATION_RESTART_NOTICE: &str = "Application restart required after change";

pub fn restart_notices(vehicle: bool, application: bool) -> Vec<&'static str> {
    vehicle
        .then_some(VEHICLE_REBOOT_NOTICE)
        .into_iter()
        .chain(application.then_some(APPLICATION_RESTART_NOTICE))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_restart_says_which_thing_has_to_restart() {
        assert_eq!(restart_notices(true, false), vec![VEHICLE_REBOOT_NOTICE], "rebooting an airframe mid-setup is not the same ask as restarting the ground station");
        assert_eq!(restart_notices(false, true), vec![APPLICATION_RESTART_NOTICE]);
        assert_eq!(restart_notices(true, true), vec![VEHICLE_REBOOT_NOTICE, APPLICATION_RESTART_NOTICE], "the Qt dialog stacks two labels rather than merging them, so nothing here is invented copy");
        assert!(restart_notices(false, false).is_empty());
        let unit = decode(&json!({ "kind": "fact", "name": "distanceUnits", "value": 0, "valueString": "0", "qgcRebootRequired": true }), "p");
        assert_eq!((unit["rebootRequired"].as_bool(), unit["vehicleRebootRequired"].as_bool(), unit["applicationRestartRequired"].as_bool()), (Some(true), Some(false), Some(true)));
        assert_eq!(unit["restartNotices"], json!([APPLICATION_RESTART_NOTICE]));
        let param = decode(&json!({ "kind": "fact", "name": "COMPASS_USE", "value": 1, "valueString": "1", "vehicleRebootRequired": true }), "p");
        assert_eq!((param["vehicleRebootRequired"].as_bool(), param["applicationRestartRequired"].as_bool()), (Some(true), Some(false)));
        assert_eq!(param["restartNotices"], json!([VEHICLE_REBOOT_NOTICE]));
    }

    #[test]
    fn a_bitmask_fact_names_its_bits_and_says_which_are_set() {
        let arming = decode(&json!({ "kind": "fact", "name": "ARMING_CHECK", "value": 82, "valueString": "82", "bitmaskStrings": ["All", "Barometer", "Compass", "GPS lock", "INS", "Parameters", "RC Channels"], "bitmaskValues": [1, 2, 4, 16, 32, 64, 128] }), "p");
        assert_eq!(arming["control"], "bitmask", "a parameter an operator sets bit by bit is not a number");
        let set: Vec<&str> = arming["bits"].as_array().unwrap().iter().filter(|b| b["set"] == true).map(|b| b["label"].as_str().unwrap()).collect();
        assert_eq!(set, ["Barometer", "GPS lock", "Parameters"], "82 is bits 2, 16 and 64");
        assert_eq!(arming["bits"].as_array().unwrap().len(), 7);
        assert_eq!(arming["bits"][0]["raw"], "1");
        let both = decode(&json!({ "kind": "fact", "name": "FS_OPTIONS", "value": 1, "valueString": "1", "enumStrings": ["None", "Continue"], "enumValues": [0, 1], "enumIndex": 1, "bitmaskStrings": ["RC", "Battery"], "bitmaskValues": [1, 2] }), "p");
        assert_eq!(both["control"], "choice", "a fact whose metadata carries both reads as an enum, as the Qt editor resolves it");
        let plain = decode(&json!({ "kind": "fact", "name": "WPNAV_SPEED", "value": 500, "valueString": "500" }), "p");
        assert_eq!(plain["control"], "number");
        assert!(plain["bits"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_bool_fact_is_a_toggle_and_an_enum_a_choice_without_unknowns() {
        let toggle = decode(&json!({ "kind": "fact", "name": "audioMuted", "typeIsBool": true, "value": true, "valueString": "true" }), "p");
        assert_eq!(toggle["control"], "toggle");
        assert_eq!(toggle["label"], "Audio Muted");
        let choice = decode(&json!({ "kind": "fact", "name": "verticalDistanceUnits", "shortDescription": "Vertical distance", "enumStrings": ["Feet", "Meters", "Unknown: 7"], "enumValues": [0, 1, 7], "enumIndex": 2, "valueString": "7" }), "p");
        assert_eq!(choice["control"], "choice");
        assert_eq!(choice["options"].as_array().unwrap().len(), 2);
        assert_eq!(choice["options"][1]["raw"], "1");
        assert_eq!(choice["display"], "7");
        assert_eq!(choice["label"], "Vertical distance");
    }

    #[test]
    fn number_bounds_follow_the_bridges_default_for_type_flags_not_a_magic_size() {
        let bounded = decode(&json!({ "kind": "fact", "name": "RTL_ALT", "min": 0.0, "max": 5e9, "minIsDefaultForType": false, "maxIsDefaultForType": false, "decimalPlaces": 1 }), "p");
        assert_eq!(bounded["control"], "number");
        assert_eq!(bounded["minimum"], 0.0);
        assert_eq!(bounded["maximum"], 5e9);
        let open = decode(&json!({ "kind": "fact", "name": "X", "min": -32768, "max": 32767, "minIsDefaultForType": true, "maxIsDefaultForType": true }), "p");
        assert_eq!(open["minimum"], Value::Null);
        assert_eq!(open["maximum"], Value::Null);
    }

    #[test]
    fn strings_and_enum_positions_fall_back_sensibly() {
        let text = decode(&json!({ "kind": "fact", "name": "rtspUrl", "typeIsString": true, "valueString": "rtsp://x" }), "p");
        assert_eq!(text["control"], "text");
        let positional = decode(&json!({ "kind": "fact", "name": "mode", "enumStrings": ["A", "B"], "enumIndex": 1 }), "p");
        assert_eq!(positional["options"][1]["raw"], "1");
        assert_eq!(positional["display"], "B");
        assert_eq!(raw_text(&json!(2.5)), "2.5");
        assert_eq!(raw_text(&json!("x")), "x");
    }
}
