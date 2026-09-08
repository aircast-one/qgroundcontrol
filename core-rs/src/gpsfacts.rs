#![allow(deprecated)]
use mavlink::dialects::ardupilotmega::MavMessage;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GpsFacts {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub hdop: Option<f64>,
    pub vdop: Option<f64>,
    pub course_over_ground: Option<f64>,
    pub lock: u32,
    pub count: u32,
}

fn hundredths(raw: u16) -> Option<f64> {
    (raw != u16::MAX).then(|| raw as f64 / 100.0)
}

fn tenths(raw: u8) -> Option<f64> {
    (raw != u8::MAX).then(|| raw as f64 / 10.0)
}

impl GpsFacts {
    pub fn apply(&mut self, message: &MavMessage) -> bool {
        match message {
            MavMessage::GPS_RAW_INT(d) => {
                self.latitude = Some(d.lat as f64 * 1e-7);
                self.longitude = Some(d.lon as f64 * 1e-7);
                self.count = if d.satellites_visible == 255 { 0 } else { d.satellites_visible as u32 };
                self.hdop = hundredths(d.eph);
                self.vdop = hundredths(d.epv);
                self.course_over_ground = hundredths(d.cog);
                self.lock = d.fix_type as u32;
                true
            }
            MavMessage::HIGH_LATENCY(d) => {
                self.latitude = Some(d.latitude as f64 * 1e-7);
                self.longitude = Some(d.longitude as f64 * 1e-7);
                self.count = 0;
                self.lock = d.gps_fix_type as u32;
                true
            }
            MavMessage::HIGH_LATENCY2(d) => {
                self.latitude = Some(d.latitude as f64 * 1e-7);
                self.longitude = Some(d.longitude as f64 * 1e-7);
                self.count = 0;
                self.hdop = tenths(d.eph);
                self.vdop = tenths(d.epv);
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
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
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
