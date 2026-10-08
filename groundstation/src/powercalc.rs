use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const CALCULATE: &str = "vehicleConfig.calculate";
pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady", "vehicle.batteries.count"];

struct Calculator {
    component: &'static str,
    title: &'static str,
    help: &'static str,
    measure: &'static str,
    measured_label: &'static str,
    reading_label: &'static str,
    param_label: &'static str,
    button: &'static str,
    no_reading: Option<&'static str>,
}

const CALCULATORS: [Calculator; 4] = [
    Calculator {
        component: "CalcVoltageDividerDialog",
        title: "Calculate Voltage Divider",
        help: "Measure battery voltage using an external voltmeter and enter the value below. Click Calculate to set the new voltage multiplier.",
        measure: "voltage",
        measured_label: "Measured voltage:",
        reading_label: "Vehicle voltage:",
        param_label: "Voltage divider:",
        button: "Calculate",
        no_reading: None,
    },
    Calculator {
        component: "CalcAmpsPerVoltDialog",
        title: "Calculate Amps per Volt",
        help: "Measure current draw using an external current meter and enter the value below. Click Calculate to set the new amps per volt value.",
        measure: "current",
        measured_label: "Measured current:",
        reading_label: "Vehicle current:",
        param_label: "Amps per volt:",
        button: "Calculate",
        no_reading: None,
    },
    Calculator {
        component: "APMCalcVoltageDividerDialog",
        title: "Calculate Voltage Multiplier",
        help: "Measure battery voltage using an external voltmeter and enter the value below. Click Calculate to set the new adjusted voltage multiplier.",
        measure: "voltage",
        measured_label: "Measured voltage:",
        reading_label: "Vehicle voltage:",
        param_label: "Voltage multiplier:",
        button: "Calculate And Set",
        no_reading: Some("Vehicle voltage telemetry is not available. Connect to a vehicle with a powered battery to enable automatic calculation."),
    },
    Calculator {
        component: "APMCalcAmpsPerVoltDialog",
        title: "Calculate Amps per Volt",
        help: "Measure current draw using an external current meter and enter the value below. Click Calculate to set the new amps per volt value.",
        measure: "current",
        measured_label: "Measured current:",
        reading_label: "Vehicle current:",
        param_label: "Amps per volt:",
        button: "Calculate And Set",
        no_reading: Some("Vehicle current telemetry is not available. Connect to a vehicle with a powered battery to enable automatic calculation."),
    },
];

pub fn calculator(component: &str, battery_index: usize, param: &str) -> Option<Value> {
    let found = CALCULATORS.iter().find(|c| c.component == component)?;
    Some(json!({
        "text": "Calculate",
        "title": found.title,
        "help": found.help,
        "measure": found.measure,
        "measuredLabel": found.measured_label,
        "readingLabel": found.reading_label,
        "paramLabel": found.param_label,
        "button": found.button,
        "noReading": found.no_reading,
        "batteryIndex": battery_index,
        "param": param,
    }))
}

fn position_of(backend: &dyn Backend, battery_id: usize) -> Option<usize> {
    let count = crate::read::value_number(&backend.value("vehicle.batteries.count")).unwrap_or(0.0).max(0.0) as usize;
    (0..count).find(|position| {
        let id = backend.value(&format!("vehicle.batteries.{position}.id"));
        id.get("rawValue").or(id.get("value")).and_then(Value::as_f64).is_some_and(|id| id as usize == battery_id)
    })
}

fn reading(backend: &dyn Backend, measure: &str, battery_index: usize) -> Option<(f64, String)> {
    let position = position_of(backend, battery_index.checked_sub(1)?)?;
    let fact = backend.value(&format!("vehicle.batteries.{position}.{measure}"));
    let value = fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64).filter(|v| v.is_finite())?;
    Some((value, fact.get("valueString").and_then(Value::as_str).unwrap_or_default().to_string()))
}

fn param_value(backend: &dyn Backend, param: &str) -> Option<(f64, String)> {
    let fact = backend.value(&format!("vehicle.parameterManager.getParameter(-1,{param})"));
    let value = fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64)?;
    Some((value, fact.get("valueString").and_then(Value::as_str).unwrap_or_default().to_string()))
}

pub fn calculated(measured: f64, current: f64, reading: f64, measure: &str) -> Option<f64> {
    let accepted = |new: f64| new.is_finite() && if measure == "current" { new != 0.0 } else { new > 0.0 };
    (measured != 0.0 && measured.is_finite() && reading != 0.0).then(|| measured * current / reading).filter(|new| accepted(*new))
}

pub fn power_calc_view(backend: &dyn Backend, args: &[String]) -> Value {
    let (measure, battery, param) = (args.first().map(String::as_str).unwrap_or(""), args.get(1).and_then(|a| a.trim().parse().ok()).unwrap_or(0usize), args.get(2).map(String::as_str).unwrap_or(""));
    let read = reading(backend, measure, battery).filter(|(value, _)| *value != 0.0);
    json!({
        "kind": "object",
        "class": "PowerCalc",
        "readingAvailable": read.is_some(),
        "readingText": read.map(|(_, text)| text),
        "paramText": param_value(backend, param).map(|(_, text)| text),
    })
}

pub fn calculate(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let text = |i: usize| given.get(i).and_then(Value::as_str).unwrap_or_default().to_string();
    let (param, measure) = (text(0), text(1));
    let battery = given.get(2).and_then(Value::as_u64).unwrap_or(0) as usize;
    let measured = given.get(3).and_then(|v| v.as_f64().or_else(|| v.as_str()?.trim().parse().ok())).unwrap_or(f64::NAN);
    let refused = |reason: &str| json!({ "ok": false, "reason": reason });
    let Some((current, _)) = param_value(backend, &param) else { return refused("That parameter is not on this vehicle.") };
    let Some((now, _)) = reading(backend, &measure, battery) else { return refused("The vehicle is not reporting that battery.") };
    let Some(new) = calculated(measured, current, now, &measure) else { return refused("Enter the measured value, with the vehicle reporting a non-zero reading.") };
    let written = object(&backend.set(&format!("vehicle.parameterManager.getParameter(-1,{param})"), &json!({ "value": new }).to_string()));
    match written.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        true => json!({ "ok": true, "result": new }),
        false => refused("The vehicle did not take the new value."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Packs;
    impl Backend for Packs {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.batteries.count" => json!({ "kind": "value", "value": 1 }),
                "vehicle.batteries.0.id" => json!({ "kind": "fact", "rawValue": 1 }),
                "vehicle.batteries.0.voltage" => json!({ "kind": "fact", "rawValue": 15.2, "valueString": "15.20" }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, _: &str, _: &str) -> String { String::new() }
        fn set(&self, _: &str, _: &str) -> String { String::new() }
        fn invoke(&self, _: &str, _: &str) -> String { String::new() }
        fn watch(&self, _: &[String]) {}
    }

    #[test]
    fn a_calculator_reads_the_battery_by_its_status_id_not_its_list_position() {
        assert_eq!(reading(&Packs, "voltage", 2), Some((15.2, "15.20".to_string())), "BATT2 is BATTERY_STATUS id 1, the only pack reporting when BATT_MONITOR is off");
        assert_eq!(reading(&Packs, "voltage", 1), None);
    }

    #[test]
    fn the_new_value_scales_the_old_by_measured_over_reported_like_the_qgc_dialogs() {
        assert_eq!(calculated(12.6, 10.0, 12.0, "voltage"), Some(10.5));
        assert_eq!(calculated(0.0, 10.0, 12.0, "voltage"), None, "an empty measurement changes nothing");
        assert_eq!(calculated(f64::NAN, 10.0, 12.0, "voltage"), None);
        assert_eq!(calculated(12.6, 10.0, 0.0, "voltage"), None, "no reading, no division");
        assert_eq!(calculated(12.6, -1.0, 12.0, "voltage"), None, "a non-positive voltage multiplier is not written");
        assert_eq!(calculated(12.6, -1.0, 12.0, "current"), Some(-1.05), "CalcAmpsPerVoltDialog writes any non-zero amps per volt");
        let shown = calculator("APMCalcAmpsPerVoltDialog", 2, "BATT2_AMP_PERVLT").unwrap();
        assert_eq!((shown["button"].clone(), shown["measure"].clone(), shown["batteryIndex"].clone()), (json!("Calculate And Set"), json!("current"), json!(2)));
        assert!(calculator("ESCCalibrationDialog", 1, "").is_none());
    }
}
