use crate::gpsrtk::BasePlan;
use crate::ubxbase::{BaseDriver, Decoder, Event, Frame, Transport};

const SBF_BAUD: u32 = 115200;
const SBF_ACK_TIMEOUT_MS: u64 = 1000;
const FORCE_INPUT: &str = "SSSSSSSSSS\n";
const ACK_MARKER: &str = "$R: ";
const PROMPT: char = '>';
const PORT_NAME_LENGTH: usize = 4;
const DATA_IO_RETRIES: usize = 5;
const SURVEY_IN: &str = "setDataInOut, USB1, Auto, RTCMv3\nsetPVTMode, Static, All, auto\n";
const STATIC_DYNAMICS: &str = "setReceiverDynamics, Low, Static\n";
const STATIC_PVT: &str = "setPVTMode, Static, , Geodetic1\n";

pub struct SbfBase<T: Transport> {
    pub transport: T,
    decoder: Decoder,
    plan: BasePlan,
    configured: bool,
    transport_lost: bool,
    events: Vec<Event>,
}

impl<T: Transport> SbfBase<T> {
    pub fn new(transport: T, plan: BasePlan) -> SbfBase<T> {
        SbfBase { transport, decoder: Decoder::default(), plan, configured: false, transport_lost: false, events: Vec::new() }
    }

    fn read_for(&mut self, timeout_ms: u64, until: impl Fn(&str) -> bool) -> Option<String> {
        let deadline = self.transport.now_ms() + timeout_ms;
        std::iter::from_fn(|| {
            let left = deadline.saturating_sub(self.transport.now_ms());
            (left > 0).then(|| self.transport.read(left))
        })
        .map_while(|read| {
            if read.is_none() {
                self.transport_lost = true;
            }
            read
        })
        .scan(String::new(), |heard, bytes| {
            heard.push_str(&String::from_utf8_lossy(&bytes));
            Some(heard.clone())
        })
        .find(|heard| until(heard))
    }

    fn command(&mut self, text: &str) -> bool {
        self.transport.write(text.as_bytes()) && self.read_for(SBF_ACK_TIMEOUT_MS, |heard| heard.contains(ACK_MARKER)).is_some()
    }

    fn port_name(&mut self) -> Option<String> {
        self.transport.write(b"\n\r");
        let heard = self.read_for(SBF_ACK_TIMEOUT_MS, |heard| heard.contains(PROMPT))?;
        Some(heard.chars().take(PORT_NAME_LENGTH).collect())
    }

    fn rtcm_commands(&self) -> Vec<String> {
        match self.plan {
            BasePlan::Fixed(position) => vec![
                format!("setStaticPosGeodetic, Geodetic1, {:.6}, {:.6}, {:.6}\n", position.latitude, position.longitude, position.altitude_m as f64),
                format!("setAntennaOffset, Main, {:.6}, {:.6}, {:.6}\n", 0.0, 0.0, 0.0),
                STATIC_DYNAMICS.to_string(),
                STATIC_PVT.to_string(),
            ],
            BasePlan::SurveyIn(_) => vec![SURVEY_IN.to_string()],
        }
    }
}

impl<T: Transport> BaseDriver for SbfBase<T> {
    fn configure(&mut self) -> bool {
        self.configured = false;
        self.transport.set_baud(SBF_BAUD);
        self.transport.write(FORCE_INPUT.as_bytes());
        let silence: Vec<String> = (1..=2).map(|i| format!("setDataInOut,COM{i},,-RTCMv3\n")).chain((1..=4).map(|i| format!("setDataInOut,USB{i},,-RTCMv3\n"))).collect();
        silence.iter().for_each(|text| {
            self.command(text);
        });
        let Some(port) = self.port_name() else { return false };
        if !self.command(&format!("setSBFOutput, Stream1, {port}, none, off\n")) {
            return false;
        }
        let usb = port.starts_with("USB1") || port.starts_with("USB2");
        if !usb && !self.command(&format!("setCOMSettings, {port}, baud{SBF_BAUD}\n")) {
            return false;
        }
        let data_io = format!("setDataInOut, {port}, Auto, SBF\n");
        if !self.command(&data_io) || !(0..DATA_IO_RETRIES).any(|_| self.command(&data_io)) {
            return false;
        }
        self.rtcm_commands().iter().for_each(|text| {
            self.command(text);
        });
        self.decoder.reset();
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
        let rtcm: Vec<Event> = self
            .decoder
            .feed(&bytes)
            .into_iter()
            .filter_map(|frame| match frame {
                Frame::Rtcm(message) => Some(Event::Rtcm(message)),
                Frame::Ubx { .. } => None,
            })
            .collect();
        let handled = !rtcm.is_empty();
        self.events.extend(rtcm);
        Some(handled)
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
    use crate::gpsrtk::{BasePosition, SurveySpec};
    use std::collections::VecDeque;

    struct Mosaic {
        written: Vec<String>,
        pending: VecDeque<u8>,
        clock_ms: u64,
        port: &'static str,
        prompts: bool,
        bauds: Vec<u32>,
    }

    impl Transport for Mosaic {
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
            let reply = match text.as_str() {
                "\n\r" if self.prompts => format!("{}>", self.port),
                t if t.starts_with("set") => format!("$R: {}", t.trim()),
                _ => String::new(),
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

    fn mosaic(port: &'static str) -> Mosaic {
        Mosaic { written: Vec::new(), pending: VecDeque::new(), clock_ms: 0, port, prompts: true, bauds: Vec::new() }
    }

    #[test]
    fn a_usb_receiver_is_set_up_as_a_surveying_base_without_touching_its_baud() {
        let survey = BasePlan::SurveyIn(SurveySpec { accuracy_m: 2.0, accuracy_driver_units: 20000.0, duration_s: 180 });
        let mut base = SbfBase::new(mosaic("USB1"), survey);
        assert!(base.configure());
        let written = &base.transport.written;
        assert_eq!(written[0], FORCE_INPUT);
        assert_eq!(written[1..7], ["setDataInOut,COM1,,-RTCMv3\n", "setDataInOut,COM2,,-RTCMv3\n", "setDataInOut,USB1,,-RTCMv3\n", "setDataInOut,USB2,,-RTCMv3\n", "setDataInOut,USB3,,-RTCMv3\n", "setDataInOut,USB4,,-RTCMv3\n"]);
        assert_eq!(written[7..], ["\n\r", "setSBFOutput, Stream1, USB1, none, off\n", "setDataInOut, USB1, Auto, SBF\n", "setDataInOut, USB1, Auto, SBF\n", SURVEY_IN]);
        assert_eq!(base.transport.bauds, [SBF_BAUD]);
    }

    #[test]
    fn a_serial_receiver_gets_its_baud_and_a_fixed_position() {
        let fixed = BasePlan::Fixed(BasePosition { latitude: 47.3977419, longitude: 8.5455938, altitude_m: 488.0, accuracy_mm: 1500.0 });
        let mut base = SbfBase::new(mosaic("COM1"), fixed);
        assert!(base.configure());
        let written = &base.transport.written;
        assert!(written.contains(&"setCOMSettings, COM1, baud115200\n".to_string()));
        assert_eq!(written[written.len() - 4..], ["setStaticPosGeodetic, Geodetic1, 47.397742, 8.545594, 488.000000\n", "setAntennaOffset, Main, 0.000000, 0.000000, 0.000000\n", STATIC_DYNAMICS, STATIC_PVT]);
        base.transport.pending.extend([b'$', b'@', 1, 2, 0xd3, 0x00, 0x01, 0x42, 9, 9, 9]);
        assert_eq!(base.receive(100), Some(true));
        assert_eq!(base.take_events(), [Event::Rtcm(vec![0xd3, 0x00, 0x01, 0x42, 9, 9, 9])]);
    }

    #[test]
    fn a_receiver_that_never_shows_its_prompt_is_not_configured() {
        let mut base = SbfBase::new(Mosaic { prompts: false, ..mosaic("USB1") }, BasePlan::SurveyIn(SurveySpec { accuracy_m: 2.0, accuracy_driver_units: 20000.0, duration_s: 180 }));
        assert!(!base.configure());
        assert_eq!(base.transport.written.last().map(String::as_str), Some("\n\r"));
    }
}
