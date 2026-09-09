pub const AREA_COUNT: u16 = 1;
pub const AREA_RADIUS: u16 = 0;
pub const UNKNOWN_METERS: f32 = -1000.0;
pub const UNKNOWN_LAT: i32 = 0;
pub const UNKNOWN_LON: i32 = 0;
pub const ALLOWED_GPS_DELAY_MS: u64 = 5000;
pub const EPOCH_2019_S: u64 = 1_546_300_800;
pub const COMP_ID_AUTOPILOT1: u8 = 1;
pub const COMP_ID_ODID_TXRX_1: u8 = 236;
pub const COMP_ID_ODID_TXRX_3: u8 = 238;
pub const ARM_STATUS_GOOD_TO_ARM: u8 = 0;
pub const ARM_STATUS_PRE_ARM_FAIL_GENERIC: u8 = 1;
pub const REGION_FAA: i64 = 0;
pub const REGION_EU: i64 = 1;
pub const LOCATION_TAKEOFF: u32 = 0;
pub const LOCATION_LIVE: u32 = 1;
pub const LOCATION_FIXED: u32 = 2;
pub const SELF_ID_EMERGENCY: u32 = 1;
const LUHN_ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyz";

pub fn luhn_mod36(input: &str) -> Option<char> {
    let n = 36i64;
    let sum = input
        .chars()
        .rev()
        .enumerate()
        .try_fold(0i64, |sum, (i, c)| {
            let code = LUHN_ALPHABET.find(c)? as i64;
            let factor = if i % 2 == 0 { 2 } else { 1 };
            let addend = factor * code;
            Some(sum + addend / n + addend % n)
        })?;
    let check = (n - sum % n) % n;
    LUHN_ALPHABET.chars().nth(check as usize)
}

pub fn eu_operator_id_valid(operator_id: &str) -> bool {
    let chars: Vec<char> = operator_id.chars().collect();
    let dashed = chars.contains(&'-');
    if !((chars.len() == 20 && dashed) || (chars.len() == 19 && !dashed)) {
        return false;
    }
    let country: String = chars[..3].iter().collect();
    if !country.chars().all(|c| c.is_ascii_uppercase()) {
        return false;
    }
    let number: String = chars[3..15].iter().collect();
    let checksum = chars[15];
    let secret: String = if dashed { chars[17..20].iter().collect() } else { chars[16..19].iter().collect() };
    luhn_mod36(&format!("{number}{secret}")) == Some(checksum)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub region: i64,
    pub operator_id: String,
    pub operator_id_type: i64,
    pub operator_id_valid: bool,
    pub send_operator_id: bool,
    pub basic_id: String,
    pub basic_id_type: i64,
    pub basic_id_ua_type: i64,
    pub send_basic_id: bool,
    pub send_self_id: bool,
    pub self_id_type: i64,
    pub self_id_free: String,
    pub self_id_emergency: String,
    pub self_id_extended: String,
    pub location_type: u32,
    pub classification_type: u32,
    pub latitude_fixed: f64,
    pub longitude_fixed: f64,
    pub altitude_fixed: f64,
    pub category_eu: u32,
    pub class_eu: u32,
}

pub fn operator_id_good(settings: &Settings) -> (bool, Option<String>) {
    if settings.region == REGION_EU {
        let good = settings.operator_id_valid;
        (good, good.then(|| settings.operator_id.chars().take(16).collect()))
    } else {
        (!settings.operator_id.is_empty() && settings.operator_id_type >= 0, None)
    }
}

pub fn gcs_basic_id_valid(settings: &Settings) -> bool {
    !settings.basic_id.is_empty() && settings.basic_id_type >= 0 && settings.basic_id_ua_type >= 0
}

pub fn self_id_description(settings: &Settings, emergency: bool) -> String {
    let text = match (emergency, settings.self_id_type) {
        (true, _) | (false, 1) => &settings.self_id_emergency,
        (false, 0) => &settings.self_id_free,
        (false, 2) => &settings.self_id_extended,
        _ => &settings.self_id_emergency,
    };
    text.chars().take(23).collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GcsFix {
    pub valid: bool,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub age_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    System { location_type: u32, classification_type: u32, latitude: i32, longitude: i32, area_count: u16, area_radius: u16, category_eu: u32, class_eu: u32, altitude: f32, timestamp_2019: u32, gps_good: bool },
    BasicId { id_type: u32, ua_type: u32, uas_id: String },
    SelfId { description_type: u32, description: String },
    OperatorId { id_type: u32, operator_id: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    Available,
    CommsGood(bool),
    StartSendTimer,
    StopSendTimer,
    StartOdidTimeout,
    ArmStatus { good: bool, error: String },
    BasicIdGood(bool),
    GcsGpsGood(bool),
}

#[derive(Debug, Default)]
pub struct RemoteId {
    pub available: bool,
    pub comms_good: bool,
    pub arm_status_good: bool,
    pub arm_status_error: String,
    pub basic_id_good: bool,
    pub gcs_gps_good: bool,
    pub emergency: bool,
    pub enforce_self_id: bool,
    pub target_system: u8,
}

impl RemoteId {
    pub fn on_arm_status(&mut self, vehicle_id: u8, sysid: u8, compid: u8, status: u8, error: &str) -> Vec<Out> {
        let from_odid = (COMP_ID_ODID_TXRX_1..=COMP_ID_ODID_TXRX_3).contains(&compid);
        if (!from_odid && compid != COMP_ID_AUTOPILOT1) || sysid != vehicle_id {
            return Vec::new();
        }
        let mut out = Vec::new();
        if !self.available {
            self.available = true;
            out.push(Out::Available);
        }
        self.target_system = sysid;
        if !self.comms_good {
            self.comms_good = true;
            out.push(Out::StartSendTimer);
            out.push(Out::CommsGood(true));
        }
        out.push(Out::StartOdidTimeout);
        if status == ARM_STATUS_GOOD_TO_ARM && !self.arm_status_good {
            if !self.basic_id_good {
                self.basic_id_good = true;
                out.push(Out::BasicIdGood(true));
            }
            self.arm_status_good = true;
            out.push(Out::ArmStatus { good: true, error: String::new() });
        }
        if status == ARM_STATUS_PRE_ARM_FAIL_GENERIC {
            self.arm_status_good = false;
            self.arm_status_error = error.to_string();
            if error == "missing basic_id message" {
                self.basic_id_good = false;
                out.push(Out::BasicIdGood(false));
            }
            out.push(Out::ArmStatus { good: false, error: error.to_string() });
        }
        out
    }

    pub fn on_odid_timeout(&mut self) -> Vec<Out> {
        self.comms_good = false;
        vec![Out::StopSendTimer, Out::CommsGood(false)]
    }

    pub fn set_emergency(&mut self, declare: bool) {
        self.emergency = declare;
        self.enforce_self_id = true;
    }

    pub fn messages(&mut self, settings: &Settings, gcs: GcsFix, now_s: u64) -> (Vec<Message>, Vec<Out>) {
        let mut out = Vec::new();
        let (position, good) = match settings.location_type {
            LOCATION_FIXED => {
                let inside = (-90.0..=90.0).contains(&settings.latitude_fixed) && (-180.0..=180.0).contains(&settings.longitude_fixed);
                if inside { ((settings.latitude_fixed, settings.longitude_fixed, settings.altitude_fixed), true) } else { ((0.0, 0.0, 0.0), false) }
            }
            _ if !gcs.valid => ((gcs.latitude, gcs.longitude, gcs.altitude), false),
            _ if settings.region == REGION_FAA && !(gcs.altitude >= 0.0) && self.gcs_gps_good => {
                self.gcs_gps_good = false;
                out.push(Out::GcsGpsGood(false));
                return (Vec::new(), out);
            }
            _ => ((gcs.latitude, gcs.longitude, gcs.altitude), gcs.age_ms <= ALLOWED_GPS_DELAY_MS),
        };
        if good != self.gcs_gps_good {
            self.gcs_gps_good = good;
            out.push(Out::GcsGpsGood(good));
        }
        let mut messages = vec![Message::System {
            location_type: settings.location_type,
            classification_type: settings.classification_type,
            latitude: if good { (position.0 * 1.0e7) as i32 } else { UNKNOWN_LAT },
            longitude: if good { (position.1 * 1.0e7) as i32 } else { UNKNOWN_LON },
            area_count: AREA_COUNT,
            area_radius: AREA_RADIUS,
            category_eu: settings.category_eu,
            class_eu: settings.class_eu,
            altitude: if good { position.2 as f32 } else { UNKNOWN_METERS },
            timestamp_2019: now_s.saturating_sub(EPOCH_2019_S) as u32,
            gps_good: good,
        }];
        if gcs_basic_id_valid(settings) && settings.send_basic_id {
            messages.push(Message::BasicId { id_type: settings.basic_id_type as u32, ua_type: settings.basic_id_ua_type as u32, uas_id: settings.basic_id.chars().take(20).collect() });
        }
        if settings.send_self_id || self.emergency || self.enforce_self_id {
            messages.push(Message::SelfId { description_type: if self.emergency { SELF_ID_EMERGENCY } else { settings.self_id_type as u32 }, description: self_id_description(settings, self.emergency) });
        }
        let (operator_good, _) = operator_id_good(settings);
        if (settings.send_operator_id || settings.region == REGION_EU) && operator_good {
            messages.push(Message::OperatorId { id_type: settings.operator_id_type as u32, operator_id: settings.operator_id.chars().take(20).collect() });
        }
        (messages, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings { region: REGION_FAA, operator_id: "FIN87astrdge12k8".into(), operator_id_type: 0, operator_id_valid: false, send_operator_id: true, basic_id: "1234".into(), basic_id_type: 1, basic_id_ua_type: 2, send_basic_id: true, send_self_id: false, self_id_type: 0, self_id_free: "Survey flight".into(), self_id_emergency: "Emergency".into(), self_id_extended: "Extended".into(), location_type: LOCATION_LIVE, classification_type: 0, latitude_fixed: 0.0, longitude_fixed: 0.0, altitude_fixed: 0.0, category_eu: 0, class_eu: 0 }
    }

    #[test]
    fn the_eu_operator_id_check_is_luhn_mod_36_over_number_and_secret() {
        let secret = "xyz";
        let number = "87astrdge12k";
        let check = luhn_mod36(&format!("{number}{secret}")).unwrap();
        let dashed = format!("FIN{number}{check}-{secret}");
        let plain = format!("FIN{number}{check}{secret}");
        assert!(eu_operator_id_valid(&dashed) && eu_operator_id_valid(&plain));
        assert!(!eu_operator_id_valid(&format!("fin{number}{check}-{secret}")));
        assert!(!eu_operator_id_valid(&format!("FIN{number}0-{secret}")) || check == '0');
        assert!(!eu_operator_id_valid("FIN123"));
        let eu = Settings { region: REGION_EU, operator_id: dashed.clone(), operator_id_valid: true, ..settings() };
        assert_eq!(operator_id_good(&eu), (true, Some(dashed.chars().take(16).collect())));
        assert_eq!(operator_id_good(&Settings { operator_id: String::new(), ..settings() }), (false, None));
        assert_eq!(operator_id_good(&settings()), (true, None));
    }

    #[test]
    fn arm_status_marks_availability_comms_and_basic_id() {
        let mut remote = RemoteId::default();
        assert!(remote.on_arm_status(1, 2, COMP_ID_ODID_TXRX_1, ARM_STATUS_GOOD_TO_ARM, "").is_empty());
        assert!(remote.on_arm_status(1, 1, 42, ARM_STATUS_GOOD_TO_ARM, "").is_empty());
        let first = remote.on_arm_status(1, 1, COMP_ID_AUTOPILOT1, ARM_STATUS_GOOD_TO_ARM, "");
        assert_eq!(first, vec![Out::Available, Out::StartSendTimer, Out::CommsGood(true), Out::StartOdidTimeout, Out::BasicIdGood(true), Out::ArmStatus { good: true, error: String::new() }]);
        let failed = remote.on_arm_status(1, 1, COMP_ID_ODID_TXRX_1, ARM_STATUS_PRE_ARM_FAIL_GENERIC, "missing basic_id message");
        assert_eq!(failed, vec![Out::StartOdidTimeout, Out::BasicIdGood(false), Out::ArmStatus { good: false, error: "missing basic_id message".into() }]);
        assert_eq!(remote.on_odid_timeout(), vec![Out::StopSendTimer, Out::CommsGood(false)]);
        assert!(!remote.comms_good && remote.available);
    }

    #[test]
    fn the_message_set_follows_the_settings_the_fix_and_the_emergency() {
        let mut remote = RemoteId::default();
        let fresh = GcsFix { valid: true, latitude: 47.5, longitude: 8.5, altitude: 400.0, age_ms: 100 };
        let (messages, out) = remote.messages(&settings(), fresh, EPOCH_2019_S + 60);
        assert_eq!(out, vec![Out::GcsGpsGood(true)]);
        assert!(matches!(messages[0], Message::System { latitude: 475000000, longitude: 85000000, timestamp_2019: 60, gps_good: true, .. }));
        assert!(matches!(messages[1], Message::BasicId { id_type: 1, ua_type: 2, ref uas_id } if uas_id == "1234"));
        assert!(matches!(messages[2], Message::OperatorId { .. }));
        assert_eq!(messages.len(), 3);
        let stale = GcsFix { age_ms: ALLOWED_GPS_DELAY_MS + 1, ..fresh };
        let (messages, out) = remote.messages(&settings(), stale, EPOCH_2019_S);
        assert_eq!(out, vec![Out::GcsGpsGood(false)]);
        assert!(matches!(messages[0], Message::System { latitude: UNKNOWN_LAT, altitude, gps_good: false, .. } if altitude == UNKNOWN_METERS));
        remote.set_emergency(true);
        let (messages, _) = remote.messages(&settings(), fresh, EPOCH_2019_S);
        assert!(messages.iter().any(|m| matches!(m, Message::SelfId { description_type: SELF_ID_EMERGENCY, description } if description == "Emergency")));
        let fixed = Settings { location_type: LOCATION_FIXED, latitude_fixed: 10.0, longitude_fixed: 200.0, ..settings() };
        let (messages, _) = remote.messages(&fixed, GcsFix { valid: false, ..fresh }, EPOCH_2019_S);
        assert!(matches!(messages[0], Message::System { gps_good: false, .. }));
        let mut faa = RemoteId { gcs_gps_good: true, ..Default::default() };
        let (messages, out) = faa.messages(&settings(), GcsFix { altitude: -5.0, ..fresh }, EPOCH_2019_S);
        assert!(messages.is_empty() && out == vec![Out::GcsGpsGood(false)]);
    }
}
