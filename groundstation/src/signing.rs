use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mavlink::SigningConfig;

use crate::signingkeys::Key;

const SIGNING_EPOCH_UNIX_SECONDS: u64 = 1_420_070_400;

pub fn signing_timestamp(now: SystemTime) -> u64 {
    let since_epoch = now.duration_since(UNIX_EPOCH + Duration::from_secs(SIGNING_EPOCH_UNIX_SECONDS)).unwrap_or_default();
    since_epoch.as_millis() as u64 * 100
}

pub fn config(key: Key, link_id: u8, allow_unsigned: bool) -> SigningConfig {
    SigningConfig::new(key, link_id, true, allow_unsigned)
}

pub struct SetupSigning {
    pub target_system: u8,
    pub target_component: u8,
    pub initial_timestamp: u64,
    pub secret_key: Key,
}

pub fn setup_signing(key: Key, target_system: u8, target_component: u8, now: SystemTime) -> SetupSigning {
    SetupSigning { target_system, target_component, initial_timestamp: signing_timestamp(now), secret_key: key }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn setup_signing_carries_the_target_the_key_and_a_live_timestamp() {
        let setup = setup_signing([7; 32], 1, 1, SystemTime::now());
        assert_ne!(setup.initial_timestamp, 0);
        assert_eq!((setup.target_system, setup.target_component, setup.secret_key), (1, 1, [7; 32]));
        let _config = config([7; 32], 0, true);
    }
}
