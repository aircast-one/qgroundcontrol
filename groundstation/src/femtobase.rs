use crate::gpsrtk::{BasePlan, BasePosition};
use crate::sbfbase::heard_within;
use crate::ubxbase::{BaseDriver, Event, Frame, RTCM3_PREAMBLE, Scan, SurveyStatus, Transport, scan};

const FEMTO_BAUD: u32 = 115200;
const ACK_WINDOW_MS: u64 = 400;
const MAX_NMEA: usize = 595;
const GGA_COMMAS: usize = 14;
const BASE_FIXED_QUALITY: i64 = 7;
const UNLOGALL: (&str, &str) = ("UNLOGALL THISPORT\r\n", "<UNLOGALL OK");
const VERSION: (&str, &str) = ("VERSION\r\n", "<VERSION OK");
const POSAVE: (&str, &str) = ("POSAVE ON \r\n", "<POSAVE OK");
const LOG_GGA: (&str, &str) = ("LOG GPGGA 1 \r\n", "<LOG OK");
const LOG_RTCM: (&str, &str) = ("LOG RTCM 1\r\n", "<LOG OK");
const FIX_OK: &str = "FIX OK";

enum Message {
    Nmea(String),
    Rtcm(Vec<u8>),
}

enum Cut {
    Take(Option<Message>, usize),
    Wait,
}

fn next_start(buffer: &[u8], from: usize) -> usize {
    buffer.iter().skip(from).position(|byte| *byte == b'$' || *byte == RTCM3_PREAMBLE).map_or(buffer.len(), |at| at + from)
}

fn nmea_checksum(body: &[u8]) -> String {
    format!("{:02X}", body.iter().fold(0u8, |sum, byte| sum ^ byte))
}

fn nmea(buffer: &[u8]) -> Cut {
    let restart = next_start(buffer, 1);
    match buffer.iter().position(|byte| *byte == b'*') {
        Some(star) if star < restart => match buffer.get(star + 1..star + 3) {
            None => Cut::Wait,
            Some(sum) => Cut::Take((sum == nmea_checksum(&buffer[1..star]).as_bytes()).then(|| Message::Nmea(String::from_utf8_lossy(&buffer[..star]).into_owned())), star + 3),
        },
        _ if restart < buffer.len() => Cut::Take(None, restart),
        _ if buffer.len() >= MAX_NMEA => Cut::Take(None, 1),
        _ => Cut::Wait,
    }
}

fn cut(buffer: &[u8]) -> Cut {
    match buffer.first() {
        None => Cut::Wait,
        Some(&b'$') => nmea(buffer),
        Some(&RTCM3_PREAMBLE) => match scan(buffer) {
            Scan::Frame(Frame::Rtcm(bytes), used) => Cut::Take(Some(Message::Rtcm(bytes)), used),
            Scan::Frame(Frame::Ubx { .. }, used) | Scan::Skip(used) => Cut::Take(None, used),
            Scan::Incomplete => Cut::Wait,
        },
        Some(_) => Cut::Take(None, next_start(buffer, 1)),
    }
}

fn nmea_degrees(ddmm: f64) -> f64 {
    let degrees = (ddmm * 0.01).trunc();
    degrees + (ddmm * 0.01 - degrees) * 100.0 / 60.0
}

struct Gga {
    latitude: f64,
    longitude: f64,
    altitude_m: f64,
    quality: i64,
    satellites: u8,
}

fn gga(sentence: &str) -> Option<Gga> {
    let fields: Vec<&str> = sentence.split(',').collect();
    let field = |index: usize| fields.get(index).copied().unwrap_or("");
    let number = |index: usize| field(index).parse::<f64>().unwrap_or(0.0);
    let whole = |index: usize| field(index).parse::<i64>().unwrap_or(0);
    let signed = |value: f64, negative: &str, index: usize| if field(index).starts_with(negative) { -value } else { value };
    (sentence.get(3..6) == Some("GGA") && fields.len() == GGA_COMMAS + 1).then(|| Gga {
        latitude: signed(nmea_degrees(number(2)), "S", 3),
        longitude: signed(nmea_degrees(number(4)), "W", 5),
        altitude_m: number(9),
        quality: whole(6),
        satellites: whole(7) as u8,
    })
}

pub struct FemtoBase<T: Transport> {
    pub transport: T,
    buffer: Vec<u8>,
    plan: BasePlan,
    configured: bool,
    transport_lost: bool,
    corrections_on: bool,
    survey_from_ms: Option<u64>,
    survey_s: u32,
    events: Vec<Event>,
}

impl<T: Transport> FemtoBase<T> {
    pub fn new(transport: T, plan: BasePlan) -> FemtoBase<T> {
        FemtoBase { transport, buffer: Vec::new(), plan, configured: false, transport_lost: false, corrections_on: false, survey_from_ms: None, survey_s: 0, events: Vec::new() }
    }

    fn acked(&mut self, (command, reply): (&str, &str)) -> bool {
        self.transport.write(command.as_bytes()) && heard_within(&mut self.transport, ACK_WINDOW_MS, &mut self.transport_lost, |heard| heard.contains(reply)).is_some()
    }

    fn status(&self, active: bool, valid: bool, latitude: f64, longitude: f64, altitude_m: f32) -> Event {
        Event::SurveyIn(SurveyStatus { duration_s: self.survey_s, mean_accuracy_mm: 0, latitude, longitude, altitude_m, flags: u8::from(valid) | (u8::from(active) << 1) })
    }

    fn start_corrections(&mut self) {
        self.acked(LOG_RTCM);
        self.corrections_on = true;
    }

    fn start_survey(&mut self) {
        if self.acked(POSAVE) {
            self.acked(LOG_GGA);
        }
        self.survey_s = 0;
        self.survey_from_ms = Some(self.transport.now_ms());
        self.events.push(self.status(true, false, 0.0, 0.0, 0.0));
    }

    fn fix_position(&mut self, position: BasePosition) {
        let command = format!("FIX POSITION {:.8} {:.8} {:.5}\r\n", position.latitude, position.longitude, f64::from(position.altitude_m));
        if self.acked((&command, FIX_OK)) {
            self.start_corrections();
            self.events.push(self.status(false, true, position.latitude, position.longitude, position.altitude_m));
            self.acked(LOG_GGA);
        }
    }

    fn sentence(&mut self, sentence: &str) {
        if let Some(fix) = gga(sentence) {
            if !self.corrections_on && fix.quality == BASE_FIXED_QUALITY {
                self.survey_from_ms = None;
                self.events.push(self.status(false, true, fix.latitude, fix.longitude, fix.altitude_m as f32));
                self.start_corrections();
            }
            self.events.push(Event::Satellites { count: fix.satellites, list: Vec::new() });
        }
        if let Some(from) = self.survey_from_ms {
            let seconds = (self.transport.now_ms().saturating_sub(from) / 1000) as u32;
            if seconds != self.survey_s {
                self.survey_s = seconds;
                self.events.push(self.status(true, false, 0.0, 0.0, 0.0));
            }
        }
    }

    fn messages(&mut self) -> Vec<Message> {
        let steps: Vec<(Option<Message>, usize)> = std::iter::successors(Some((None, 0usize)), |(_, at)| match cut(&self.buffer[*at..]) {
            Cut::Take(message, used) => Some((message, at + used)),
            Cut::Wait => None,
        })
        .collect();
        let consumed = steps.last().map_or(0, |(_, at)| *at);
        self.buffer.drain(..consumed);
        steps.into_iter().filter_map(|(message, _)| message).collect()
    }
}

impl<T: Transport> BaseDriver for FemtoBase<T> {
    fn configure(&mut self) -> bool {
        self.configured = false;
        self.corrections_on = false;
        self.survey_from_ms = None;
        self.transport.set_baud(FEMTO_BAUD);
        if !(0..2).any(|_| self.acked(UNLOGALL) && self.acked(VERSION)) {
            return false;
        }
        self.buffer.clear();
        match self.plan {
            BasePlan::SurveyIn(_) => self.start_survey(),
            BasePlan::Fixed(position) => self.fix_position(position),
        }
        self.configured = true;
        true
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<bool> {
        let Some(bytes) = self.transport.read(timeout_ms) else {
            self.transport_lost = true;
            return None;
        };
        if !self.configured {
            return Some(false);
        }
        self.buffer.extend_from_slice(&bytes);
        let before = self.events.len();
        self.messages().into_iter().for_each(|message| match message {
            Message::Nmea(sentence) => self.sentence(&sentence),
            Message::Rtcm(frame) => self.events.push(Event::Rtcm(frame)),
        });
        Some(self.events.len() > before)
    }

    fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    fn transport_lost(&self) -> bool {
        self.transport_lost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpsrtk::SurveySpec;
    use std::collections::VecDeque;

    struct Receiver {
        written: Vec<String>,
        pending: VecDeque<u8>,
        clock_ms: u64,
        answers: bool,
        bauds: Vec<u32>,
    }

    impl Transport for Receiver {
        fn read(&mut self, timeout_ms: u64) -> Option<Vec<u8>> {
            match self.pending.is_empty() {
                true => {
                    self.clock_ms += timeout_ms.max(1);
                    Some(Vec::new())
                }
                false => Some(self.pending.drain(..).collect()),
            }
        }

        fn write(&mut self, bytes: &[u8]) -> bool {
            let text = String::from_utf8_lossy(bytes).to_string();
            let word = text.split_whitespace().next().unwrap_or("").to_string();
            let reply = match (self.answers, word.as_str()) {
                (false, _) => String::new(),
                (true, "FIX") => "FIX OK".to_string(),
                (true, _) => format!("<{word} OK"),
            };
            self.pending.extend(reply.bytes());
            self.written.push(text);
            true
        }

        fn set_baud(&mut self, baud: u32) -> bool {
            self.bauds.push(baud);
            true
        }

        fn now_ms(&self) -> u64 {
            self.clock_ms
        }
    }

    fn receiver() -> Receiver {
        Receiver { written: Vec::new(), pending: VecDeque::new(), clock_ms: 0, answers: true, bauds: Vec::new() }
    }

    fn survey() -> BasePlan {
        BasePlan::SurveyIn(SurveySpec { accuracy_m: 2.0, accuracy_driver_units: 20000.0, duration_s: 180 })
    }

    const SURVEYING: &str = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76\r\n";
    const SURVEYED: &str = "$GPGGA,092751.000,5321.6802,N,00630.3372,W,7,12,1.03,61.7,M,55.2,M,,*4A\r\n";

    #[test]
    fn a_surveying_base_averages_its_position_then_streams_rtcm_like_gps_driver_femto() {
        let mut base = FemtoBase::new(receiver(), survey());
        assert!(base.configure());
        assert_eq!(base.transport.written, [UNLOGALL.0, VERSION.0, POSAVE.0, LOG_GGA.0]);
        assert_eq!(base.transport.bauds, [FEMTO_BAUD]);
        assert_eq!(base.take_events(), [base.status(true, false, 0.0, 0.0, 0.0)]);

        base.transport.clock_ms += 2_500;
        base.transport.pending.extend(SURVEYING.bytes());
        assert_eq!(base.receive(100), Some(true));
        let Event::SurveyIn(ticking) = base.take_events().remove(1) else { panic!("survey tick expected") };
        assert_eq!((ticking.duration_s, ticking.flags), (2, 0b10));

        base.transport.pending.extend(SURVEYED.bytes());
        assert_eq!(base.receive(100), Some(true));
        let events = base.take_events();
        let Event::SurveyIn(done) = events[0] else { panic!("finished survey expected") };
        assert_eq!(done.flags, 0b01);
        assert!((done.latitude - 53.361_336_666).abs() < 1e-8 && (done.longitude + 6.505_62).abs() < 1e-8, "{done:?}");
        assert_eq!(done.altitude_m, 61.7);
        assert_eq!(events[1], Event::Satellites { count: 12, list: Vec::new() });
        assert_eq!(base.transport.written.last().map(String::as_str), Some(LOG_RTCM.0));

        base.transport.pending.extend([b'x', 0xd3, 0x00, 0x01, 0x42, 9, 9, 9]);
        assert_eq!(base.receive(100), Some(true));
        assert_eq!(base.take_events(), [Event::Rtcm(vec![0xd3, 0x00, 0x01, 0x42, 9, 9, 9])]);
    }

    #[test]
    fn a_fixed_base_is_told_its_position_and_streams_at_once() {
        let fixed = BasePlan::Fixed(BasePosition { latitude: 47.3977419, longitude: 8.5455938, altitude_m: 488.0, accuracy_mm: 1500.0 });
        let mut base = FemtoBase::new(receiver(), fixed);
        assert!(base.configure());
        assert_eq!(base.transport.written[2..], ["FIX POSITION 47.39774190 8.54559380 488.00000\r\n", LOG_RTCM.0, LOG_GGA.0]);
        assert_eq!(base.take_events(), [base.status(false, true, 47.3977419, 8.5455938, 488.0)]);
        base.transport.pending.extend(SURVEYED.bytes());
        base.receive(100);
        assert_eq!(base.take_events(), [Event::Satellites { count: 12, list: Vec::new() }], "a fixed base never re-reports a finished survey");
    }

    #[test]
    fn a_sentence_with_a_bad_checksum_or_a_split_read_is_handled_like_the_driver() {
        let mut base = FemtoBase::new(receiver(), survey());
        base.configure();
        base.take_events();
        base.transport.pending.extend(SURVEYED.replace("*4A", "*4B").bytes());
        assert_eq!(base.receive(100), Some(false));
        let (head, tail) = SURVEYED.split_at(30);
        base.transport.pending.extend(head.bytes());
        assert_eq!(base.receive(100), Some(false));
        base.transport.pending.extend(tail.bytes());
        assert_eq!(base.receive(100), Some(true));
    }

    #[test]
    fn a_receiver_that_never_answers_is_tried_twice_and_not_configured() {
        let mut base = FemtoBase::new(Receiver { answers: false, ..receiver() }, survey());
        assert!(!base.configure());
        assert_eq!(base.transport.written, [UNLOGALL.0, UNLOGALL.0]);
    }
}
