use serde_json::Value;

pub fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "NULL",
        Value::Bool(_) => "Bool",
        Value::Number(_) => "Double",
        Value::String(_) => "String",
        Value::Array(_) => "Array",
        Value::Object(_) => "Object",
    }
}

pub fn validate_keys(object: &Value, keys: &[(&str, &str, bool)]) -> Result<(), String> {
    let missing: Vec<&str> = keys.iter().filter(|(key, _, required)| *required && object.get(key).is_none()).map(|(key, _, _)| *key).collect();
    if !missing.is_empty() {
        return Err(format!("The following required keys are missing: {}", missing.join(", ")));
    }
    keys.iter()
        .filter_map(|(key, expected, _)| object.get(key).map(|value| (key, expected, type_name(value))))
        .find(|(_, expected, actual)| **expected != *actual && !(**expected == "NULL" && *actual == "Double"))
        .map_or(Ok(()), |(key, expected, actual)| Err(format!("Incorrect value type - key:type:expected {key}:{actual}:{expected}")))
}

pub fn to_int(value: &Value, default: i32) -> i32 {
    value.as_f64().filter(|v| v.fract() == 0.0 && *v >= f64::from(i32::MIN) && *v <= f64::from(i32::MAX)).map_or(default, |v| v as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_are_checked_like_json_parsing_validate_keys() {
        let keys = [("a", "String", true), ("b", "Double", true), ("c", "Double", false)];
        assert_eq!(validate_keys(&json!({}), &keys).unwrap_err(), "The following required keys are missing: a, b");
        assert_eq!(validate_keys(&json!({"a": "x", "b": 1, "c": null}), &keys).unwrap_err(), "Incorrect value type - key:type:expected c:NULL:Double");
        assert!(validate_keys(&json!({"a": "x", "b": 1}), &keys).is_ok());
        let nan = [("n", "NULL", true)];
        assert!(validate_keys(&json!({"n": null}), &nan).is_ok());
        assert!(validate_keys(&json!({"n": 3.5}), &nan).is_ok(), "JsonParsing::validateKeyTypes: Null signals a possible NaN on a double value");
        assert_eq!(validate_keys(&json!({"n": "x"}), &nan).unwrap_err(), "Incorrect value type - key:type:expected n:String:NULL");
    }

    #[test]
    fn to_int_takes_whole_numbers_only_like_qjsonvalue() {
        assert_eq!(to_int(&json!(7), 1), 7);
        assert_eq!(to_int(&json!(7.5), 1), 1);
        assert_eq!(to_int(&json!("7"), 1), 1);
        assert_eq!(to_int(&json!(3e10), 0), 0);
    }
}
