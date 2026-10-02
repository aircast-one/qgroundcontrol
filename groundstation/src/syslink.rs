use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady"];
pub const SET_CHANNEL: &str = "syslink.setChannel";
pub const SET_ADDRESS: &str = "syslink.setAddress";
pub const SET_RATE: &str = "syslink.setRate";
pub const SYSLINK_RESET_DEFAULTS: &str = "syslink.resetDefaults";
pub const RATES: [&str; 3] = ["750Kb/s", "1Mb/s", "2Mb/s"];
const RADIO_CHANNELS: std::ops::RangeInclusive<i64> = 0..=125;
const PARAMETERS: [&str; 4] = ["SLNK_RADIO_CHAN", "SLNK_RADIO_RATE", "SLNK_RADIO_ADDR1", "SLNK_RADIO_ADDR2"];

fn path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter(-1,{name})")
}

fn fact(backend: &dyn Backend, name: &str) -> Option<Value> {
    let fact = object(&backend.get(&path(name)));
    (fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty())).then_some(fact)
}

fn raw(backend: &dyn Backend, name: &str) -> Option<u64> {
    fact(backend, name).and_then(|f| f.get("rawValue").or(f.get("value")).and_then(Value::as_f64)).map(|v| v as i64 as u32 as u64)
}

pub fn address_text(upper: u64, lower: u64) -> String {
    format!("{:x}", (upper << 32) | (lower & 0xFFFF_FFFF))
}

pub fn address_words(text: &str) -> Option<(u32, u32)> {
    let value = if text.trim().is_empty() { 0 } else { u64::from_str_radix(text.trim(), 16).ok()? };
    Some(((value >> 32) as u32, (value & 0xFFFF_FFFF) as u32))
}

pub fn syslink_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let Some(channel) = raw(backend, "SLNK_RADIO_CHAN") else { return json!({ "kind": "object", "class": "Syslink", "available": false }) };
    json!({
        "kind": "object",
        "class": "Syslink",
        "available": true,
        "channel": channel,
        "channelHint": "Channel can be between 0 and 125",
        "address": address_text(raw(backend, "SLNK_RADIO_ADDR1").unwrap_or(0), raw(backend, "SLNK_RADIO_ADDR2").unwrap_or(0)),
        "addressHint": "Address in hex. Default is E7E7E7E7E7.",
        "rate": raw(backend, "SLNK_RADIO_RATE"),
        "rates": RATES,
    })
}

fn write(backend: &dyn Backend, name: &str, value: u64) -> bool {
    crate::read::flag(&object(&backend.set(&format!("{}.rawValue", path(name)), &json!({ "value": value }).to_string())), "ok")
}

fn answer(written: bool) -> Value {
    json!({ "ok": written, "reason": if written { Value::Null } else { json!("The vehicle did not take the new setting.") } })
}

pub fn run(backend: &dyn Backend, action: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    match action {
        SET_CHANNEL => match given.first().and_then(Value::as_i64).filter(|c| RADIO_CHANNELS.contains(c)) {
            Some(channel) => answer(write(backend, "SLNK_RADIO_CHAN", channel as u64)),
            None => json!({ "ok": false, "reason": "Channel can be between 0 and 125" }),
        },
        SET_ADDRESS => match given.first().and_then(Value::as_str).and_then(address_words) {
            Some((upper, lower)) => answer(write(backend, "SLNK_RADIO_ADDR1", u64::from(upper)) && write(backend, "SLNK_RADIO_ADDR2", u64::from(lower))),
            None => json!({ "ok": false, "reason": "Address in hex. Default is E7E7E7E7E7." }),
        },
        SET_RATE => match given.first().and_then(Value::as_u64).filter(|r| (*r as usize) < RATES.len()) {
            Some(rate) if raw(backend, "SLNK_RADIO_RATE") == Some(rate) => json!({ "ok": true }),
            Some(rate) => answer(write(backend, "SLNK_RADIO_RATE", rate)),
            None => json!({ "ok": false, "reason": "Pick one of the listed data rates." }),
        },
        SYSLINK_RESET_DEFAULTS => answer(PARAMETERS.iter().all(|name| {
            fact(backend, name).and_then(|f| f.get("defaultValue").and_then(Value::as_f64)).is_some_and(|default| write(backend, name, default as i64 as u32 as u64))
        })),
        _ => json!({ "ok": false, "reason": format!("{action} is not a Syslink action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [SET_CHANNEL, SET_ADDRESS, SET_RATE, SYSLINK_RESET_DEFAULTS].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_is_two_parameters_read_as_one_hex_number_like_the_controller() {
        assert_eq!(address_text(0xE7, 0xE7E7_E7E7), "e7e7e7e7e7");
        assert_eq!(address_words("E7E7E7E7E7"), Some((0xE7, 0xE7E7_E7E7)));
        assert_eq!(address_words("zz"), None);
        assert_eq!(address_words(""), Some((0, 0)), "QString::toULongLong reads an empty field as zero");
    }
}
