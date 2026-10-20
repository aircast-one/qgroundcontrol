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

struct Hosted {
    devices: Vec<String>,
    scanning: bool,
}

static HOOKS: OnceLock<Hooks> = OnceLock::new();
static RECEIVERS: Mutex<BTreeMap<u32, Receiver>> = Mutex::new(BTreeMap::new());
static HOSTED: Mutex<Hosted> = Mutex::new(Hosted {
    devices: Vec::new(),
    scanning: false,
});
static REFUSALS: Mutex<BTreeMap<u32, String>> = Mutex::new(BTreeMap::new());
pub const SCAN: &str = "links.bluetoothScan";

pub fn install(hooks: Hooks) {
    let _ = HOOKS.set(hooks);
}

pub fn available() -> bool {
    HOOKS.get().is_some()
}

pub fn host_devices(lines: &str) {
    HOSTED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .devices = lines.lines().map(str::to_string).collect();
}

pub fn hosted_devices() -> Vec<String> {
    HOSTED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .devices
        .clone()
}

pub fn host_scanning(scanning: bool) {
    HOSTED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .scanning = scanning;
}

pub fn hosted_scanning() -> bool {
    HOSTED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .scanning
}

pub fn refused(id: u32, reason: &str) {
    let reason = reason.trim();
    if !reason.is_empty() {
        REFUSALS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, reason.to_string());
    }
}

fn refusal(id: u32, address: &str) -> String {
    REFUSALS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&id)
        .unwrap_or_else(|| format!("Bluetooth device {address} could not be connected"))
}

pub fn device_from_info(info: &str) -> Option<(String, String)> {
    let (name, address) = info.split_once('\t')?;
    let address = address.trim();
    (!address.is_empty()).then(|| {
        (
            if name.trim().is_empty() {
                address.to_string()
            } else {
                name.trim().to_string()
            },
            address.to_string(),
        )
    })
}

pub fn devices() -> Vec<(String, String)> {
    HOOKS
        .get()
        .map(|hooks| {
            (hooks.devices)()
                .iter()
                .filter_map(|info| device_from_info(info))
                .collect()
        })
        .unwrap_or_default()
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
    let start = serde_json::from_str::<Vec<Value>>(args)
        .ok()?
        .first()?
        .as_bool()?;
    Some(json!({ "ok": (hooks.scan)(start) }))
}

fn receiver(id: u32) -> Option<Receiver> {
    RECEIVERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&id)
        .cloned()
}

pub fn received(id: u32, bytes: Vec<u8>) {
    if let Some(deliver) = receiver(id) {
        deliver(Event::Bytes(bytes));
    }
}

pub fn closed(id: u32, reason: &str) {
    let taken = RECEIVERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&id);
    if let Some(deliver) = taken {
        deliver(Event::Disconnected(reason.to_string()));
    }
}

pub struct PlatformBluetooth {
    id: u32,
}

impl PlatformBluetooth {
    pub fn open(
        id: u32,
        address: &str,
        on_event: impl Fn(Event) + Send + Sync + 'static,
    ) -> Result<PlatformBluetooth, String> {
        let hooks = HOOKS
            .get()
            .ok_or_else(|| "this build has no Bluetooth host".to_string())?;
        if address.is_empty() {
            return Err("Pick a Bluetooth device for this link.".to_string());
        }
        RECEIVERS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, Arc::new(on_event));
        let opened = (hooks.open)(id, address);
        let refusal = refusal(id, address);
        match opened {
            true => Ok(PlatformBluetooth { id }),
            false => {
                RECEIVERS
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(&id);
                Err(refusal)
            }
        }
    }

    pub fn write(&self, bytes: &[u8]) -> bool {
        HOOKS
            .get()
            .is_some_and(|hooks| (hooks.write)(self.id, bytes))
    }

    pub fn close(&self) {
        RECEIVERS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.id);
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
        assert_eq!(
            device_from_info("HC-05\t98:D3:31:F6:12:34"),
            Some(("HC-05".to_string(), "98:D3:31:F6:12:34".to_string()))
        );
        assert_eq!(
            device_from_info("\tAA:BB:CC:DD:EE:FF"),
            Some((
                "AA:BB:CC:DD:EE:FF".to_string(),
                "AA:BB:CC:DD:EE:FF".to_string()
            )),
            "an unnamed device shows its address, as QGC lists it"
        );
        assert_eq!(device_from_info("no address\t"), None);
        assert_eq!(device_from_info("garbage"), None);
    }

    #[test]
    fn a_native_host_pushes_the_devices_it_found_and_whether_it_is_scanning() {
        host_devices("HC-05\t98:D3:31:F6:12:34\n\tAA:BB:CC:DD:EE:FF\nnot a device");
        host_scanning(true);
        assert_eq!(
            hosted_devices()
                .iter()
                .filter_map(|info| device_from_info(info))
                .collect::<Vec<_>>(),
            [
                ("HC-05".to_string(), "98:D3:31:F6:12:34".to_string()),
                (
                    "AA:BB:CC:DD:EE:FF".to_string(),
                    "AA:BB:CC:DD:EE:FF".to_string()
                ),
            ],
            "one name<TAB>address line per device, as BluetoothLinks.devicesInfo answers on Android"
        );
        assert!(hosted_scanning());
        host_devices("");
        host_scanning(false);
        assert!(hosted_devices().is_empty() && !hosted_scanning());
    }

    #[test]
    fn a_refused_open_says_why_once_and_falls_back_to_the_plain_message() {
        refused(9001, "  This PC has no Bluetooth radio.  ");
        refused(9002, "   ");
        assert_eq!(
            refusal(9001, "98:D3:31:F6:12:34"),
            "This PC has no Bluetooth radio."
        );
        assert_eq!(
            refusal(9001, "98:D3:31:F6:12:34"),
            "Bluetooth device 98:D3:31:F6:12:34 could not be connected",
            "a reason is used for the open that reported it, not for the next one"
        );
        assert_eq!(
            refusal(9002, "AA:BB:CC:DD:EE:FF"),
            "Bluetooth device AA:BB:CC:DD:EE:FF could not be connected",
            "a blank reason keeps the plain message, as Android's host never gives one"
        );
    }
}
