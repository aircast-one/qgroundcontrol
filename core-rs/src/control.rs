use serde_json::{Value, json};

use crate::label::humanise;
use crate::read::{object, refused};
use crate::router::Backend;

pub const DEPS: &[&str] = &[];

pub fn control_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return refused("view.control needs the path of a fact, as view.control(settings.appSettings.audioMuted)") };
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
    let synthetic = text("unknownEnumLabel");
    let is_synthetic = |label: &str| !synthetic.is_empty() && label == synthetic;
    let options: Vec<Value> = labels
        .iter()
        .zip(raws.iter())
        .filter(|(label, _)| !is_synthetic(label))
        .map(|(label, raw)| json!({ "label": label, "raw": raw_text(raw) }))
        .collect();
    let bit_labels: Vec<String> = fact.get("bitmaskStrings").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    let bit_values: Vec<Value> = fact.get("bitmaskValues").and_then(Value::as_array).filter(|v| v.len() == bit_labels.len()).cloned().unwrap_or_default();
    let value_bits = fact.get("value").and_then(Value::as_i64).unwrap_or(0);
    let bits: Vec<Value> = bit_labels
        .iter()
        .zip(bit_values.iter())
        .filter_map(|(label, raw)| raw.as_i64().filter(|bit| *bit != 0).map(|bit| json!({ "label": label, "raw": raw_text(raw), "set": value_bits & bit != 0 })))
        .collect();
    // A whole-number fact and a real one are the same "number" control to a head, and the difference
    // is not cosmetic: convertAndValidateRaw truncates 3.7 to 3 on an integer fact and answers that
    // it succeeded, so the operator asks for 3.7, the vehicle gets 3, and nothing anywhere says so.
    // decimalPlaces is not this answer - a real-typed percentage legitimately declares zero.
    let whole = flag("typeIsInteger");
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
        .filter(|l| !is_synthetic(l))
        .cloned()
        .unwrap_or_else(|| text("valueString"));
    let bound = |key: &str, default_flag: &str| (!flag(default_flag)).then(|| fact.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())).flatten();
    // minString and maxString are populated whatever minIsDefaultForType says, so a fact that
    // declares no floor still carries the string for the smallest number its type can hold. Serving
    // that beside a null minimum is how a field ends up printing "Min -3.4e38". One gate decides the
    // number and its spelling together, so the two cannot disagree about whether a bound exists.
    let bound_text = |key: &str, default_flag: &str| (!flag(default_flag)).then(|| fact.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string)).flatten();
    // Fact declares a typed defaultValue beside the string one, but it was missing from the
    // bridge's kFactProperties allowlist and so arrived nowhere - added there rather than parsed
    // back out of defaultValueString here, because that string is already formatted to
    // decimalPlaces and reading a number out of it would be the round trip through a rendered
    // string that Android's plainNumber does, one layer further down where nobody can see it.
    let has_default = flag("defaultValueAvailable");
    let described = text("shortDescription");
    json!({
        "kind": "object",
        "class": "Control",
        "path": path,
        "name": name,
        "label": if described.is_empty() { humanise(&name) } else { described },
        "control": control,
        "value": fact.get("value").cloned().unwrap_or(Value::Null),
        "valueMeters": crate::read::metres(fact),
        "valueString": text("valueString"),
        "display": display,
        "units": text("units"),
        "readOnly": fact.get("readOnly").and_then(Value::as_bool).unwrap_or(true),
        "options": options,
        "bits": bits,
        "decimalPlaces": fact.get("decimalPlaces").and_then(Value::as_i64).unwrap_or(0),
        "wholeNumbersOnly": whole,
        "minimum": bound("min", "minIsDefaultForType"),
        "maximum": bound("max", "maxIsDefaultForType"),
        "minimumText": bound_text("minString", "minIsDefaultForType"),
        "maximumText": bound_text("maxString", "maxIsDefaultForType"),
        "changedFromDefault": has_default.then(|| !flag("valueEqualsDefault")),
        "longDescription": Some(text("longDescription")).filter(|long| !long.is_empty()),
        "defaultText": has_default.then(|| text("defaultValueString")).filter(|shown| !shown.is_empty()),
        "defaultValue": has_default.then(|| fact.get("defaultValue").cloned()).flatten().unwrap_or(Value::Null),
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
    fn the_toggle_branch_is_only_safe_while_no_bool_fact_names_its_alternatives() {
        let bool_with_labels = json!({ "kind": "fact", "name": "useFixedBasePosition", "typeIsBool": true, "value": false, "enumStrings": ["Survey-In", "Specify position"], "enumValues": [0, 1], "enumIndex": 0 });
        assert_eq!(
            decode(&bool_with_labels, "p")["control"],
            "toggle",
            "typeIsBool is matched before the enum branch, so a bool fact carrying named alternatives is served as a toggle and its options are discarded. This is the behaviour, not the wish - it is pinned because it is invisible: a head draws a switch and nothing anywhere says two labels were dropped"
        );
        assert!(
            decode(&bool_with_labels, "p")["options"].as_array().is_some_and(|o| o.len() == 2),
            "the options are still served, so a head that wanted them could reach them - which is the only reason the ordering is tolerable"
        );

        let groups = include_str!("../../src/Settings/RTK.SettingsGroup.json");
        assert!(groups.contains("useFixedBasePosition"), "the settings group file has moved, so the premise below is being checked against nothing");
        let bool_with_enum_strings = serde_json::from_str::<serde_json::Value>(groups).unwrap()["QGC.MetaData.Facts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|fact| fact["type"] == "bool" && fact.get("enumStrings").is_some());
        assert!(
            !bool_with_enum_strings,
            "RTK is the group where a bool most obviously wants two labels - QGC draws useFixedBasePosition as a Survey-In / Specify position radio pair - and it declares none. Across all 22 settings groups, 198 facts, 59 of them bool, not one bool declares enumStrings, which is why the ordering above has never mattered. If that changes this fails here rather than rendering a silent switch"
        );
    }

    #[test]
    fn a_parameter_with_no_default_is_not_a_parameter_that_differs_from_one() {
        let at = |fact: Value| decode(&fact, "p");
        let stock = at(json!({ "kind": "fact", "name": "RTL_ALT", "defaultValueAvailable": true, "valueEqualsDefault": true }));
        assert_eq!(stock["changedFromDefault"], false);

        let changed = at(json!({ "kind": "fact", "name": "RTL_ALT", "defaultValueAvailable": true, "valueEqualsDefault": false }));
        assert_eq!(changed["changedFromDefault"], true, "ParameterEditor.qml:253 draws an orange dot on defaultValueAvailable && !valueEqualsDefault, and neither native head drew anything - an operator could not tell which parameters an unfamiliar airframe had been changed from stock");

        let stockless = at(json!({ "kind": "fact", "name": "RTL_ALT", "defaultValueAvailable": false, "valueEqualsDefault": false }));
        assert_eq!(
            stockless["changedFromDefault"],
            Value::Null,
            "Fact::valueEqualsDefault returns FALSE when there is no default at all, so the raw flag means both differs-from-stock and has-no-stock. Serving it unguarded marks every parameter without a default as modified; QGC only escapes that because the QML happens to write defaultValueAvailable && on the same line"
        );

        assert_eq!(at(json!({ "kind": "fact", "name": "x", "longDescription": "How far to climb before returning" }))["longDescription"], "How far to climb before returning", "ParameterEditorController matches a search term against name, shortDescription AND longDescription; a head with only the first two silently returns a shorter list");
        assert_eq!(at(json!({ "kind": "fact", "name": "x" }))["longDescription"], Value::Null, "absent rather than empty, so a fact with no long description says so instead of carrying a blank that reads like one someone deleted");
    }

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

        let ordinary = decode(&json!({ "kind": "fact", "name": "RTL_ALT", "value": 30, "valueString": "30" }), "p");
        assert_eq!(ordinary["rebootRequired"], false, "a parameter that takes effect immediately must not ask for a restart");
        assert_eq!(ordinary["vehicleRebootRequired"], false);
        assert_eq!(ordinary["applicationRestartRequired"], false);
        assert_eq!(ordinary["restartNotices"], json!([]));
    }

    #[test]
    fn a_fact_that_will_not_say_whether_it_is_writable_is_not_offered_as_writable() {
        let silent = decode(&json!({ "kind": "fact", "name": "MYSTERY", "value": 1, "valueString": "1" }), "p");
        assert_eq!(silent["readOnly"], true, "an inverted flag that fails open hands an operator an editable control the core could not vouch for");
        let stated = decode(&json!({ "kind": "fact", "name": "WPNAV_SPEED", "value": 500, "valueString": "500", "readOnly": false }), "p");
        assert_eq!(stated["readOnly"], false, "a fact that says it is writable is taken at its word");
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
        let malformed = decode(&json!({ "kind": "fact", "name": "ODD", "value": 3, "valueString": "3", "bitmaskStrings": ["Nothing", "Real"], "bitmaskValues": [0, 2] }), "p");
        let labels: Vec<&str> = malformed["bits"].as_array().unwrap().iter().map(|b| b["label"].as_str().unwrap()).collect();
        assert_eq!(labels, ["Real"], "a bit worth nothing is dropped here rather than drawn as a checkbox that can never change anything");
        let high = decode(&json!({ "kind": "fact", "name": "SIGNED", "value": -128, "valueString": "-128", "bitmaskStrings": ["Top"], "bitmaskValues": [-128] }), "p");
        assert_eq!(high["bits"][0]["set"], true, "an int8 parameter carries its top bit as -128, which is still that bit");
        let plain = decode(&json!({ "kind": "fact", "name": "WPNAV_SPEED", "value": 500, "valueString": "500" }), "p");
        assert_eq!(plain["control"], "number");
        assert!(plain["bits"].as_array().unwrap().is_empty());
    }

    #[test]
    fn the_synthetic_entry_is_recognised_in_a_language_that_is_not_english() {
        let german = decode(
            &json!({ "kind": "fact", "name": "verticalDistanceUnits", "enumStrings": ["Fuss", "Meter", "Unbekannt: 7"], "enumValues": [0, 1, 7], "enumIndex": 2, "valueString": "7", "unknownEnumLabel": "Unbekannt: 7" }),
            "p",
        );
        assert_eq!(german["options"].as_array().unwrap().len(), 2, "Fact synthesises this entry through tr(), so matching the English prefix offered it as a real choice in every other language");
        assert_eq!(german["display"], "7", "and showed its translated placeholder where the raw value belongs");
    }

    #[test]
    fn a_bool_fact_is_a_toggle_and_an_enum_a_choice_without_unknowns() {
        let toggle = decode(&json!({ "kind": "fact", "name": "audioMuted", "typeIsBool": true, "value": true, "valueString": "true" }), "p");
        assert_eq!(toggle["control"], "toggle");
        assert_eq!(toggle["label"], "Audio Muted");
        let choice = decode(&json!({ "kind": "fact", "name": "verticalDistanceUnits", "shortDescription": "Vertical distance", "enumStrings": ["Feet", "Meters", "Unknown: 7"], "enumValues": [0, 1, 7], "enumIndex": 2, "valueString": "7", "unknownEnumLabel": "Unknown: 7" }), "p");
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
    fn a_whole_number_fact_says_so_before_anything_truncates_it() {
        let counted = decode(&json!({ "kind": "fact", "name": "SR0_POSITION", "typeIsInteger": true, "value": 4, "decimalPlaces": 0 }), "p");
        assert_eq!(counted["wholeNumbersOnly"], true, "FactMetaData::convertAndValidateRaw does QVariant(3.7).toInt() for an int fact - 3 with convertOk true - and setRawValue passes convertOnly so the range check never runs, so a head sending 3.7 is told it succeeded and the vehicle gets 3");
        assert_eq!(counted["control"], "number");

        let real = decode(&json!({ "kind": "fact", "name": "WPNAV_SPEED", "value": 5.0, "decimalPlaces": 0 }), "p");
        assert_eq!(real["wholeNumbersOnly"], false, "decimalPlaces is not the same question - a real-typed fact declaring zero decimals is what you write for a percentage, and keying on it would refuse fractions the vehicle accepts");
        assert_eq!(real["decimalPlaces"], 0, "which is why both travel");

        let text = decode(&json!({ "kind": "fact", "name": "rtspUrl", "typeIsString": true, "valueString": "rtsp://x" }), "p");
        assert_eq!(text["wholeNumbersOnly"], false, "a string is not a whole number, and a head gating a numeric keyboard on this must not get true for one");
    }

    #[test]
    fn only_a_fact_measured_in_metres_carries_a_value_in_metres() {
        let altitude = decode(&json!({ "kind": "fact", "name": "RTL_ALT", "value": 328.0, "rawValue": 100.0, "rawUnits": "vertical m", "units": "ft" }), "p");
        assert_eq!(altitude["valueMeters"], 100.0, "value is cooked to the operator's display unit, so on an imperial profile it is 328 - a map annotation fed that number places the marker three times too high");
        assert_eq!(altitude["value"], 328.0, "and the cooked value still travels for the label beside it");

        ["m", "meter", "meters"].iter().for_each(|spelling| {
            let horizontal = decode(&json!({ "kind": "fact", "name": "D", "rawValue": 7.5, "rawUnits": spelling }), "p");
            assert_eq!(horizontal["valueMeters"], 7.5, "{spelling} is a length QGC translates, and all three spellings appear in its own table");
        });

        [("deg", 45.0), ("secs", 30.0), ("m/s", 12.0), ("m^2", 400.0), ("cm/px", 2.0)].iter().for_each(|(unit, raw)| {
            let other = decode(&json!({ "kind": "fact", "name": "X", "rawValue": raw, "rawUnits": unit }), "p");
            assert_eq!(other["valueMeters"], Value::Null, "{unit} is not a distance, and a field named for metres holding {raw} of something else is worse than no field - m^2 is an area and cm/px is a ground resolution, so neither is rescued by being metric");
        });

        let unmeasured = decode(&json!({ "kind": "fact", "name": "X", "rawUnits": "m" }), "p");
        assert_eq!(unmeasured["valueMeters"], Value::Null, "a length with no raw value reported is absent rather than zero");
    }

    #[test]
    fn a_bound_with_no_spelling_and_a_spelling_with_no_bound_cannot_happen() {
        let bounded = decode(&json!({ "kind": "fact", "name": "RTL_ALT", "min": 5.0, "max": 120.0, "minString": "5", "maxString": "120", "minIsDefaultForType": false, "maxIsDefaultForType": false, "defaultValueAvailable": true, "defaultValue": 10.0, "defaultValueString": "10" }), "p");
        assert_eq!(bounded["minimumText"], "5");
        assert_eq!(bounded["maximumText"], "120");
        assert_eq!(bounded["defaultValue"], 10.0);
        assert_eq!(bounded["defaultText"], "10");

        let open = decode(&json!({ "kind": "fact", "name": "X", "min": -3.4e38, "max": 3.4e38, "minString": "-3.4e+38", "maxString": "3.4e+38", "minIsDefaultForType": true, "maxIsDefaultForType": true }), "p");
        assert_eq!(open["minimum"], Value::Null);
        assert_eq!(
            open["minimumText"], Value::Null,
            "the bridge fills minString whatever minIsDefaultForType says, so a fact declaring no floor still carries the smallest number its type can hold - serving that beside a null minimum is how a field prints \"Altitude must be at least -3.4e38\""
        );
        assert_eq!(open["maximumText"], Value::Null);
        assert_eq!(open["defaultValue"], Value::Null, "no defaultValueAvailable means no default, and value beside it is the live reading rather than one");
        assert_eq!(open["defaultText"], Value::Null);
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
