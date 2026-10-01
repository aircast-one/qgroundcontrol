use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use serde_json::{Value, json};

use crate::platformserial::Event;

pub struct Hooks {
    pub open: fn(u32, &str) -> bool,
    pub write: fn(u32, &[u8]) -> bool,
    pub close: fn(u32),
    pub devices: fn() -> Vec<String>,
    pub scan: fn(bool) -> bool,
    pub scanning: fn() -> bool,
}

type Receiver = Arc<dyn Fn(Event) + Send + Sync>;

static HOOKS: OnceLock<Hooks> = OnceLock::new();
static RECEIVERS: Mutex<BTreeMap<u32, Receiver>> = Mutex::new(BTreeMap::new());
pub const SCAN: &str = "links.bluetoothScan";

pub fn install(hooks: Hooks) {
    let _ = HOOKS.set(hooks);
}

pub fn available() -> bool {
    HOOKS.get().is_some()
}

pub fn device_from_info(info: &str) -> Option<(String, String)> {
    let (name, address) = info.split_once('\t')?;
    let address = address.trim();
    (!address.is_empty()).then(|| (if name.trim().is_empty() { address.to_string() } else { name.trim().to_string() }, address.to_string()))
}

pub fn devices() -> Vec<(String, String)> {
    HOOKS.get().map(|hooks| (hooks.devices)().iter().filter_map(|info| device_from_info(info)).collect()).unwrap_or_default()
}

pub fn state() -> Value {
    json!({
        "available": available(),
        "scanning": HOOKS.get().is_some_and(|hooks| (hooks.scanning)()),
        "devices": devices().iter().map(|(name, address)| json!({ "name": name, "address": address })).collect::<Vec<_>>(),
    })
}

pub fn scan(args: &str) -> Option<Value> {
    let hooks = HOOKS.get()?;
    let start = serde_json::from_str::<Vec<Value>>(args).ok()?.first()?.as_bool()?;
    Some(json!({ "ok": (hooks.scan)(start) }))
}

fn receiver(id: u32) -> Option<Receiver> {
    RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).get(&id).cloned()
}

pub fn received(id: u32, bytes: Vec<u8>) {
    if let Some(deliver) = receiver(id) {
        deliver(Event::Bytes(bytes));
    }
}

pub fn closed(id: u32, reason: &str) {
    let taken = RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(&id);
    if let Some(deliver) = taken {
        deliver(Event::Disconnected(reason.to_string()));
    }
}

pub struct PlatformBluetooth {
    id: u32,
}

impl PlatformBluetooth {
    pub fn open(id: u32, address: &str, on_event: impl Fn(Event) + Send + Sync + 'static) -> Result<PlatformBluetooth, String> {
        let hooks = HOOKS.get().ok_or_else(|| "this build has no Bluetooth host".to_string())?;
        if address.is_empty() {
            return Err("Pick a Bluetooth device for this link.".to_string());
        }
        RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).insert(id, Arc::new(on_event));
        match (hooks.open)(id, address) {
            true => Ok(PlatformBluetooth { id }),
            false => {
                RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(&id);
                Err(format!("Bluetooth device {address} could not be connected"))
            }
        }
    }

    pub fn write(&self, bytes: &[u8]) -> bool {
        HOOKS.get().is_some_and(|hooks| (hooks.write)(self.id, bytes))
    }

    pub fn close(&self) {
        RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(&self.id);
        if let Some(hooks) = HOOKS.get() {
            (hooks.close)(self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_line_is_its_name_and_address() {
        assert_eq!(device_from_info("HC-05\t98:D3:31:F6:12:34"), Some(("HC-05".to_string(), "98:D3:31:F6:12:34".to_string())));
        assert_eq!(device_from_info("\tAA:BB:CC:DD:EE:FF"), Some(("AA:BB:CC:DD:EE:FF".to_string(), "AA:BB:CC:DD:EE:FF".to_string())), "an unnamed device shows its address, as QGC lists it");
        assert_eq!(device_from_info("no address\t"), None);
        assert_eq!(device_from_info("garbage"), None);
    }
}
