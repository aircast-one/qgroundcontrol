#![allow(deprecated)]
use mavlink::dialects::ardupilotmega::MavMessage;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GpsFacts {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub hdop: Option<f64>,
    pub vdop: Option<f64>,
    pub course_over_ground: Option<f64>,
    pub yaw: Option<f64>,
    pub lock: u32,
    pub count: u32,
    pub telemetry: bool,
    pub integrity: Integrity,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Integrity {
    pub system_errors: u32,
    pub spoofing: u8,
    pub jamming: u8,
    pub authentication: u8,
    pub corrections_quality: u8,
    pub system_quality: u8,
    pub signal_quality: u8,
    pub post_processing_quality: u8,
}

pub const GNSS_INTEGRITY: u32 = 441;
const INTEGRITY_WIRE_BYTES: usize = 17;
const NOT_REPORTED: u8 = 255;

pub fn integrity_of(payload: &[u8]) -> (u8, Integrity) {
    let wire: Vec<u8> = payload.iter().copied().chain(std::iter::repeat(0)).take(INTEGRITY_WIRE_BYTES).collect();
    let integrity = Integrity {
        system_errors: u32::from_le_bytes([wire[0], wire[1], wire[2], wire[3]]),
        authentication: wire[9],
        jamming: wire[10],
        spoofing: wire[11],
        corrections_quality: wire[13],
        system_quality: wire[14],
        signal_quality: wire[15],
        post_processing_quality: wire[16],
    };
    (wire[8], integrity)
}

fn reported(value: u8) -> Option<u8> {
    (value != NOT_REPORTED).then_some(value)
}

fn authentication_weight(value: u8) -> i32 {
    match value {
        0 => 0,
        4 => 1,
        1 => 2,
        3 => 3,
        2 => 4,
        _ => -1,
    }
}

pub fn aggregate(first: &Integrity, second: &Integrity) -> (u8, u8, u8) {
    let worst = |a: u8, b: u8| reported(a).max(reported(b)).unwrap_or(NOT_REPORTED);
    let authentication = match authentication_weight(first.authentication) >= authentication_weight(second.authentication) {
        true => first.authentication,
        false => second.authentication,
    };
    (worst(first.spoofing, second.spoofing), worst(first.jamming, second.jamming), reported(authentication).unwrap_or(NOT_REPORTED))
}

fn hundredths(raw: u16) -> Option<f64> {
    (raw != u16::MAX).then(|| raw as f64 / 100.0)
}

fn tenths(raw: u8) -> Option<f64> {
    (raw != u8::MAX).then(|| raw as f64 / 10.0)
}

impl GpsFacts {
    pub fn apply_second(&mut self, message: &MavMessage) -> bool {
        let MavMessage::GPS2_RAW(d) = message else { return false };
        self.latitude = Some(d.lat as f64 * 1e-7);
        self.longitude = Some(d.lon as f64 * 1e-7);
        self.count = if d.satellites_visible == 255 { 0 } else { d.satellites_visible as u32 };
        self.hdop = hundredths(d.eph);
        self.vdop = hundredths(d.epv);
        self.course_over_ground = hundredths(d.cog);
        self.yaw = hundredths(d.yaw);
        self.lock = d.fix_type as u32;
        self.telemetry = true;
        true
    }

    pub fn apply(&mut self, message: &MavMessage) -> bool {
        match message {
            MavMessage::GPS_RAW_INT(d) => {
                self.latitude = Some(d.lat as f64 * 1e-7);
                self.longitude = Some(d.lon as f64 * 1e-7);
                self.count = if d.satellites_visible == 255 { 0 } else { d.satellites_visible as u32 };
                self.hdop = hundredths(d.eph);
                self.vdop = hundredths(d.epv);
                self.course_over_ground = hundredths(d.cog);
                self.yaw = hundredths(d.yaw);
                self.lock = d.fix_type as u32;
                self.telemetry = true;
                true
            }
            MavMessage::HIGH_LATENCY(d) => {
                self.latitude = Some(d.latitude as f64 * 1e-7);
                self.longitude = Some(d.longitude as f64 * 1e-7);
                self.count = 0;
                self.telemetry = true;
                true
            }
            MavMessage::HIGH_LATENCY2(d) => {
                self.latitude = Some(d.latitude as f64 * 1e-7);
                self.longitude = Some(d.longitude as f64 * 1e-7);
                self.count = 0;
                self.hdop = tenths(d.eph);
                self.vdop = tenths(d.epv);
                self.telemetry = true;
                true
            }

            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::{GPS_RAW_INT_DATA, GpsFixType};

    #[test]
    fn gnss_integrity_decodes_from_its_wire_order_even_when_trailing_zeros_were_trimmed() {
        let payload = [5, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 1, 3, 2, 1, 0, 7, 8];
        let (receiver, integrity) = integrity_of(&payload);
        assert_eq!(receiver, 1);
        assert_eq!(
            integrity,
            Integrity { system_errors: 5, authentication: 3, jamming: 2, spoofing: 1, corrections_quality: 7, system_quality: 8, signal_quality: 0, post_processing_quality: 0 }
        );
    }

    #[test]
    fn the_aggregate_takes_the_worst_interference_and_the_most_telling_authentication() {
        let receiver = |spoofing, jamming, authentication| Integrity { spoofing, jamming, authentication, ..Integrity::default() };
        assert_eq!(aggregate(&receiver(1, 3, 3), &receiver(2, 255, 2)), (2, 3, 2), "Error outranks Ok");
        assert_eq!(aggregate(&receiver(255, 255, 255), &receiver(255, 255, 255)), (255, 255, 255));
        assert_eq!(aggregate(&receiver(0, 0, 4), &receiver(0, 0, 0)), (0, 0, 4), "Disabled outranks Unknown");
        assert_eq!(aggregate(&receiver(0, 0, 1), &receiver(0, 0, 4)), (0, 0, 1), "Initializing outranks Disabled");
    }

    #[test]
    fn gps_raw_int_scales_like_the_fact_group() {
        let mut facts = GpsFacts::default();
        let mut data = GPS_RAW_INT_DATA::default();
        data.lat = 473764000;
        data.lon = 85481000;
        data.eph = 121;
        data.epv = u16::MAX;
        data.cog = 27000;
        data.satellites_visible = 11;
        data.fix_type = GpsFixType::GPS_FIX_TYPE_3D_FIX;
        assert!(facts.apply(&MavMessage::GPS_RAW_INT(data)));
        assert!((facts.latitude.unwrap() - 47.3764).abs() < 1e-9);
        assert_eq!(facts.hdop, Some(1.21));
        assert_eq!(facts.vdop, None);
        assert_eq!(facts.course_over_ground, Some(270.0));
        assert_eq!((facts.lock, facts.count), (3, 11));
        let mut unknown = GPS_RAW_INT_DATA::default();
        unknown.satellites_visible = 255;
        facts.apply(&MavMessage::GPS_RAW_INT(unknown));
        assert_eq!(facts.count, 0);
        assert!(!facts.apply(&MavMessage::HEARTBEAT(Default::default())));
    }

    #[test]
    fn the_sample_log_leaves_a_plausible_fix() {
        let bytes = crate::samplelog::bytes();
        let mut facts = GpsFacts::default();
        let mut applied = 0usize;
        crate::tlog::for_each(&bytes, |_, _, message| {
            if facts.apply(message) {
                applied += 1;
            }
        });
        assert!(applied > 0, "no GPS messages in the sample log");
        let (lat, lon) = (facts.latitude.unwrap(), facts.longitude.unwrap());
        assert!((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon));
    }
}
