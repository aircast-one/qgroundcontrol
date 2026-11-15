use std::ffi::{CStr, CString};

use groundstation::nativehost::{qgc_bridge_free, qgc_bridge_get};
use serde_json::Value;

pub fn core_value(path: &str) -> Value {
    let Ok(path) = CString::new(path) else {
        return Value::Null;
    };
    let raw = unsafe { qgc_bridge_get(path.as_ptr()) };
    if raw.is_null() {
        return Value::Null;
    }
    let text = unsafe { CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe { qgc_bridge_free(raw) };
    serde_json::from_str(&text).unwrap_or(Value::Null)
}
