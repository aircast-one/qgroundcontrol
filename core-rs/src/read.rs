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

pub fn text(object: &Value, key: &str) -> String {
    object.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn value_number(json: &str) -> Option<f64> {
    object(json).get("value")?.as_f64().filter(|v| v.is_finite())
}

pub fn value_string(json: &str) -> String {
    object(json).get("value").and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn result_integer(json: &str) -> Option<i64> {
    ok_result(json)?.as_i64()
}

pub fn result_number(json: &str) -> Option<f64> {
    ok_result(json)?.as_f64().filter(|v| v.is_finite())
}

fn ok_result(json: &str) -> Option<Value> {
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
