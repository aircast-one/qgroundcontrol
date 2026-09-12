use serde_json::{Value, json};

use crate::router::Backend;

pub fn object(json: &str) -> Value {
    serde_json::from_str(json).unwrap_or(Value::Null)
}

pub fn flag(object: &Value, key: &str) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub fn integer(object: &Value, key: &str) -> Option<i64> {
    object.get(key).and_then(Value::as_i64)
}

pub fn truthy(object: &Value, key: &str) -> bool {
    match object.get(key) {
        Some(Value::Bool(set)) => *set,
        Some(Value::Number(n)) => n.as_f64().map(|v| v != 0.0).unwrap_or(false),
        _ => false,
    }
}

pub fn text(object: &Value, key: &str) -> String {
    object.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn value_number(json: &str) -> Option<f64> {
    object(json).get("value")?.as_f64().filter(|v| v.is_finite())
}

pub fn value_string(json: &str) -> String {
    object(json).get("value").and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn result_flag(json: &str) -> bool {
    let v = object(json);
    flag(&v, "ok") && flag(&v, "result")
}

pub fn result_integer(json: &str) -> Option<i64> {
    ok_result(json)?.as_i64()
}

pub fn result_number(json: &str) -> Option<f64> {
    ok_result(json)?.as_f64().filter(|v| v.is_finite())
}

pub fn ok_result(json: &str) -> Option<Value> {
    let reply = object(json);
    (reply.get("ok") == Some(&Value::Bool(true))).then(|| reply.get("result").cloned()).flatten()
}

pub fn fact_flag(object: &Value, name: &str) -> bool {
    object
        .get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| facts.iter().find(|f| f.get("name").and_then(Value::as_str) == Some(name)))
        .and_then(|f| f.get("value"))
        .map(|v| v.as_bool().unwrap_or(v.as_i64().unwrap_or(0) != 0))
        .unwrap_or(false)
}

pub struct Unit {
    pub name: String,
    pub factor: f64,
}

impl Unit {
    pub fn vertical(backend: &dyn Backend) -> Unit {
        Unit::read(backend, "metersToAppSettingsVerticalDistanceUnits", "appSettingsVerticalDistanceUnitsString", "m")
    }

    pub fn horizontal(backend: &dyn Backend) -> Unit {
        Unit::read(backend, "metersToAppSettingsHorizontalDistanceUnits", "appSettingsHorizontalDistanceUnitsString", "m")
    }

    pub fn area(backend: &dyn Backend) -> Unit {
        Unit::read(backend, "squareMetersToAppSettingsAreaUnits", "appSettingsAreaUnitsString", "m\u{b2}")
    }

    pub fn speed(backend: &dyn Backend) -> Unit {
        Unit::read(backend, "metersSecondToAppSettingsSpeedUnits", "appSettingsSpeedUnitsString", "m/s")
    }

    fn read(backend: &dyn Backend, conversion: &str, name_property: &str, fallback: &str) -> Unit {
        let factor = result_number(&backend.invoke(&format!("units.{conversion}"), "[1.0]")).filter(|f| *f > 0.0).unwrap_or(1.0);
        let name = object(&backend.get_fields("units", name_property))
            .get(name_property)
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
            .unwrap_or(fallback)
            .to_string();
        Unit { name, factor }
    }

    pub fn show(&self, meters: f64) -> f64 {
        meters * self.factor
    }

    pub fn meters(&self, shown: f64) -> f64 {
        shown / self.factor
    }

    pub fn label(&self, meters: f64) -> String {
        format!("{:.1} {}", self.show(meters), self.name)
    }
}

pub const WHOLE_NUMBER_FROM: f64 = 100.0;

pub fn altitude_text(metres: f64, vertical: &Unit, signed: bool) -> String {
    let sign = match (metres < 0.0, signed) {
        (true, _) => "-",
        (false, true) => "+",
        (false, false) => "",
    };
    format!("{sign}{}", format_measure(vertical.show(metres.abs()), &vertical.name))
}

pub fn range_text(low: f64, high: f64, unit: &Unit) -> String {
    let (shown_low, shown_high) = (unit.show(low), unit.show(high));
    let whole = shown_low.abs().max(shown_high.abs()) >= WHOLE_NUMBER_FROM;
    let spell = |value: f64| match whole {
        true => format!("{value:.0}"),
        false => format!("{value:.1}"),
    };
    format!("{} {} to {} {}", spell(shown_low), unit.name, spell(shown_high), unit.name)
}

pub fn refused(reason: &str) -> Value {
    json!({ "kind": "null", "reason": reason })
}

pub fn settled(number: String) -> String {
    match number.strip_prefix('-').filter(|rest| rest.chars().all(|c| c == '0' || c == '.')) {
        Some(rest) => rest.to_string(),
        None => number,
    }
}

pub fn format_measure(value: f64, units: &str) -> String {
    let number = settled(if value.abs() >= WHOLE_NUMBER_FROM { format!("{value:.0}") } else { format!("{value:.1}") });
    format!("{number} {}", units.replace("^2", "\u{b2}"))
}

#[cfg(test)]
mod measure_tests {
    use super::format_measure;

    #[test]
    fn a_vehicle_on_the_ground_does_not_read_as_below_its_launch_point() {
        assert_eq!(super::format_measure(-0.04, "m"), "0.0 m", "a stationary vehicle reports a relative altitude a hair under zero and Rust rounds -0.04 to -0.0, which on an altimeter reads as the aircraft being below where it took off");
        assert_eq!(super::format_measure(-0.004, "m"), "0.0 m");
        assert_eq!(super::format_measure(-0.4, "m"), "-0.4 m", "a real descent keeps its sign");
        assert_eq!(super::format_measure(-0.05, "m"), "-0.1 m", "and so does one that rounds to a tenth");
        assert_eq!(super::format_measure(-120.0, "m"), "-120 m", "the whole-number threshold compared the signed value, so a depth of 120 metres carried a tenth that the same height above sea level does not");
        assert_eq!(super::format_measure(0.04, "m"), "0.0 m");
    }

    #[test]
    fn measures_keep_a_tenth_under_a_hundred_and_write_squared_units() {
        assert_eq!(format_measure(45.26, "m^2"), "45.3 m\u{b2}");
        assert_eq!(format_measure(89999.4, "m^2"), "89999 m\u{b2}");
        assert_eq!(format_measure(100.0, "ft"), "100 ft");
        assert_eq!(format_measure(99.96, "m"), "100.0 m");
        assert_eq!(format_measure(40.0, "m"), "40.0 m");
    }
}
