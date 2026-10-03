use crate::femtobase::{Message, nmea_degrees, take_messages};
use crate::gpsrtk::{ASHTECH_BAUD, BasePlan, BasePosition, MAX_SATELLITES, Satellite};
use crate::sbfbase::heard_within;
use crate::ubxbase::{BaseDriver, Event, SurveyStatus, Transport};

const ASH_RESPONSE_TIMEOUT_MS: u64 = 200;
const PORT_QUERY: &str = "$PASHQ,PRT\r\n";
const BOARD_QUERY: &str = "$PASHQ,RID\r\n";
const SETUP: [&str; 2] = ["$PASHS,POP,20\r\n", "$PASHS,SNS,SOL\r\n"];
const MESSAGES: [&str; 7] = [
    "$PASHS,NME,ALL,{port},OFF\r\n",
    "$PASHS,ATM,ALL,{port},OFF\r\n",
    "$PASHS,OUT,{port},ON\r\n",
    "$PASHS,NME,ZDA,{port},ON,3\r\n",
    "$PASHS,NME,GST,{port},ON,3\r\n",
    "$PASHS,NME,POS,{port},ON,0.05\r\n",
    "$PASHS,NME,GSV,{port},ON,1\r\n",
];
const RTCM_OUTPUT: [&str; 14] = [
    "$PASHS,NME,POS,{port},ON,0.2\r\n",
    "$PASHS,RT3,1074,{port},ON,1\r\n",
    "$PASHS,RT3,1084,{port},ON,1\r\n",
    "$PASHS,RT3,1094,{port},ON,1\r\n",
    "$PASHS,RT3,1114,{port},ON,1\r\n",
    "$PASHS,RT3,1124,{port},ON,1\r\n",
    "$PASHS,RT3,1006,{port},ON,1\r\n",
    "$PASHS,RT3,1033,{port},ON,31\r\n",
    "$PASHS,RT3,1013,{port},ON,1\r\n",
    "$PASHS,RT3,1029,{port},ON,1\r\n",
    "$PASHS,RT3,1230,{port},ON\r\n",
    "$PASHS,RT3,1005,{port},ON,1\r\n",
    "$PASHS,RT3,1077,{port},ON,1\r\n",
    "$PASHS,RT3,1087,{port},ON,1\r\n",
];
const SURVEY_BASE: [&str; 2] = ["$PASHS,ANP,OWN,TRM55971.00\r\n", "$PASHS,STI,0001\r\n"];
const POS_COMMAS: usize = 18;
const PRT_COMMAS: usize = 3;
const SATELLITES_PER_GSV: usize = 4;

fn sentences(heard: &str) -> Vec<String> {
    take_messages(&mut heard.as_bytes().to_vec())
        .into_iter()
        .filter_map(|message| match message {
            Message::Nmea(sentence) => Some(sentence),
            Message::Rtcm(_) => None,
        })
        .collect()
}

fn commas(sentence: &str) -> usize {
    sentence.matches(',').count()
}

fn ack(sentence: &str) -> Option<bool> {
    match sentence.get(..10) {
        Some("$PASHR,ACK") => Some(true),
        Some("$PASHR,NAK") => Some(false),
        _ => None,
    }
}

fn port_of(sentence: &str) -> Option<char> {
    (sentence.starts_with("$PASHR,PRT,") && commas(sentence) == PRT_COMMAS).then(|| sentence.chars().nth(11)).flatten()
}

fn mb_two(sentence: &str) -> Option<bool> {
    sentence.strip_prefix("$PASHR,RID,").map(|board| board.starts_with("MB2"))
}

fn receipt(sentence: &str) -> Option<()> {
    sentence.starts_with("$PASHR,RECEIPT,").then_some(())
}

fn leading_number(field: &str) -> Option<f64> {
    let end = field.char_indices().find(|(at, c)| !(c.is_ascii_digit() || *c == '.' || (*at == 0 && (*c == '-' || *c == '+')))).map_or(field.len(), |(at, _)| at);
    field[..end].parse().ok()
}

fn leading_int(field: &str) -> i64 {
    leading_number(field).map_or(0, |value| value as i64)
}

fn ddmm(degrees: f64) -> f64 {
    degrees.trunc() * 100.0 + degrees.fract() * 60.0
}

fn hemisphere(value: f64, positive: char, negative: char) -> (f64, char) {
    if value < 0.0 { (-value, negative) } else { (value, positive) }
}

fn fixed_position_command(position: BasePosition) -> String {
    let (latitude, ns) = hemisphere(position.latitude, 'N', 'S');
    let (longitude, ew) = hemisphere(position.longitude, 'E', 'W');
    format!("$PASHS,POS,{:.8},{ns},{:.8},{ew},{:.5},PC1", ddmm(latitude), ddmm(longitude), f64::from(position.altitude_m))
}

fn signed(value: f64, sector: &str, negative: &str) -> f64 {
    if sector.starts_with(negative) { -value } else { value }
}

fn surveyed_position(sentence: &str) -> Option<(f64, f64, f32)> {
    let (_, finished) = sentence.split_once("FINISHED,")?;
    let fields: Vec<&str> = finished.split(',').collect();
    let field = |index: usize| fields.get(index).copied().unwrap_or("");
    let number = |index: usize| leading_number(field(index)).unwrap_or(0.0);
    (!sentence.contains("ERR")).then(|| (nmea_degrees(signed(number(2), field(3), "S")), nmea_degrees(signed(number(4), field(5), "W")), number(6) as f32))
}

fn position_found(sentence: &str) -> bool {
    let fields: Vec<&str> = sentence.split(',').collect();
    sentence.starts_with("$PASHR,POS,") && commas(sentence) == POS_COMMAS && [5, 7, 9].iter().filter(|index| fields.get(**index).and_then(|field| leading_number(field)).is_some()).count() == 3
}

pub struct AshtechBase<T: Transport> {
    pub transport: T,
    buffer: Vec<u8>,
    plan: BasePlan,
    port: char,
    mb_two: bool,
    configured: bool,
    transport_lost: bool,
    corrections_on: bool,
    survey_from_ms: Option<u64>,
    survey_s: u32,
    satellites: [Satellite; MAX_SATELLITES],
    events: Vec<Event>,
}

impl<T: Transport> AshtechBase<T> {
    pub fn new(transport: T, plan: BasePlan) -> AshtechBase<T> {
        AshtechBase {
            transport,
            buffer: Vec::new(),
            plan,
            port: 'A',
            mb_two: false,
            configured: false,
            transport_lost: false,
            corrections_on: false,
            survey_from_ms: None,
            survey_s: 0,
            satellites: [Satellite::default(); MAX_SATELLITES],
            events: Vec::new(),
        }
    }

    fn reply<R>(&mut self, command: &str, pick: impl Fn(&str) -> Option<R>) -> Option<R> {
        if !self.transport.write(command.as_bytes()) {
            return None;
        }
        heard_within(&mut self.transport, ASH_RESPONSE_TIMEOUT_MS, &mut self.transport_lost, |heard| sentences(heard).iter().any(|sentence| pick(sentence).is_some()))
            .and_then(|heard| sentences(&heard).iter().find_map(|sentence| pick(sentence)))
    }

    fn acked(&mut self, command: &str) -> bool {
        self.reply(command, ack) == Some(true)
    }

    fn on_port(&self, template: &str) -> String {
        template.replace("{port}", &self.port.to_string())
    }

    fn acked_on_port(&mut self, templates: &[&str]) {
        templates.iter().for_each(|template| {
            let command = self.on_port(template);
            self.acked(&command);
        });
    }

    fn status(&self, active: bool, valid: bool, latitude: f64, longitude: f64, altitude_m: f32) -> Event {
        Event::SurveyIn(SurveyStatus { duration_s: self.survey_s, mean_accuracy_mm: 0, latitude, longitude, altitude_m, flags: u8::from(valid) | (u8::from(active) << 1) })
    }

    fn unknown_status(&self, active: bool, valid: bool) -> Event {
        self.status(active, valid, f64::NAN, f64::NAN, f32::NAN)
    }

    fn start_corrections(&mut self) {
        self.corrections_on = true;
        match self.plan {
            BasePlan::SurveyIn(spec) => {
                self.reply(&format!("$PASHS,POS,AVG,{}\r\n", spec.duration_s), receipt);
                SURVEY_BASE.iter().for_each(|command| {
                    self.acked(command);
                });
                self.survey_s = 0;
                self.survey_from_ms = Some(self.transport.now_ms());
                self.events.push(self.unknown_status(true, false));
            }
            BasePlan::Fixed(position) => {
                self.acked(&fixed_position_command(position));
                self.acked_on_port(&RTCM_OUTPUT);
                self.events.push(self.status(false, true, position.latitude, position.longitude, position.altitude_m));
            }
        }
    }

    fn satellites_in_view(&mut self, sentence: &str) {
        let fields: Vec<&str> = sentence.split(',').collect();
        let number = |index: usize| leading_int(fields.get(index).copied().unwrap_or(""));
        let (all, this, visible) = (number(1), number(2), number(3));
        if !sentence.starts_with("$GP") || this < 1 || this > all {
            return;
        }
        let last = this == all;
        let end = if last { visible - (this - 1) * SATELLITES_PER_GSV as i64 } else { SATELLITES_PER_GSV as i64 };
        (0..end.clamp(0, SATELLITES_PER_GSV as i64) as usize).for_each(|y| {
            let field = |offset: usize| number(4 + y * 4 + offset) as u8;
            let slot = y + (this as usize - 1) * SATELLITES_PER_GSV;
            if let Some(satellite) = self.satellites.get_mut(slot) {
                *satellite = Satellite { svid: field(0), used: field(3) > 0, elevation_raw: field(1), azimuth_raw: field(2), snr_db: field(3), prn: 0 };
            }
        });
        if last {
            let count = visible.clamp(0, MAX_SATELLITES as i64) as u8;
            self.events.push(Event::Satellites { count, list: self.satellites[..count as usize].to_vec() });
        }
    }

    fn sentence(&mut self, sentence: &str) {
        if sentence.get(3..7) == Some("GSV,") {
            self.satellites_in_view(sentence);
        } else if position_found(sentence) && self.mb_two && !self.corrections_on {
            self.start_corrections();
        } else if receipt(sentence).is_some() && sentence.contains("FINISHED,") {
            self.survey_from_ms = None;
            match surveyed_position(sentence) {
                None => self.events.push(self.unknown_status(false, false)),
                Some((latitude, longitude, altitude_m)) => {
                    self.events.push(self.status(false, true, latitude, longitude, altitude_m));
                    self.acked_on_port(&RTCM_OUTPUT);
                }
            }
        }
        if let Some(from) = self.survey_from_ms {
            let seconds = (self.transport.now_ms().saturating_sub(from) / 1000) as u32;
            if seconds != self.survey_s {
                self.survey_s = seconds;
                self.events.push(self.unknown_status(true, false));
            }
        }
    }
}

impl<T: Transport> BaseDriver for AshtechBase<T> {
    fn configure(&mut self) -> bool {
        self.configured = false;
        self.corrections_on = false;
        self.survey_from_ms = None;
        self.transport.set_baud(ASHTECH_BAUD);
        let Some(port) = (0..2).find_map(|_| self.reply(PORT_QUERY, port_of)) else {
            return false;
        };
        self.port = port;
        let Some(mb_two) = self.reply(BOARD_QUERY, mb_two) else {
            return false;
        };
        self.mb_two = mb_two;
        SETUP.iter().for_each(|command| {
            self.acked(command);
        });
        self.acked_on_port(&MESSAGES);
        self.buffer.clear();
        if self.mb_two {
            self.events.push(self.unknown_status(true, false));
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
        take_messages(&mut self.buffer).into_iter().for_each(|message| match message {
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

    fn checksummed(body: &str) -> String {
        format!("${body}*{:02X}\r\n", body.bytes().fold(0u8, |sum, byte| sum ^ byte))
    }

    struct Receiver {
        written: Vec<String>,
        pending: VecDeque<u8>,
        clock_ms: u64,
        board: &'static str,
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
            let reply = match (self.answers, text.as_str()) {
                (false, _) => String::new(),
                (true, PORT_QUERY) => checksummed("PASHR,PRT,B,9"),
                (true, BOARD_QUERY) => checksummed(&format!("PASHR,RID,{},30,D1A0,ZZ", self.board)),
                (true, command) if command.starts_with("$PASHS,POS,AVG") => checksummed("PASHR,RECEIPT,POS,AVG,STARTED,INTERVAL,100,114502.56,28.12.2011"),
                (true, _) => checksummed("PASHR,ACK"),
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
        Receiver { written: Vec::new(), pending: VecDeque::new(), clock_ms: 0, board: "MB2", answers: true, bauds: Vec::new() }
    }

    fn survey() -> BasePlan {
        BasePlan::SurveyIn(SurveySpec { accuracy_m: 2.0, accuracy_driver_units: 20000.0, duration_s: 100 })
    }

    fn on_port_b(templates: &[&str]) -> Vec<String> {
        templates.iter().map(|template| template.replace("{port}", "B")).collect()
    }

    fn flags(event: &Event) -> (u32, u8, bool) {
        match event {
            Event::SurveyIn(status) => (status.duration_s, status.flags, status.latitude.is_finite()),
            other => panic!("survey status expected, got {other:?}"),
        }
    }

    const POSITION: &str = "PASHR,POS,2,10,125410.00,5525.8138702,N,03833.9587380,E,131.555,1.0,0.0,0.007,-0.001,2.0,1.0,1.7,1.0,";
    const NO_POSITION: &str = "PASHR,POS,0,3,125410.00,,,,,,,,,,99.9,99.9,99.9,99.9,";
    const FINISHED: &str = "PASHR,RECEIPT,POS,AVG,100,FINISHED,114642.81,28.12.2011,5542.5178481,N,03739.2954994,E,176.334,OK,CONTINUOUS,100.20";

    #[test]
    fn an_mb_two_is_configured_like_gps_driver_ashtech_and_waits_for_a_position() {
        let mut base = AshtechBase::new(receiver(), survey());
        assert!(base.configure());
        assert_eq!(base.transport.bauds, [ASHTECH_BAUD]);
        let expected: Vec<String> = [PORT_QUERY, BOARD_QUERY].into_iter().chain(SETUP).map(String::from).chain(on_port_b(&MESSAGES)).collect();
        assert_eq!(base.transport.written, expected);
        assert_eq!(base.take_events().iter().map(flags).collect::<Vec<_>>(), [(0, 0b10, false)]);

        base.transport.pending.extend(checksummed(NO_POSITION).bytes());
        assert_eq!(base.receive(100), Some(false), "no correction output before the receiver has a position");
        assert_eq!(base.transport.written.len(), expected.len());
    }

    #[test]
    fn a_surveying_base_averages_then_streams_rtcm() {
        let mut base = AshtechBase::new(receiver(), survey());
        base.configure();
        base.take_events();
        let configured = base.transport.written.len();

        base.transport.pending.extend(checksummed(POSITION).bytes());
        assert_eq!(base.receive(100), Some(true));
        let started: Vec<String> = ["$PASHS,POS,AVG,100\r\n"].into_iter().chain(SURVEY_BASE).map(String::from).collect();
        assert_eq!(base.transport.written[configured..], started);
        assert_eq!(base.take_events().iter().map(flags).collect::<Vec<_>>(), [(0, 0b10, false)]);

        base.transport.clock_ms += 3_000;
        base.transport.pending.extend(checksummed(POSITION).bytes());
        base.receive(100);
        assert_eq!(base.take_events().iter().map(flags).collect::<Vec<_>>(), [(3, 0b10, false)], "one tick per new second, the survey is not restarted");

        base.transport.pending.extend(checksummed(FINISHED).bytes());
        base.receive(100);
        let Event::SurveyIn(done) = base.take_events().remove(0) else { panic!("finished survey expected") };
        assert_eq!(done.flags, 0b01);
        assert!((done.latitude - 55.708_630_801_666).abs() < 1e-9 && (done.longitude - 37.654_924_99).abs() < 1e-9, "{done:?}");
        assert_eq!(done.altitude_m, 176.334);
        assert_eq!(base.transport.written[configured + started.len()..], on_port_b(&RTCM_OUTPUT));

        base.transport.pending.extend([b'x', 0xd3, 0x00, 0x01, 0x42, 9, 9, 9]);
        assert_eq!(base.receive(100), Some(true));
        assert_eq!(base.take_events(), [Event::Rtcm(vec![0xd3, 0x00, 0x01, 0x42, 9, 9, 9])]);
    }

    #[test]
    fn a_failed_survey_reports_invalid_and_sends_no_corrections() {
        let mut base = AshtechBase::new(receiver(), survey());
        base.configure();
        base.transport.pending.extend(checksummed(POSITION).bytes());
        base.receive(100);
        base.take_events();
        let written = base.transport.written.len();
        base.transport.pending.extend(checksummed("PASHR,RECEIPT,POS,AVG,100,FINISHED,124628.01,28.12.2011,ERR").bytes());
        base.receive(100);
        assert_eq!(base.take_events().iter().map(flags).collect::<Vec<_>>(), [(0, 0, false)]);
        assert_eq!(base.transport.written.len(), written);
    }

    #[test]
    fn a_fixed_base_is_told_its_position_in_ddmm_once_it_has_a_fix() {
        let fixed = BasePlan::Fixed(BasePosition { latitude: -47.5, longitude: 8.25, altitude_m: 488.0, accuracy_mm: 1500.0 });
        let mut base = AshtechBase::new(receiver(), fixed);
        base.configure();
        base.take_events();
        let configured = base.transport.written.len();
        base.transport.pending.extend(checksummed(POSITION).bytes());
        base.receive(100);
        assert_eq!(base.transport.written[configured], "$PASHS,POS,4730.00000000,S,815.00000000,E,488.00000,PC1");
        assert_eq!(base.transport.written[configured + 1..], on_port_b(&RTCM_OUTPUT));
        let Event::SurveyIn(done) = base.take_events().remove(0) else { panic!("fixed position status expected") };
        assert_eq!((done.flags, done.latitude, done.longitude, done.altitude_m), (0b01, -47.5, 8.25, 488.0));
    }

    #[test]
    fn another_board_is_configured_but_never_becomes_a_base() {
        let mut base = AshtechBase::new(Receiver { board: "BD9", ..receiver() }, survey());
        assert!(base.configure());
        assert!(base.take_events().is_empty());
        let written = base.transport.written.len();
        base.transport.pending.extend(checksummed(POSITION).bytes());
        assert_eq!(base.receive(100), Some(false));
        assert_eq!(base.transport.written.len(), written);
    }

    #[test]
    fn gps_satellites_in_view_are_counted_like_the_driver() {
        let mut base = AshtechBase::new(receiver(), survey());
        base.configure();
        base.take_events();
        base.transport.pending.extend(checksummed("GLGSV,1,1,02,70,40,100,30,71,20,300,").bytes());
        base.transport.pending.extend(checksummed("GPGSV,2,1,06,02,02,213,,03,-3,000,,11,00,121,,14,13,172,05").bytes());
        base.transport.pending.extend(checksummed("GPGSV,2,2,06,20,45,090,41,21,10,270,").bytes());
        assert_eq!(base.receive(100), Some(true));
        let events = base.take_events();
        let [Event::Satellites { count, list }] = events.as_slice() else { panic!("one satellite report expected, got {events:?}") };
        assert_eq!(*count, 6);
        assert_eq!(list.iter().map(|satellite| (satellite.svid, satellite.used)).collect::<Vec<_>>(), [(2, false), (3, false), (11, false), (14, true), (20, true), (21, false)]);
        assert_eq!(list[1].elevation_deg(), -3);
    }

    #[test]
    fn a_receiver_that_never_answers_is_asked_for_its_port_twice() {
        let mut base = AshtechBase::new(Receiver { answers: false, ..receiver() }, survey());
        assert!(!base.configure());
        assert_eq!(base.transport.written, [PORT_QUERY, PORT_QUERY]);
    }
}
