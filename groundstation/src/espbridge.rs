use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::control::decode;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady", "vehicle.messagesReceived", "vehicle.messagesLost", "vehicle.messagesSent"];
pub const SET_TEXT: &str = "espBridge.setText";
pub const SET_BAUD: &str = "espBridge.baud";
pub const REBOOT_BRIDGE: &str = "espBridge.reboot";
pub const RESTORE_DEFAULTS: &str = "espBridge.restoreDefaults";
pub const RESET_COUNTERS: &str = "espBridge.resetCounters";
pub const BRIDGE_OPENED: &str = "espBridge.open";
pub const COMPONENT: u8 = 240;
const DEFAULT_IP: &str = "192.168.4.1";
const STATUS_INTERVAL_MS: u64 = 1000;
const STATUS_TIMEOUT: Duration = Duration::from_secs(2);
const TEXT_BYTES: usize = 16;
pub const BAUD_RATES: [i64; 5] = [57600, 115200, 230400, 460800, 921600];
const WIFI_CHANNELS: std::ops::RangeInclusive<i64> = 1..=11;
const TEXT_FIELDS: [(&str, &str, bool); 4] = [("ssid", "WIFI_SSID", false), ("password", "WIFI_PASSWORD", false), ("ssidSta", "WIFI_SSIDSTA", true), ("passwordSta", "WIFI_PWDSTA", true)];

fn path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter({COMPONENT},{name})")
}

pub fn fact(backend: &dyn Backend, name: &str) -> Option<Value> {
    let fact = object(&backend.get(&path(name)));
    let present = fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
    present.then_some(fact)
}

fn raw(backend: &dyn Backend, name: &str) -> Option<u32> {
    fact(backend, name).and_then(|f| f.get("rawValue").or(f.get("value")).and_then(Value::as_f64)).map(|v| v as i64 as u32)
}

pub fn unpack(words: [u32; 4]) -> String {
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).take_while(|b| *b != 0).collect();
    String::from_utf8_lossy(&bytes).to_string()
}

pub fn pack(text: &str) -> [u32; 4] {
    let mut bytes = [0u8; TEXT_BYTES];
    text.bytes().take(TEXT_BYTES).enumerate().for_each(|(i, b)| bytes[i] = b);
    std::array::from_fn(|i| u32::from_le_bytes([bytes[i * 4], bytes[i * 4 + 1], bytes[i * 4 + 2], bytes[i * 4 + 3]]))
}

fn text(backend: &dyn Backend, prefix: &str) -> Option<String> {
    let words: Option<Vec<u32>> = (1..=4).map(|i| raw(backend, &format!("{prefix}{i}"))).collect();
    words.map(|w| unpack([w[0], w[1], w[2], w[3]]))
}

pub fn baud_index(baud: i64) -> usize {
    BAUD_RATES.iter().position(|b| *b == baud).unwrap_or(BAUD_RATES.len() - 1)
}

pub fn ip_address(stored: Option<u32>) -> String {
    stored.map_or_else(|| DEFAULT_IP.to_string(), |raw| std::net::Ipv4Addr::from(u32::from_be(raw)).to_string())
}

#[derive(Default)]
struct Status {
    fetched_ms: Option<u64>,
    counts: Option<Value>,
    reset: bool,
    errors: u32,
}

static STATUS: Mutex<Status> = Mutex::new(Status { fetched_ms: None, counts: None, reset: false, errors: 0 });

fn refresh_status(ip: String, now_ms: u64) {
    let due = {
        let mut status = STATUS.lock().unwrap_or_else(PoisonError::into_inner);
        let due = status.errors < 2 && status.fetched_ms.is_none_or(|at| now_ms.saturating_sub(at) >= STATUS_INTERVAL_MS);
        if due {
            status.fetched_ms = Some(now_ms);
        }
        due.then(|| std::mem::take(&mut status.reset))
    };
    let Some(reset) = due else { return };
    std::thread::spawn(move || {
        let url = format!("http://{ip}/status.json{}", if reset { "?r=1" } else { "" });
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(STATUS_TIMEOUT)).build().into();
        let answer = agent.get(&url).call().ok().and_then(|mut r| r.body_mut().read_to_string().ok()).and_then(|t| serde_json::from_str::<Value>(&t).ok());
        let mut status = STATUS.lock().unwrap_or_else(PoisonError::into_inner);
        match answer.filter(|a| a.get("errors").is_none()) {
            Some(counts) => {
                status.counts = Some(counts);
                status.errors = 0;
            }
            None => status.errors += 1,
        }
    });
}

pub fn esp_bridge_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let Some(channel) = fact(backend, "WIFI_CHANNEL") else { return json!({ "kind": "object", "class": "EspBridge", "available": false }) };
    let ip = ip_address(raw(backend, "WIFI_IPADDRESS"));
    refresh_status(ip.clone(), crate::hub::now_ms());
    let counts = STATUS.lock().unwrap_or_else(PoisonError::into_inner).counts.clone();
    let count = |key: &str| counts.as_ref().and_then(|c| c.get(key).cloned()).unwrap_or(Value::Null);
    let vehicle = object(&backend.get_fields("vehicle", "messagesReceived,messagesLost,messagesSent"));
    let decoded = |name: &str| fact(backend, name).map(|f| decode(&f, &path(name)));
    let has_sta = fact(backend, "WIFI_SSIDSTA1").is_some();
    let busy = crate::hub::lock().active().is_some_and(|v| v.esp_wait.is_some());
    json!({
        "kind": "object",
        "class": "EspBridge",
        "available": true,
        "mode": decoded("WIFI_MODE"),
        "modeIndex": raw(backend, "WIFI_MODE"),
        "channel": channel.get("rawValue").or(channel.get("value")).cloned(),
        "channels": WIFI_CHANNELS.collect::<Vec<_>>(),
        "channelPath": path("WIFI_CHANNEL"),
        "modePath": path("WIFI_MODE"),
        "ssid": text(backend, "WIFI_SSID"),
        "password": text(backend, "WIFI_PASSWORD"),
        "ssidSta": has_sta.then(|| text(backend, "WIFI_SSIDSTA")).flatten(),
        "passwordSta": has_sta.then(|| text(backend, "WIFI_PWDSTA")).flatten(),
        "baudRates": BAUD_RATES,
        "baudIndex": raw(backend, "UART_BAUDRATE").map(|b| baud_index(i64::from(b))),
        "hostPort": decoded("WIFI_UDP_HPORT"),
        "clientPort": decoded("WIFI_UDP_CPORT"),
        "ipAddress": ip,
        "status": {
            "vehicle": { "received": count("vpackets"), "lost": count("vlost"), "sent": count("vsent") },
            "bridge": { "received": count("gpackets"), "lost": count("glost"), "sent": count("gsent") },
            "qgc": { "received": vehicle.get("messagesReceived").cloned().unwrap_or(Value::Null), "lost": vehicle.get("messagesLost").cloned().unwrap_or(Value::Null), "sent": vehicle.get("messagesSent").cloned().unwrap_or(Value::Null) },
        },
        "busy": busy,
        "rebootPrompt": "This will restart the WiFi Bridge so the settings you've changed can take effect. Note that you may have to change your computer WiFi settings and QGroundControl link settings to match these changes. Are you sure you want to restart it?",
    })
}

fn write_raw(backend: &dyn Backend, name: &str, value: u32) -> bool {
    crate::read::flag(&object(&backend.set(&format!("{}.rawValue", path(name)), &json!({ "value": value }).to_string())), "ok")
}

fn command(backend: &dyn Backend, op: &str) -> Value {
    crate::guided::dispatch(backend, Some(json!({ "action": "espBridge", "op": op })), crate::guided::active_id(backend), "", "[]")
}

pub fn run(backend: &dyn Backend, action: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    match action {
        SET_TEXT => {
            let field = given.first().and_then(Value::as_str).unwrap_or("");
            let value = given.get(1).and_then(Value::as_str).unwrap_or("");
            let Some((_, prefix, optional)) = TEXT_FIELDS.iter().find(|(f, _, _)| *f == field) else { return json!({ "ok": false, "reason": format!("{field} is not a bridge text field") }) };
            if *optional && fact(backend, &format!("{prefix}1")).is_none() {
                return json!({ "ok": false, "reason": "This bridge firmware has no station mode." });
            }
            let written = pack(value).iter().enumerate().all(|(i, word)| write_raw(backend, &format!("{prefix}{}", i + 1), *word));
            json!({ "ok": written, "reason": if written { Value::Null } else { json!("The bridge did not take the new setting.") } })
        }
        SET_BAUD => match given.first().and_then(Value::as_u64).and_then(|i| BAUD_RATES.get(i as usize)) {
            Some(baud) => json!({ "ok": write_raw(backend, "UART_BAUDRATE", *baud as u32) }),
            None => json!({ "ok": false, "reason": "Pick one of the listed baud rates." }),
        },
        REBOOT_BRIDGE => command(backend, "reboot"),
        RESTORE_DEFAULTS => command(backend, "restoreDefaults"),
        BRIDGE_OPENED => command(backend, "open"),
        RESET_COUNTERS => {
            STATUS.lock().unwrap_or_else(PoisonError::into_inner).reset = true;
            backend.invoke("vehicle.resetCounters", "[]");
            json!({ "ok": true })
        }
        _ => json!({ "ok": false, "reason": format!("{action} is not a WiFi bridge action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [SET_TEXT, SET_BAUD, REBOOT_BRIDGE, RESTORE_DEFAULTS, RESET_COUNTERS, BRIDGE_OPENED].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_packed_four_bytes_per_parameter_like_the_controller() {
        let words = pack("PixRacer");
        assert_eq!(words[0], u32::from_le_bytes(*b"PixR"));
        assert_eq!(words[2], 0);
        assert_eq!(unpack(words), "PixRacer");
        assert_eq!(unpack(pack("a-very-long-network-name")), "a-very-long-netw", "sixteen bytes fit in four parameters");
    }

    #[test]
    fn baud_and_address_read_like_qgc() {
        assert_eq!((baud_index(57600), baud_index(921600), baud_index(12345)), (0, 4, 4), "anything unknown shows as 921600");
        assert_eq!(ip_address(None), "192.168.4.1");
        assert_eq!(ip_address(Some(u32::from_be(u32::from(std::net::Ipv4Addr::new(10, 0, 0, 7))))), "10.0.0.7");
    }
}
