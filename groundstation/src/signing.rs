use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mavlink::SigningConfig;
use sha2::{Digest, Sha256};

const KEY_BYTES: usize = 32;
const SIGNING_EPOCH_UNIX_SECONDS: u64 = 1_420_070_400;

pub fn secret_key(passphrase: &[u8]) -> [u8; KEY_BYTES] {
    match passphrase.is_empty() {
        true => [0u8; KEY_BYTES],
        false => Sha256::digest(passphrase).into(),
    }
}

pub fn signing_timestamp(now: SystemTime) -> u64 {
    let since_epoch = now.duration_since(UNIX_EPOCH + Duration::from_secs(SIGNING_EPOCH_UNIX_SECONDS)).unwrap_or_default();
    since_epoch.as_millis() as u64 * 100
}

pub fn config(passphrase: &[u8], link_id: u8, allow_unsigned: bool) -> SigningConfig {
    SigningConfig::new(secret_key(passphrase), link_id, true, allow_unsigned)
}

pub struct SetupSigning {
    pub target_system: u8,
    pub target_component: u8,
    pub initial_timestamp: u64,
    pub secret_key: [u8; KEY_BYTES],
}

pub fn setup_signing(passphrase: &[u8], target_system: u8, target_component: u8, now: SystemTime) -> SetupSigning {
    SetupSigning { target_system, target_component, initial_timestamp: signing_timestamp(now), secret_key: secret_key(passphrase) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_is_the_sha256_of_the_passphrase_and_zero_when_empty() {
        assert_eq!(secret_key(b"secret_key"), <[u8; 32]>::from(Sha256::digest(b"secret_key")));
        assert_eq!(secret_key(&[b'1'; 32]), <[u8; 32]>::from(Sha256::digest([b'1'; 32])));
        assert_eq!(secret_key(b""), [0u8; 32]);
        assert_ne!(secret_key(b"a"), secret_key(b"b"));
    }

    #[test]
    fn the_timestamp_counts_ten_microsecond_ticks_since_2015() {
        let epoch = UNIX_EPOCH + Duration::from_secs(SIGNING_EPOCH_UNIX_SECONDS);
        assert_eq!(signing_timestamp(epoch), 0);
        assert_eq!(signing_timestamp(epoch + Duration::from_millis(1)), 100);
        assert_eq!(signing_timestamp(epoch + Duration::from_secs(1)), 100_000);
        assert_eq!(signing_timestamp(UNIX_EPOCH), 0);
        assert!(signing_timestamp(SystemTime::now()) > 0);
    }

    #[test]
    fn setup_signing_carries_the_target_and_a_live_timestamp() {
        let setup = setup_signing(b"secret_key", 1, 1, SystemTime::now());
        assert_ne!(setup.initial_timestamp, 0);
        assert_eq!((setup.target_system, setup.target_component), (1, 1));
        assert_eq!(setup.secret_key, secret_key(b"secret_key"));
        let _config = config(b"secret_key", 0, true);
    }
}
