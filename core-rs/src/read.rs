use serde_json::Value;

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

/// A yes-or-no the C++ may declare either way. RadioComponentController declares
/// `Q_PROPERTY(int rollChannelReversed)` over a getter returning bool, so the bridge sends a
/// number; the sibling properties beside it declare bool and send one. Reading with `flag` or
/// `integer` couples the answer to which was written, and correcting that obvious typo upstream
/// would silently turn every reversed channel into a normal one.
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

pub fn format_measure(value: f64, units: &str) -> String {
    let number = if value >= WHOLE_NUMBER_FROM { format!("{value:.0}") } else { format!("{value:.1}") };
    format!("{number} {}", units.replace("^2", "\u{b2}"))
}

#[cfg(test)]
mod measure_tests {
    use super::format_measure;

    #[test]
    fn measures_keep_a_tenth_under_a_hundred_and_write_squared_units() {
        assert_eq!(format_measure(45.26, "m^2"), "45.3 m\u{b2}");
        assert_eq!(format_measure(89999.4, "m^2"), "89999 m\u{b2}");
        assert_eq!(format_measure(100.0, "ft"), "100 ft");
        assert_eq!(format_measure(99.96, "m"), "100.0 m");
        assert_eq!(format_measure(40.0, "m"), "40.0 m");
    }
}
