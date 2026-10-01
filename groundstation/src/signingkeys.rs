use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::router::Backend;
use crate::settingsini::Setting;
use crate::transport::LinkId;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable"];
pub const ADD_PASSPHRASE_KEY: &str = "signingKeys.addPassphrase";
pub const ADD_RAW_KEY: &str = "signingKeys.addRaw";
pub const GENERATE_KEY: &str = "signingKeys.generate";
pub const REMOVE_KEY: &str = "signingKeys.remove";
pub const EXPORT_KEY: &str = "signingKeys.export";
pub const ENABLE_SIGNING: &str = "signing.enable";
pub const DISABLE_SIGNING: &str = "signing.disable";

pub const KEY_BYTES: usize = 32;
pub const MIN_PASSPHRASE_LENGTH: usize = 8;
const PBKDF2_SALT: &[u8] = b"QGroundControl-MAVLink-Signing-v1";
const PBKDF2_ITERATIONS: u32 = 600_000;
const MAX_KEYS: usize = 64;
const KEYS_SETTINGS_GROUP: &str = "MAVLinkSigningKeys";
const KEY_MANIFEST: &str = "manifest";
const KEY_SUBGROUP: &str = "keys";
const TIMESTAMP_SUBGROUP: &str = "timestamps";
const TIMESTAMP_FLUSH_MS: u64 = 5000;
const ADD_REFUSED: &str = "Could not add key. Name may already exist or input is invalid.";

pub type Key = [u8; KEY_BYTES];

pub fn derive(passphrase: &str, iterations: u32) -> Key {
    pbkdf2::pbkdf2_hmac_array::<sha2::Sha256, KEY_BYTES>(passphrase.as_bytes(), PBKDF2_SALT, iterations)
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn from_hex(text: &str) -> Option<Key> {
    let bytes: Option<Vec<u8>> = (text.len() == KEY_BYTES * 2 && text.is_ascii())
        .then(|| (0..KEY_BYTES).map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()).collect())
        .flatten();
    bytes.and_then(|b| b.try_into().ok())
}

pub fn random_hex() -> Option<String> {
    let mut bytes = [0u8; KEY_BYTES];
    getrandom::fill(&mut bytes).ok()?;
    Some(to_hex(&bytes))
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Keys {
    keys: Vec<(String, Key)>,
    timestamps: BTreeMap<String, u64>,
}

impl Keys {
    pub fn names(&self) -> Vec<String> {
        self.keys.iter().map(|(name, _)| name.clone()).collect()
    }

    pub fn all(&self) -> Vec<(String, Key, u64)> {
        self.keys.iter().map(|(name, key)| (name.clone(), *key, self.last_timestamp(name))).collect()
    }

    pub fn last_timestamp(&self, name: &str) -> u64 {
        self.timestamps.get(name).copied().unwrap_or(0)
    }

    pub fn record(&mut self, batch: &[(String, u64)]) -> bool {
        let newer: Vec<&(String, u64)> = batch.iter().filter(|(name, ts)| self.key(name).is_some() && *ts > self.last_timestamp(name)).collect();
        newer.iter().for_each(|(name, ts)| {
            self.timestamps.insert(name.clone(), *ts);
        });
        !newer.is_empty()
    }

    pub fn key(&self, name: &str) -> Option<Key> {
        self.keys.iter().find(|(n, _)| n == name).map(|(_, key)| *key)
    }

    fn accepts(&self, name: &str) -> bool {
        !name.is_empty() && self.keys.len() < MAX_KEYS && self.key(name).is_none()
    }

    pub fn add(&mut self, name: &str, key: Key) -> bool {
        let accepted = self.accepts(name);
        if accepted {
            self.keys.push((name.to_string(), key));
        }
        accepted
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.keys.len();
        self.keys.retain(|(n, _)| n != name);
        self.timestamps.remove(name);
        self.keys.len() != before
    }

    pub fn from_entries(entries: &BTreeMap<String, Setting>) -> Keys {
        let manifest = match entries.get(&format!("{KEYS_SETTINGS_GROUP}/{KEY_MANIFEST}")) {
            Some(Setting::List(names)) => names.clone(),
            Some(Setting::Text(name)) if !name.is_empty() => vec![name.clone()],
            _ => Vec::new(),
        };
        let keys = manifest
            .iter()
            .filter_map(|name| match entries.get(&format!("{KEYS_SETTINGS_GROUP}/{KEY_SUBGROUP}/{name}")) {
                Some(Setting::Text(hex)) => from_hex(hex).map(|key| (name.clone(), key)),
                _ => None,
            })
            .collect();
        let timestamps = manifest
            .iter()
            .filter_map(|name| match entries.get(&format!("{KEYS_SETTINGS_GROUP}/{TIMESTAMP_SUBGROUP}/{name}")) {
                Some(Setting::Text(ts)) => ts.parse::<u64>().ok().filter(|ts| *ts > 0).map(|ts| (name.clone(), ts)),
                _ => None,
            })
            .collect();
        Keys { keys, timestamps }
    }

    pub fn entries(&self) -> BTreeMap<String, Setting> {
        std::iter::once((format!("{KEYS_SETTINGS_GROUP}/{KEY_MANIFEST}"), Setting::List(self.names())))
            .chain(self.keys.iter().map(|(name, key)| (format!("{KEYS_SETTINGS_GROUP}/{KEY_SUBGROUP}/{name}"), Setting::Text(to_hex(key)))))
            .chain(self.timestamps.iter().map(|(name, ts)| (format!("{KEYS_SETTINGS_GROUP}/{TIMESTAMP_SUBGROUP}/{name}"), Setting::Text(ts.to_string()))))
            .collect()
    }
}

pub fn load() -> Keys {
    Keys::from_entries(&crate::settingsstore::entries_under(KEYS_SETTINGS_GROUP))
}

fn save(keys: &Keys) {
    crate::settingsstore::replace_group(KEYS_SETTINGS_GROUP, keys.entries());
}

fn args(text: &str) -> Vec<String> {
    serde_json::from_str::<Vec<Value>>(text).unwrap_or_default().iter().map(|v| v.as_str().unwrap_or_default().to_string()).collect()
}

fn added(keys: Keys, name: &str, key: Option<Key>) -> Value {
    let mut keys = keys;
    match key.is_some_and(|key| keys.add(name, key)) {
        true => {
            save(&keys);
            json!({ "ok": true })
        }
        false => json!({ "ok": false, "reason": ADD_REFUSED }),
    }
}

pub fn run(path: &str, text: &str) -> Value {
    let given = args(text);
    let arg = |i: usize| given.get(i).map(String::as_str).unwrap_or_default();
    match path {
        ADD_PASSPHRASE_KEY => added(load(), arg(0), (arg(1).chars().count() >= MIN_PASSPHRASE_LENGTH).then(|| derive(arg(1), PBKDF2_ITERATIONS))),
        ADD_RAW_KEY => added(load(), arg(0), from_hex(arg(1))),
        GENERATE_KEY => match random_hex() {
            Some(hex) => json!({ "ok": true, "result": hex }),
            None => json!({ "ok": false, "reason": "The system random source is unavailable." }),
        },
        REMOVE_KEY => {
            let mut keys = load();
            if keys.remove(arg(0)) {
                save(&keys);
            }
            json!({ "ok": true })
        }
        EXPORT_KEY => match load().key(arg(0)) {
            Some(key) => json!({ "ok": true, "result": to_hex(&key) }),
            None => json!({ "ok": false, "reason": format!("No key is called {}", arg(0)) }),
        },
        ENABLE_SIGNING => match load().key(arg(0)) {
            Some(key) => {
                let seed = load().last_timestamp(arg(0));
                change_signing(|signing, link, target, now_ms| signing.begin_enable(link, target, arg(0), key, seed, now_ms))
            }
            None => json!({ "ok": false, "reason": format!("No key is called {}", arg(0)) }),
        },
        DISABLE_SIGNING => change_signing(|signing, link, target, now_ms| signing.begin_disable(link, target, now_ms)),
        _ => json!({ "ok": false, "reason": format!("{path} is not a signing key action") }),
    }
}

fn active_link() -> Option<(LinkId, (u8, u8))> {
    crate::hub::lock().active().map(|v| (v.link, (v.id, v.component)))
}

fn send_setup(link: LinkId, data: &mavlink::dialects::ardupilotmega::SETUP_SIGNING_DATA) {
    (0..crate::signing::SETUP_COPIES).for_each(|_| {
        if let Some(bytes) = crate::mavout::encode_next(&crate::mavout::Outbound::SetupSigning { data: data.clone() }) {
            crate::linkhost::write(&crate::linkhost::TRANSPORTS, link, &bytes);
        }
    });
}


fn change_signing(begin: impl Fn(&mut crate::signing::Signing, LinkId, (u8, u8), u64) -> Result<mavlink::dialects::ardupilotmega::SETUP_SIGNING_DATA, String>) -> Value {
    let Some((link, target)) = active_link() else { return json!({ "ok": false, "reason": "No vehicle is connected." }) };
    let begun = begin(&mut crate::signing::lock(), link, target, crate::hub::now_ms());
    match begun {
        Ok(data) => {
            send_setup(link, &data);
            json!({ "ok": true })
        }
        Err(reason) => json!({ "ok": false, "reason": reason }),
    }
}

static LAST_FLUSH_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn flush_timestamps(now_ms: u64) {
    let last = LAST_FLUSH_MS.load(std::sync::atomic::Ordering::Relaxed);
    if now_ms.saturating_sub(last) < TIMESTAMP_FLUSH_MS {
        return;
    }
    LAST_FLUSH_MS.store(now_ms, std::sync::atomic::Ordering::Relaxed);
    let batch = crate::signing::lock().take_timestamps();
    if batch.is_empty() {
        return;
    }
    let mut keys = load();
    if keys.record(&batch) {
        save(&keys);
    }
}

pub fn tick(now_ms: u64) {
    flush_timestamps(now_ms);
    let (resend, notices) = crate::signing::lock().tick(now_ms);
    resend.iter().for_each(|(link, data)| send_setup(*link, data));
    notices.iter().for_each(|notice| {
        crate::noticeboard::post_from_vehicle(crate::noticeboard::MESSAGE, notice);
    });
}

pub fn owns(path: &str) -> bool {
    [ADD_PASSPHRASE_KEY, ADD_RAW_KEY, GENERATE_KEY, REMOVE_KEY, EXPORT_KEY, ENABLE_SIGNING, DISABLE_SIGNING].contains(&path)
}

pub fn signing_keys_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = crate::read::flag(&crate::read::object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let available = crate::vehiclefacade::switched_on();
    let names = if available { load().names() } else { Vec::new() };
    let armed = crate::hub::lock().active().is_some_and(crate::hub::Vehicle::armed);
    let link = active_link().map(|(link, _)| link);
    let link_name = link.and_then(|link| crate::linkhost::TRANSPORTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).describe(link)).map_or_else(|| "active link".to_string(), |(name, _, _)| name);
    let signing = crate::signing::lock();
    let status = link.map(|link| signing.status(link)).unwrap_or_default();
    json!({
        "kind": "object",
        "class": "SigningKeys",
        "available": available,
        "vehicle": vehicle && link.is_some(),
        "armed": armed,
        "state": if status.state.is_empty() { "off" } else { status.state },
        "linkName": link_name,
        "activeKey": if status.key_name.is_empty() { "None".to_string() } else { status.key_name.clone() },
        "minPassphraseLength": MIN_PASSPHRASE_LENGTH,
        "keys": names.iter().map(|name| json!({ "name": name, "inUse": signing.key_in_use(name), "activeOnVehicle": status.key_name == *name })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_passphrase_is_stretched_with_pbkdf2_hmac_sha256_and_qgcs_salt() {
        assert_eq!(to_hex(&pbkdf2::pbkdf2_hmac_array::<sha2::Sha256, 32>(b"password", b"salt", 1)), "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b", "RFC 7914 vector");
        assert_ne!(derive("correct horse", 2), derive("correct horse", 1));
        assert_eq!(derive("correct horse", 1), pbkdf2::pbkdf2_hmac_array::<sha2::Sha256, 32>(b"correct horse", PBKDF2_SALT, 1));
    }

    #[test]
    fn a_raw_key_is_exactly_64_hex_characters() {
        let hex = random_hex().unwrap();
        assert_eq!(hex.len(), 64);
        assert_eq!(from_hex(&hex).map(|k| to_hex(&k)), Some(hex.clone()));
        assert!(from_hex(&hex[..62]).is_none());
        assert!(from_hex(&"zz".repeat(32)).is_none());
        assert_ne!(random_hex(), Some(hex), "two generated keys differ");
    }

    #[test]
    fn names_are_unique_and_the_store_round_trips_through_the_settings_layout() {
        let mut keys = Keys::default();
        assert!(keys.add("field", [1; 32]));
        assert!(!keys.add("field", [2; 32]), "a name is taken once");
        assert!(!keys.add("", [2; 32]));
        assert!(keys.add("bench", [3; 32]));
        let entries = keys.entries();
        assert_eq!(entries.get("MAVLinkSigningKeys/manifest"), Some(&Setting::List(vec!["field".into(), "bench".into()])));
        assert_eq!(Keys::from_entries(&entries), keys);
        assert!(keys.remove("field"));
        assert_eq!(keys.names(), ["bench"]);
        assert!(!keys.remove("field"));
        assert!(keys.record(&[("bench".into(), 500), ("gone".into(), 900)]));
        assert!(!keys.record(&[("bench".into(), 400)]), "a timestamp only moves forward");
        let restored = Keys::from_entries(&keys.entries());
        assert_eq!((restored.last_timestamp("bench"), restored.last_timestamp("gone")), (500, 0));
        assert_eq!(restored.all(), [("bench".to_string(), [3; 32], 500)]);
    }
}
