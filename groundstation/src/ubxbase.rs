use crate::gpsrtk::{BasePlan, Satellite};

const SYNC: [u8; 2] = [0xb5, 0x62];
const RTCM3_PREAMBLE: u8 = 0xd3;
const MAX_UBX_PAYLOAD: usize = 1240;
const MAX_RTCM_PAYLOAD: usize = 1023;

const CLASS_NAV: u8 = 0x01;
const CLASS_ACK: u8 = 0x05;
const CLASS_CFG: u8 = 0x06;
const CLASS_MON: u8 = 0x0a;
const CLASS_RTCM3: u8 = 0xf5;

const ACK_NAK: u8 = 0x00;
const ACK_ACK: u8 = 0x01;
const CFG_PRT: (u8, u8) = (CLASS_CFG, 0x00);
const CFG_MSG: (u8, u8) = (CLASS_CFG, 0x01);
const CFG_RATE: (u8, u8) = (CLASS_CFG, 0x08);
const CFG_NAV5: (u8, u8) = (CLASS_CFG, 0x24);
const CFG_TMODE3: (u8, u8) = (CLASS_CFG, 0x71);
const CFG_VALSET: (u8, u8) = (CLASS_CFG, 0x8a);
const MON_VER: (u8, u8) = (CLASS_MON, 0x04);
const MON_HW: (u8, u8) = (CLASS_MON, 0x09);
const NAV_POSLLH: (u8, u8) = (CLASS_NAV, 0x02);
const NAV_STATUS: (u8, u8) = (CLASS_NAV, 0x03);
const NAV_DOP: (u8, u8) = (CLASS_NAV, 0x04);
const NAV_SOL: (u8, u8) = (CLASS_NAV, 0x06);
const NAV_PVT: (u8, u8) = (CLASS_NAV, 0x07);
const NAV_VELNED: (u8, u8) = (CLASS_NAV, 0x12);
const NAV_TIMEUTC: (u8, u8) = (CLASS_NAV, 0x21);
const NAV_SVINFO: (u8, u8) = (CLASS_NAV, 0x30);
const NAV_SAT: (u8, u8) = (CLASS_NAV, 0x35);
const NAV_SVIN: (u8, u8) = (CLASS_NAV, 0x3b);
const RTCM3_MESSAGES: [(u8, u8); 6] = [(CLASS_RTCM3, 0x05), (CLASS_RTCM3, 0x4d), (CLASS_RTCM3, 0x57), (CLASS_RTCM3, 0xe6), (CLASS_RTCM3, 0x61), (CLASS_RTCM3, 0x7f)];

const CONFIG_TIMEOUT_MS: u64 = 250;
const FIRST_VALSET_TIMEOUT_MS: u64 = 2000;
const FLUSH_MS: u64 = 20;
const PROBE_BAUD_RATES: [u32; 7] = [38400, 57600, 9600, 115200, 230400, 460800, 921600];
const BAUD_M8_AND_NEWER: u32 = 115200;
const BAUD_PRE_M8: u32 = 38400;
const STATIONARY_DYNAMIC_MODEL: u8 = 2;
const MAX_SATELLITES: usize = 20;

const PRT_PORT_UART1: u8 = 0x01;
const PRT_PORT_USB: u8 = 0x03;
const PRT_MODE_8N1: u32 = 0x0000_08d0;
const PROTO_UBX: u16 = 1;
const PROTO_RTCM: u16 = 1 << 5;
const RATE_MEAS_MS: u16 = 200;
const RATE_MEAS_BASE_MS: u16 = 1000;
const NAV5_MASK: u16 = 0x0005;
const NAV5_FIX_3D: u8 = 2;
const LAYER_RAM: u8 = 1;

const KEY_I2CINPROT_UBX: u32 = 0x1071_0001;
const KEY_I2CINPROT_NMEA: u32 = 0x1071_0002;
const KEY_I2CINPROT_RTCM3X: u32 = 0x1071_0004;
const KEY_I2COUTPROT_UBX: u32 = 0x1072_0001;
const KEY_I2COUTPROT_NMEA: u32 = 0x1072_0002;
const KEY_I2COUTPROT_RTCM3X: u32 = 0x1072_0004;
const KEY_UART1_BAUDRATE: u32 = 0x4052_0001;
const KEY_UART1_STOPBITS: u32 = 0x2052_0002;
const KEY_UART1_DATABITS: u32 = 0x2052_0003;
const KEY_UART1_PARITY: u32 = 0x2052_0004;
const KEY_UART1INPROT_UBX: u32 = 0x1073_0001;
const KEY_UART1INPROT_NMEA: u32 = 0x1073_0002;
const KEY_UART1INPROT_RTCM3X: u32 = 0x1073_0004;
const KEY_UART1OUTPROT_UBX: u32 = 0x1074_0001;
const KEY_UART1OUTPROT_NMEA: u32 = 0x1074_0002;
const KEY_UART1OUTPROT_RTCM3X: u32 = 0x1074_0004;
const KEY_USBINPROT_UBX: u32 = 0x1077_0001;
const KEY_USBINPROT_NMEA: u32 = 0x1077_0002;
const KEY_USBINPROT_RTCM3X: u32 = 0x1077_0004;
const KEY_USBOUTPROT_UBX: u32 = 0x1078_0001;
const KEY_USBOUTPROT_NMEA: u32 = 0x1078_0002;
const KEY_USBOUTPROT_RTCM3X: u32 = 0x1078_0004;
const KEY_NAVHPG_DGNSSMODE: u32 = 0x2014_0011;
const KEY_NAVSPG_FIXMODE: u32 = 0x2011_0011;
const KEY_NAVSPG_UTCSTANDARD: u32 = 0x2011_001c;
const KEY_NAVSPG_DYNMODEL: u32 = 0x2011_0021;
const KEY_ODO_USE_ODO: u32 = 0x1022_0001;
const KEY_ODO_USE_COG: u32 = 0x1022_0002;
const KEY_ODO_OUTLPVEL: u32 = 0x1022_0003;
const KEY_ODO_OUTLPCOG: u32 = 0x1022_0004;
const KEY_ITFM_ENABLE: u32 = 0x1041_000d;
const KEY_RATE_MEAS: u32 = 0x3021_0001;
const KEY_RATE_NAV: u32 = 0x3021_0002;
const KEY_RATE_TIMEREF: u32 = 0x2021_0003;
const KEY_TMODE_MODE: u32 = 0x2003_0001;
const KEY_TMODE_POS_TYPE: u32 = 0x2003_0002;
const KEY_TMODE_LAT: u32 = 0x4003_0009;
const KEY_TMODE_LON: u32 = 0x4003_000a;
const KEY_TMODE_HEIGHT: u32 = 0x4003_000b;
const KEY_TMODE_LAT_HP: u32 = 0x2003_000c;
const KEY_TMODE_LON_HP: u32 = 0x2003_000d;
const KEY_TMODE_HEIGHT_HP: u32 = 0x2003_000e;
const KEY_TMODE_FIXED_POS_ACC: u32 = 0x4003_000f;
const KEY_TMODE_SVIN_MIN_DUR: u32 = 0x4003_0010;
const KEY_TMODE_SVIN_ACC_LIMIT: u32 = 0x4003_0011;
const KEY_MSGOUT_MON_RF: u32 = 0x2091_0359;
const KEY_MSGOUT_NAV_SVIN: u32 = 0x2091_0088;
const KEY_MSGOUT_NAV_SAT: u32 = 0x2091_0015;
const KEY_MSGOUT_NAV_STATUS: u32 = 0x2091_001a;
const KEY_MSGOUT_NAV_DOP: u32 = 0x2091_0038;
const KEY_MSGOUT_NAV_PVT: u32 = 0x2091_0006;
const KEY_MSGOUT_NAV_HPPOSLLH: u32 = 0x2091_0033;
const KEY_MSGOUT_NAV_RELPOSNED: u32 = 0x2091_008d;
const KEY_MSGOUT_RXM_RTCM: u32 = 0x2091_0268;
const KEY_MSGOUT_RTCM: [u32; 6] = [0x2091_02bd, 0x2091_02cc, 0x2091_02d1, 0x2091_0303, 0x2091_0318, 0x2091_02d6];
const RTCM_RATES: [u8; 6] = [5, 1, 1, 1, 1, 1];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Board {
    Unknown,
    UBlox5,
    UBlox6,
    UBlox7,
    UBlox8,
    UBlox9,
    UBlox9F9pL1L2,
    UBlox9F9pL1L5,
    UBlox10,
}

impl Board {
    fn f9p(self) -> bool {
        matches!(self, Board::UBlox9F9pL1L2 | Board::UBlox9F9pL1L5)
    }

    fn generation9(self) -> bool {
        matches!(self, Board::UBlox9 | Board::UBlox9F9pL1L2 | Board::UBlox9F9pL1L5)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurveyStatus {
    pub duration_s: u32,
    pub mean_accuracy_mm: u32,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f32,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    SurveyIn(SurveyStatus),
    Satellites { count: u8, list: Vec<Satellite> },
    Rtcm(Vec<u8>),
}

pub trait Transport {
    fn read(&mut self, timeout_ms: u64) -> Option<Vec<u8>>;
    fn write(&mut self, bytes: &[u8]) -> bool;
    fn set_baud(&mut self, baud: u32) -> bool;
    fn now_ms(&self) -> u64;
}

#[derive(Debug, Clone, PartialEq)]
enum Frame {
    Ubx { class: u8, id: u8, payload: Vec<u8> },
    Rtcm(Vec<u8>),
}

#[derive(Debug, Default)]
struct Decoder {
    buffer: Vec<u8>,
}

fn checksum(bytes: &[u8]) -> [u8; 2] {
    let (a, b) = bytes.iter().fold((0u8, 0u8), |(a, b), byte| {
        let a = a.wrapping_add(*byte);
        (a, b.wrapping_add(a))
    });
    [a, b]
}

pub fn frame(class: u8, id: u8, payload: &[u8]) -> Vec<u8> {
    let length = (payload.len() as u16).to_le_bytes();
    let body: Vec<u8> = [class, id, length[0], length[1]].into_iter().chain(payload.iter().copied()).collect();
    SYNC.iter().copied().chain(body.iter().copied()).chain(checksum(&body)).collect()
}

enum Scan {
    Frame(Frame, usize),
    Skip(usize),
    Incomplete,
}

fn scan(buffer: &[u8]) -> Scan {
    match buffer {
        [] => Scan::Incomplete,
        [0xb5] => Scan::Incomplete,
        [0xb5, 0x62, rest @ ..] => match rest {
            [class, id, low, high, ..] => {
                let length = u16::from_le_bytes([*low, *high]) as usize;
                let total = 8 + length;
                match (length > MAX_UBX_PAYLOAD, buffer.len() >= total) {
                    (true, _) => Scan::Skip(1),
                    (false, false) => Scan::Incomplete,
                    (false, true) => match checksum(&buffer[2..6 + length]) == [buffer[6 + length], buffer[7 + length]] {
                        true => Scan::Frame(Frame::Ubx { class: *class, id: *id, payload: buffer[6..6 + length].to_vec() }, total),
                        false => Scan::Skip(1),
                    },
                }
            }
            _ => Scan::Incomplete,
        },
        [RTCM3_PREAMBLE, rest @ ..] => match rest {
            [high, low, ..] => {
                let length = (((*high & 0x03) as usize) << 8) | *low as usize;
                let total = 3 + length + 3;
                match (length > MAX_RTCM_PAYLOAD, buffer.len() >= total) {
                    (true, _) => Scan::Skip(1),
                    (false, false) => Scan::Incomplete,
                    (false, true) => Scan::Frame(Frame::Rtcm(buffer[..total].to_vec()), total),
                }
            }
            _ => Scan::Incomplete,
        },
        _ => Scan::Skip(buffer.iter().position(|byte| *byte == SYNC[0] || *byte == RTCM3_PREAMBLE).unwrap_or(buffer.len()).max(1)),
    }
}

impl Decoder {
    fn reset(&mut self) {
        self.buffer.clear();
    }

    fn feed(&mut self, bytes: &[u8]) -> Vec<Frame> {
        self.buffer.extend_from_slice(bytes);
        let (frames, consumed) = std::iter::successors(Some((None, 0usize)), |(_, at)| match scan(&self.buffer[*at..]) {
            Scan::Frame(frame, used) => Some((Some(frame), at + used)),
            Scan::Skip(used) => Some((None, at + used)),
            Scan::Incomplete => None,
        })
        .fold((Vec::new(), 0), |(frames, _), (frame, at)| ([frames, frame.into_iter().collect()].concat(), at));
        self.buffer.drain(..consumed);
        frames
    }
}

pub fn valset(values: &[(u32, Value)]) -> Vec<u8> {
    [0u8, LAYER_RAM, 0, 0].into_iter().chain(values.iter().flat_map(|(key, value)| key.to_le_bytes().into_iter().chain(value.bytes()))).collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    U8(u8),
    I8(i8),
    U16(u16),
    U32(u32),
    I32(i32),
}

impl Value {
    fn bytes(self) -> Vec<u8> {
        match self {
            Value::U8(v) => vec![v],
            Value::I8(v) => v.to_le_bytes().to_vec(),
            Value::U16(v) => v.to_le_bytes().to_vec(),
            Value::U32(v) => v.to_le_bytes().to_vec(),
            Value::I32(v) => v.to_le_bytes().to_vec(),
        }
    }
}

fn cfg_prt(baud: u32) -> Vec<u8> {
    [PRT_PORT_UART1, PRT_PORT_USB]
        .iter()
        .flat_map(|port| {
            [*port, 0, 0, 0]
                .into_iter()
                .chain(PRT_MODE_8N1.to_le_bytes())
                .chain(baud.to_le_bytes())
                .chain(PROTO_UBX.to_le_bytes())
                .chain((PROTO_UBX | PROTO_RTCM).to_le_bytes())
                .chain([0, 0, 0, 0])
        })
        .collect()
}

fn cfg_msg((class, id): (u8, u8), rate: u8) -> Vec<u8> {
    vec![class, id, rate]
}

fn cfg_rate(meas_ms: u16) -> Vec<u8> {
    meas_ms.to_le_bytes().into_iter().chain(1u16.to_le_bytes()).chain(0u16.to_le_bytes()).collect()
}

fn cfg_nav5() -> Vec<u8> {
    NAV5_MASK.to_le_bytes().into_iter().chain([STATIONARY_DYNAMIC_MODEL, NAV5_FIX_3D]).chain(std::iter::repeat_n(0, 32)).collect()
}

fn high_precision(value: f64, scale: f64) -> (i32, i8) {
    let whole = (value * scale) as i64;
    ((whole / 100) as i32, (whole % 100) as i8)
}

fn cfg_tmode3(plan: Option<BasePlan>) -> Vec<u8> {
    let zeros = |count: usize| std::iter::repeat_n(0u8, count);
    match plan {
        None => zeros(40).collect(),
        Some(BasePlan::SurveyIn(spec)) => [0u8, 0]
            .into_iter()
            .chain(1u16.to_le_bytes())
            .chain(zeros(20))
            .chain(spec.duration_s.to_le_bytes())
            .chain((spec.accuracy_driver_units as u32).to_le_bytes())
            .chain(zeros(8))
            .collect(),
        Some(BasePlan::Fixed(position)) => {
            let (lat, lat_hp) = high_precision(position.latitude, 1e9);
            let (lon, lon_hp) = high_precision(position.longitude, 1e9);
            let (alt, alt_hp) = high_precision(position.altitude_m as f64, 1e4);
            [0u8, 0]
                .into_iter()
                .chain((2u16 | 1 << 8).to_le_bytes())
                .chain(lat.to_le_bytes())
                .chain(lon.to_le_bytes())
                .chain(alt.to_le_bytes())
                .chain([lat_hp as u8, lon_hp as u8, alt_hp as u8, 0])
                .chain(((position.accuracy_mm * 10.0) as u32).to_le_bytes())
                .chain(zeros(16))
                .collect()
        }
    }
}

pub fn ecef_to_lla(x: f64, y: f64, z: f64) -> (f64, f64, f32) {
    let a = 6_378_137.0_f64;
    let e = 8.181_919_084_262_2e-2_f64;
    let (asq, esq) = (a * a, e * e);
    let b = (asq * (1.0 - esq)).sqrt();
    let ep = ((asq - b * b) / (b * b)).sqrt();
    let p = (x * x + y * y).sqrt();
    let th = (a * z).atan2(b * p);
    let longitude = y.atan2(x);
    let latitude = (z + ep * ep * b * th.sin().powi(3)).atan2(p - esq * a * th.cos().powi(3));
    let n = a / (1.0 - esq * latitude.sin().powi(2)).sqrt();
    let altitude = (p / latitude.cos() - n) as f32;
    (latitude.to_degrees(), longitude.to_degrees(), altitude)
}

fn le_u32(payload: &[u8], at: usize) -> u32 {
    payload.get(at..at + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn le_i32(payload: &[u8], at: usize) -> i32 {
    le_u32(payload, at) as i32
}

fn byte(payload: &[u8], at: usize) -> u8 {
    payload.get(at).copied().unwrap_or(0)
}

pub fn survey_status(payload: &[u8]) -> SurveyStatus {
    let axis = |at: usize, hp_at: usize| (le_i32(payload, at) as f64 + (byte(payload, hp_at) as i8) as f64 * 0.01) * 0.01;
    let (latitude, longitude, altitude_m) = ecef_to_lla(axis(12, 24), axis(16, 25), axis(20, 26));
    SurveyStatus {
        duration_s: le_u32(payload, 8),
        mean_accuracy_mm: le_u32(payload, 28) / 10,
        latitude,
        longitude,
        altitude_m,
        flags: (byte(payload, 36) & 1) | ((byte(payload, 37) & 1) << 1),
    }
}

fn sat_svid(gnss: u8, sv: u8) -> u8 {
    match (gnss, sv) {
        (0, 1..=32) => sv,
        (1, 120..=158) => sv,
        (2, 1..=36) => sv + 210,
        (3, 1..=4) => sv + 158,
        (3, 5..=37) => sv + 28,
        (4, 1..=10) => sv + 172,
        (5, 1..=10) => sv + 192,
        (6, 1..=32) => sv + 64,
        _ => 255,
    }
}

fn azimuth_raw(degrees: i16) -> u8 {
    (degrees as f32 * 255.0 / 360.0) as u8
}

pub fn nav_sat(payload: &[u8]) -> (u8, Vec<Satellite>) {
    let count = byte(payload, 5).min(MAX_SATELLITES as u8);
    let list = payload
        .get(8..)
        .unwrap_or_default()
        .chunks_exact(12)
        .take(count as usize)
        .map(|sv| {
            let svid = sat_svid(sv[0], sv[1]);
            Satellite { svid, used: sv[8] & 0x01 != 0, elevation_raw: sv[3], azimuth_raw: azimuth_raw(i16::from_le_bytes([sv[4], sv[5]])), snr_db: sv[2], prn: svid }
        })
        .collect();
    (count, list)
}

pub fn nav_svinfo(payload: &[u8]) -> (u8, Vec<Satellite>) {
    let count = byte(payload, 4).min(MAX_SATELLITES as u8);
    let list = payload
        .get(8..)
        .unwrap_or_default()
        .chunks_exact(12)
        .take(count as usize)
        .map(|ch| Satellite { svid: ch[1], used: (ch[2] >> 3) & 0x01 != 0, elevation_raw: ch[5], azimuth_raw: azimuth_raw(i16::from_le_bytes([ch[6], ch[7]])), snr_db: ch[4], prn: ch[1] })
        .collect();
    (count, list)
}

pub fn board_of(payload: &[u8]) -> Board {
    let hardware = payload.get(30..40).map(|b| String::from_utf8_lossy(b).trim_end_matches('\0').to_string()).unwrap_or_default();
    let extensions: Vec<String> = payload.get(40..).unwrap_or_default().chunks(30).map(|b| String::from_utf8_lossy(b).trim_end_matches('\0').to_string()).collect();
    let base = match hardware.as_str() {
        "00040005" => Board::UBlox5,
        "00040007" => Board::UBlox6,
        "00070000" => Board::UBlox7,
        "00080000" => Board::UBlox8,
        "00190000" => Board::UBlox9,
        "000A0000" => Board::UBlox10,
        _ => Board::Unknown,
    };
    extensions.iter().fold(base, |board, extension| match board {
        Board::UBlox9 if extension.starts_with("FWVER=") && extension.contains("HPGL1L5") => Board::UBlox9F9pL1L5,
        Board::UBlox9 if extension.starts_with("MOD=") && extension.contains("F9P") => Board::UBlox9F9pL1L2,
        other => other,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ack {
    Idle,
    Waiting(u8, u8),
    Got,
    Refused,
}

pub struct UbxBase<T: Transport> {
    pub transport: T,
    decoder: Decoder,
    pub board: Board,
    pub protocol_27: bool,
    pub plan: Option<BasePlan>,
    pub configured: bool,
    ack: Ack,
    activate_requested: bool,
    events: Vec<Event>,
}

impl<T: Transport> UbxBase<T> {
    pub fn new(transport: T, plan: BasePlan) -> UbxBase<T> {
        UbxBase {
            transport,
            decoder: Decoder::default(),
            board: Board::Unknown,
            protocol_27: false,
            plan: Some(plan),
            configured: false,
            ack: Ack::Idle,
            activate_requested: false,
            events: Vec::new(),
        }
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    fn send(&mut self, (class, id): (u8, u8), payload: &[u8]) -> bool {
        self.transport.write(&frame(class, id, payload))
    }

    fn handle(&mut self, frame: Frame) -> bool {
        match frame {
            Frame::Rtcm(bytes) => {
                self.events.push(Event::Rtcm(bytes));
                true
            }
            Frame::Ubx { class: CLASS_ACK, id, payload } => {
                let about = (byte(&payload, 0), byte(&payload, 1));
                if self.ack == Ack::Waiting(about.0, about.1) {
                    self.ack = if id == ACK_ACK { Ack::Got } else { Ack::Refused };
                }
                id == ACK_ACK || id == ACK_NAK
            }
            Frame::Ubx { class, id, payload } if (class, id) == MON_VER => {
                self.board = board_of(&payload);
                if self.ack == Ack::Waiting(class, id) {
                    self.ack = Ack::Got;
                }
                true
            }
            Frame::Ubx { class, id, payload } if (class, id) == NAV_SVIN => {
                let status = survey_status(&payload);
                self.events.push(Event::SurveyIn(status));
                if status.flags == 1 {
                    self.activate_requested = true;
                }
                true
            }
            Frame::Ubx { class, id, payload } if (class, id) == NAV_SAT && self.configured => {
                let (count, list) = nav_sat(&payload);
                self.events.push(Event::Satellites { count, list });
                true
            }
            Frame::Ubx { class, id, payload } if (class, id) == NAV_SVINFO && self.configured => {
                let (count, list) = nav_svinfo(&payload);
                self.events.push(Event::Satellites { count, list });
                true
            }
            Frame::Ubx { .. } => false,
        }
    }

    fn receive_once(&mut self, timeout_ms: u64) -> Option<bool> {
        let bytes = self.transport.read(timeout_ms)?;
        let frames = self.decoder.feed(&bytes);
        Some(frames.into_iter().map(|frame| self.handle(frame)).collect::<Vec<bool>>().contains(&true))
    }

    pub fn receive(&mut self, timeout_ms: u64) -> Option<bool> {
        let handled = self.receive_once(timeout_ms)?;
        if self.activate_requested && self.configured {
            self.activate_requested = false;
            self.activate_rtcm_output(true);
        }
        Some(handled)
    }

    fn wait_for_ack(&mut self, message: (u8, u8), timeout_ms: u64) -> bool {
        self.ack = Ack::Waiting(message.0, message.1);
        let deadline = self.transport.now_ms() + timeout_ms;
        while self.ack == Ack::Waiting(message.0, message.1) && self.transport.now_ms() < deadline {
            if self.receive_once(deadline.saturating_sub(self.transport.now_ms())).is_none() {
                break;
            }
        }
        let got = self.ack == Ack::Got;
        self.ack = Ack::Idle;
        got
    }

    fn send_and_ack(&mut self, message: (u8, u8), payload: &[u8], timeout_ms: u64) -> bool {
        self.send(message, payload) && self.wait_for_ack(message, timeout_ms)
    }

    fn send_and_drain(&mut self, message: (u8, u8), payload: &[u8]) -> bool {
        let sent = self.send(message, payload);
        self.wait_for_ack(message, CONFIG_TIMEOUT_MS);
        sent
    }

    fn flush(&mut self) {
        self.decoder.reset();
        let _ = self.transport.read(FLUSH_MS);
        self.decoder.reset();
    }

    fn port(&self, key: u32, rate: u8) -> Vec<(u32, Value)> {
        std::iter::once((key + 1, Value::U8(rate))).chain((self.board != Board::UBlox10).then_some((key + 3, Value::U8(rate)))).collect()
    }

    fn detect_baud(&mut self) -> Option<u32> {
        PROBE_BAUD_RATES.iter().find_map(|test| {
            self.transport.set_baud(*test);
            self.flush();
            let probe = valset(&[
                (KEY_UART1_STOPBITS, Value::U8(1)),
                (KEY_UART1_DATABITS, Value::U8(0)),
                (KEY_UART1_PARITY, Value::U8(0)),
                (KEY_UART1INPROT_UBX, Value::U8(1)),
                (KEY_UART1INPROT_NMEA, Value::U8(0)),
                (KEY_UART1OUTPROT_UBX, Value::U8(1)),
                (KEY_UART1OUTPROT_NMEA, Value::U8(0)),
            ]);
            let desired = match self.send_and_ack(CFG_VALSET, &probe, FIRST_VALSET_TIMEOUT_MS) {
                true => {
                    self.protocol_27 = true;
                    self.send_and_drain(CFG_VALSET, &valset(&[(KEY_UART1_BAUDRATE, Value::U32(BAUD_M8_AND_NEWER))])).then_some(BAUD_M8_AND_NEWER)?
                }
                false => {
                    self.protocol_27 = false;
                    if !self.send_and_ack(CFG_PRT, &cfg_prt(*test), CONFIG_TIMEOUT_MS) {
                        return None;
                    }
                    self.send_and_drain(CFG_PRT, &cfg_prt(BAUD_PRE_M8)).then_some(BAUD_PRE_M8)?
                }
            };
            if desired != *test {
                self.transport.set_baud(desired);
                self.flush();
            }
            Some(desired)
        })
    }

    fn configure_device(&mut self) -> bool {
        let protocols = [
            (KEY_UART1INPROT_RTCM3X, Value::U8(0)),
            (KEY_UART1OUTPROT_RTCM3X, Value::U8(1)),
            (KEY_USBINPROT_UBX, Value::U8(1)),
            (KEY_USBINPROT_RTCM3X, Value::U8(0)),
            (KEY_USBINPROT_NMEA, Value::U8(0)),
            (KEY_USBOUTPROT_UBX, Value::U8(1)),
            (KEY_USBOUTPROT_RTCM3X, Value::U8(1)),
            (KEY_USBOUTPROT_NMEA, Value::U8(0)),
        ];
        if self.board != Board::UBlox10 && !self.send_and_ack(CFG_VALSET, &valset(&protocols), CONFIG_TIMEOUT_MS) {
            return false;
        }
        let navigation = [
            (KEY_NAVSPG_FIXMODE, Value::U8(3)),
            (KEY_NAVSPG_UTCSTANDARD, Value::U8(3)),
            (KEY_NAVSPG_DYNMODEL, Value::U8(STATIONARY_DYNAMIC_MODEL)),
            (KEY_ODO_USE_ODO, Value::U8(0)),
            (KEY_ODO_USE_COG, Value::U8(0)),
            (KEY_ODO_OUTLPVEL, Value::U8(0)),
            (KEY_ODO_OUTLPCOG, Value::U8(0)),
            (KEY_RATE_MEAS, Value::U16(if self.board.f9p() { 100 } else { 125 })),
            (KEY_RATE_NAV, Value::U16(1)),
            (KEY_RATE_TIMEREF, Value::U8(0)),
        ];
        if !self.send_and_ack(CFG_VALSET, &valset(&navigation), CONFIG_TIMEOUT_MS) {
            return false;
        }
        self.send_and_drain(CFG_VALSET, &valset(&[(KEY_NAVHPG_DGNSSMODE, Value::U8(3))]));
        self.send_and_drain(CFG_VALSET, &valset(&[(KEY_ITFM_ENABLE, Value::U8(1))]));
        let high_precision = !matches!(self.board, Board::UBlox10 | Board::UBlox9);
        let outputs: Vec<(u32, Value)> = [
            Some((KEY_MSGOUT_NAV_PVT, 1)),
            high_precision.then_some((KEY_MSGOUT_NAV_HPPOSLLH, 1)),
            high_precision.then_some((KEY_MSGOUT_NAV_RELPOSNED, 0)),
            Some((KEY_MSGOUT_NAV_DOP, 1)),
            Some((KEY_MSGOUT_NAV_SAT, 10)),
            Some((KEY_MSGOUT_NAV_STATUS, 1)),
            Some((KEY_MSGOUT_MON_RF, 1)),
            self.board.generation9().then_some((KEY_MSGOUT_RXM_RTCM, 1)),
        ]
        .into_iter()
        .flatten()
        .flat_map(|(key, rate)| self.port(key, rate))
        .collect();
        if !self.send_and_ack(CFG_VALSET, &valset(&outputs), CONFIG_TIMEOUT_MS) {
            return false;
        }
        let i2c: Vec<(u32, Value)> = [
            Some(KEY_I2CINPROT_UBX),
            Some(KEY_I2CINPROT_NMEA),
            (self.board != Board::UBlox10).then_some(KEY_I2CINPROT_RTCM3X),
            Some(KEY_I2COUTPROT_UBX),
            Some(KEY_I2COUTPROT_NMEA),
            self.board.f9p().then_some(KEY_I2COUTPROT_RTCM3X),
        ]
        .into_iter()
        .flatten()
        .map(|key| (key, Value::U8(0)))
        .collect();
        self.send_and_ack(CFG_VALSET, &valset(&i2c), CONFIG_TIMEOUT_MS)
    }

    fn rate_and_ack(&mut self, message: (u8, u8), rate: u8) -> bool {
        self.send_and_ack(CFG_MSG, &cfg_msg(message, rate), CONFIG_TIMEOUT_MS)
    }

    fn configure_device_pre_27(&mut self) -> bool {
        if !self.send_and_ack(CFG_RATE, &cfg_rate(RATE_MEAS_MS), CONFIG_TIMEOUT_MS) || !self.send_and_ack(CFG_NAV5, &cfg_nav5(), CONFIG_TIMEOUT_MS) {
            return false;
        }
        let nav_pvt = self.rate_and_ack(NAV_PVT, 1);
        let legacy = [(NAV_TIMEUTC, 5), (NAV_POSLLH, 1), (NAV_SOL, 1), (NAV_VELNED, 1)];
        let common = [(NAV_STATUS, 1), (NAV_DOP, 1), (NAV_SVINFO, 5), (MON_HW, 1)];
        legacy.iter().filter(|_| !nav_pvt).chain(common.iter()).all(|(message, rate)| self.rate_and_ack(*message, *rate))
    }

    fn restart_survey_in(&mut self) -> bool {
        match self.protocol_27 {
            true => self.restart_survey_in_27(),
            false => self.restart_survey_in_pre_27(),
        }
    }

    fn restart_survey_in_27(&mut self) -> bool {
        let silenced: Vec<(u32, Value)> = KEY_MSGOUT_RTCM.iter().flat_map(|key| self.port(*key, 0)).collect();
        self.send_and_drain(CFG_VALSET, &valset(&silenced));
        match self.plan {
            Some(BasePlan::SurveyIn(spec)) => {
                let survey: Vec<(u32, Value)> = [(KEY_TMODE_MODE, Value::U8(1)), (KEY_TMODE_SVIN_MIN_DUR, Value::U32(spec.duration_s)), (KEY_TMODE_SVIN_ACC_LIMIT, Value::U32(spec.accuracy_driver_units as u32))]
                    .into_iter()
                    .chain(self.port(KEY_MSGOUT_NAV_SVIN, 5))
                    .collect();
                self.send_and_ack(CFG_VALSET, &valset(&survey), CONFIG_TIMEOUT_MS)
            }
            Some(BasePlan::Fixed(position)) => {
                let (lat, lat_hp) = high_precision(position.latitude, 1e9);
                let (lon, lon_hp) = high_precision(position.longitude, 1e9);
                let (alt, alt_hp) = high_precision(position.altitude_m as f64, 1e4);
                let fixed = [
                    (KEY_TMODE_MODE, Value::U8(2)),
                    (KEY_TMODE_POS_TYPE, Value::U8(1)),
                    (KEY_TMODE_LAT, Value::I32(lat)),
                    (KEY_TMODE_LAT_HP, Value::I8(lat_hp)),
                    (KEY_TMODE_LON, Value::I32(lon)),
                    (KEY_TMODE_LON_HP, Value::I8(lon_hp)),
                    (KEY_TMODE_HEIGHT, Value::I32(alt)),
                    (KEY_TMODE_HEIGHT_HP, Value::I8(alt_hp)),
                    (KEY_TMODE_FIXED_POS_ACC, Value::U32((position.accuracy_mm * 10.0) as u32)),
                ];
                self.send_and_ack(CFG_VALSET, &valset(&fixed), CONFIG_TIMEOUT_MS) && self.activate_rtcm_output(true)
            }
            None => false,
        }
    }

    fn restart_survey_in_pre_27(&mut self) -> bool {
        RTCM3_MESSAGES.iter().for_each(|message| {
            self.send(CFG_MSG, &cfg_msg(*message, 0));
        });
        if !self.send_and_ack(CFG_TMODE3, &cfg_tmode3(None), CONFIG_TIMEOUT_MS) {
            return false;
        }
        match self.plan {
            Some(survey @ BasePlan::SurveyIn(_)) => self.send_and_ack(CFG_TMODE3, &cfg_tmode3(Some(survey)), CONFIG_TIMEOUT_MS) && self.rate_and_ack(NAV_SVIN, 5),
            Some(fixed @ BasePlan::Fixed(_)) => self.send_and_ack(CFG_TMODE3, &cfg_tmode3(Some(fixed)), CONFIG_TIMEOUT_MS) && self.activate_rtcm_output(true),
            None => false,
        }
    }

    fn activate_rtcm_output(&mut self, reduce_update_rate: bool) -> bool {
        match self.protocol_27 {
            true => {
                let outputs: Vec<(u32, Value)> = reduce_update_rate
                    .then_some((KEY_RATE_MEAS, Value::U16(RATE_MEAS_BASE_MS)))
                    .into_iter()
                    .chain(KEY_MSGOUT_RTCM.iter().zip(RTCM_RATES).flat_map(|(key, rate)| self.port(*key, rate)))
                    .chain(self.port(KEY_MSGOUT_NAV_SVIN, 0))
                    .collect();
                self.send_and_ack(CFG_VALSET, &valset(&outputs), CONFIG_TIMEOUT_MS)
            }
            false => {
                if reduce_update_rate && !self.send(CFG_RATE, &cfg_rate(RATE_MEAS_BASE_MS)) {
                    return false;
                }
                self.send(CFG_MSG, &cfg_msg(NAV_SVIN, 0));
                RTCM3_MESSAGES.iter().zip(RTCM_RATES).all(|(message, rate)| self.send(CFG_MSG, &cfg_msg(*message, rate)))
            }
        }
    }

    pub fn configure(&mut self) -> bool {
        self.configured = false;
        if self.detect_baud().is_none() {
            return false;
        }
        if !self.send(MON_VER, &[]) || !self.wait_for_ack(MON_VER, CONFIG_TIMEOUT_MS) {
            return false;
        }
        if self.board == Board::UBlox8 && self.send(CFG_PRT, &cfg_prt(BAUD_M8_AND_NEWER)) {
            self.wait_for_ack(CFG_PRT, CONFIG_TIMEOUT_MS);
            self.transport.set_baud(BAUD_M8_AND_NEWER);
        }
        let device = match self.protocol_27 {
            true => self.configure_device(),
            false => self.configure_device_pre_27(),
        };
        if !device || !self.restart_survey_in() {
            return false;
        }
        self.configured = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpsrtk::{BasePosition, SurveySpec};
    use std::collections::VecDeque;

    struct Receiver {
        baud: u32,
        listens_at: u32,
        protocol_27: bool,
        hardware: &'static str,
        extensions: Vec<&'static str>,
        written: Vec<(u8, u8, Vec<u8>)>,
        bauds: Vec<u32>,
        pending: VecDeque<u8>,
        clock_ms: u64,
        decoder: Decoder,
    }

    impl Receiver {
        fn new(listens_at: u32, protocol_27: bool, hardware: &'static str, extensions: Vec<&'static str>) -> Receiver {
            Receiver { baud: 0, listens_at, protocol_27, hardware, extensions, written: Vec::new(), bauds: Vec::new(), pending: VecDeque::new(), clock_ms: 0, decoder: Decoder::default() }
        }

        fn reply(&mut self, class: u8, id: u8, payload: &[u8]) {
            self.pending.extend(frame(class, id, payload));
        }

        fn mon_ver(&self) -> Vec<u8> {
            let field = |text: &str, size: usize| text.bytes().chain(std::iter::repeat(0)).take(size).collect::<Vec<u8>>();
            [field("ROM CORE", 30), field(self.hardware, 10)].into_iter().chain(self.extensions.iter().map(|e| field(e, 30))).flatten().collect()
        }
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
            let frames = self.decoder.feed(bytes);
            frames.into_iter().for_each(|written| {
                let Frame::Ubx { class, id, payload } = written else { return };
                self.written.push((class, id, payload.clone()));
                if self.baud != self.listens_at {
                    return;
                }
                match (class, id) {
                    (CLASS_CFG, 0x8a) if !self.protocol_27 => self.reply(CLASS_ACK, ACK_NAK, &[class, id]),
                    (CLASS_CFG, _) => {
                        if (class, id) == CFG_VALSET && payload.get(4..8) == Some(&KEY_UART1_BAUDRATE.to_le_bytes()) {
                            self.listens_at = le_u32(&payload, 8);
                        }
                        if (class, id) == CFG_PRT && le_u32(&payload, 8) != self.baud {
                            self.listens_at = le_u32(&payload, 8);
                            return;
                        }
                        self.reply(CLASS_ACK, ACK_ACK, &[class, id]);
                    }
                    (CLASS_MON, 0x04) => {
                        let version = self.mon_ver();
                        self.reply(CLASS_MON, 0x04, &version);
                    }
                    _ => {}
                }
            });
            true
        }

        fn set_baud(&mut self, baud: u32) -> bool {
            self.baud = baud;
            self.bauds.push(baud);
            true
        }

        fn now_ms(&self) -> u64 {
            self.clock_ms
        }
    }

    fn survey() -> BasePlan {
        BasePlan::SurveyIn(SurveySpec { accuracy_m: 2.0, accuracy_driver_units: 20000.0, duration_s: 180 })
    }

    fn keys_in(payload: &[u8]) -> Vec<u32> {
        std::iter::successors(Some(4usize), |at| Some(at + 4 + key_width(le_u32(payload, *at))))
            .take_while(|at| *at + 4 <= payload.len())
            .map(|at| le_u32(payload, at))
            .collect()
    }

    fn key_width(key: u32) -> usize {
        match key >> 28 {
            1 | 2 => 1,
            3 => 2,
            4 => 4,
            _ => 8,
        }
    }

    #[test]
    fn frames_round_trip_and_rtcm_passes_through_between_them() {
        let svin = frame(CLASS_NAV, 0x3b, &[7; 40]);
        let rtcm = [0xd3, 0x00, 0x03, 0x41, 0x42, 0x43, 0x01, 0x02, 0x03];
        let stream: Vec<u8> = [vec![0x00, 0x24], svin.clone(), rtcm.to_vec(), frame(CLASS_ACK, ACK_ACK, &[6, 0x8a])].concat();
        let mut decoder = Decoder::default();
        let (first, second) = stream.split_at(13);
        let mut frames = decoder.feed(first);
        frames.extend(decoder.feed(second));
        assert_eq!(frames, [Frame::Ubx { class: CLASS_NAV, id: 0x3b, payload: vec![7; 40] }, Frame::Rtcm(rtcm.to_vec()), Frame::Ubx { class: CLASS_ACK, id: ACK_ACK, payload: vec![6, 0x8a] }]);
        let mut corrupt = svin;
        corrupt[10] ^= 0xff;
        assert!(Decoder::default().feed(&corrupt).is_empty(), "a bad checksum drops the frame");
    }

    #[test]
    fn a_generation_9_receiver_is_found_at_its_baud_and_set_up_as_a_survey_in_base() {
        let device = Receiver::new(57600, true, "00190000", vec!["FWVER=HPG 1.32", "PROTVER=27.31", "MOD=ZED-F9P"]);
        let mut base = UbxBase::new(device, survey());
        assert!(base.configure());
        assert_eq!((base.board, base.protocol_27), (Board::UBlox9F9pL1L2, true));
        assert_eq!(base.transport.bauds, [38400, 57600, 115200], "38400 answers nothing, 57600 acks the probe, then the receiver is moved to 115200");
        let valsets: Vec<Vec<u32>> = base.transport.written.iter().filter(|(c, i, _)| (*c, *i) == CFG_VALSET).map(|(_, _, p)| keys_in(p)).collect();
        let tmode = valsets.iter().find(|keys| keys.contains(&KEY_TMODE_MODE)).expect("survey-in is configured");
        assert_eq!(tmode[..3], [KEY_TMODE_MODE, KEY_TMODE_SVIN_MIN_DUR, KEY_TMODE_SVIN_ACC_LIMIT]);
        assert_eq!(tmode[3..], [KEY_MSGOUT_NAV_SVIN + 1, KEY_MSGOUT_NAV_SVIN + 3], "NAV-SVIN on UART1 and USB");
        let outputs = valsets.iter().find(|keys| keys.contains(&(KEY_MSGOUT_NAV_PVT + 1))).unwrap();
        assert!(outputs.contains(&(KEY_MSGOUT_RXM_RTCM + 3)) && outputs.contains(&(KEY_MSGOUT_NAV_HPPOSLLH + 1)));
    }

    #[test]
    fn a_survey_that_completes_turns_the_rtcm_stream_on() {
        let device = Receiver::new(38400, true, "00190000", vec!["MOD=ZED-F9P"]);
        let mut base = UbxBase::new(device, survey());
        assert!(base.configure());
        let svin: Vec<u8> = [vec![0; 8], 181u32.to_le_bytes().to_vec(), 400_000_000_i32.to_le_bytes().to_vec(), 0i32.to_le_bytes().to_vec(), 400_000_000_i32.to_le_bytes().to_vec(), vec![0; 4], 15_000u32.to_le_bytes().to_vec(), 300u32.to_le_bytes().to_vec(), vec![1, 0, 0, 0]].concat();
        base.transport.reply(CLASS_NAV, 0x3b, &svin);
        base.transport.pending.extend([0xd3, 0x00, 0x01, 0x99, 0, 0, 0]);
        let before = base.transport.written.len();
        assert_eq!(base.receive(100), Some(true));
        let events = base.take_events();
        let Event::SurveyIn(status) = &events[0] else { panic!("{events:?}") };
        assert_eq!((status.duration_s, status.mean_accuracy_mm, status.flags), (181, 1500, 1));
        assert!((status.latitude - 45.0).abs() < 0.5 && status.longitude.abs() < 1e-9, "{status:?}");
        assert_eq!(events[1], Event::Rtcm(vec![0xd3, 0x00, 0x01, 0x99, 0, 0, 0]));
        let activation = keys_in(&base.transport.written[before].2);
        assert_eq!(activation[0], KEY_RATE_MEAS);
        assert!(activation.contains(&(KEY_MSGOUT_RTCM[0] + 1)) && activation.contains(&(KEY_MSGOUT_NAV_SVIN + 3)));
    }

    #[test]
    fn an_m8p_falls_back_to_cfg_prt_and_tmode3_and_a_fixed_base_streams_at_once() {
        let device = Receiver::new(9600, false, "00080000", vec!["PROTVER=20.30"]);
        let fixed = BasePlan::Fixed(BasePosition { latitude: 47.3977419, longitude: 8.5455938, altitude_m: 488.0, accuracy_mm: 1500.0 });
        let mut base = UbxBase::new(device, fixed);
        assert!(base.configure());
        assert_eq!((base.board, base.protocol_27), (Board::UBlox8, false));
        assert_eq!(*base.transport.bauds.last().unwrap(), BAUD_M8_AND_NEWER, "an M8 is moved to 115200 once MON-VER names it");
        let tmode: Vec<&Vec<u8>> = base.transport.written.iter().filter(|(c, i, _)| (*c, *i) == CFG_TMODE3).map(|(_, _, p)| p).collect();
        assert_eq!(tmode.len(), 2, "time mode is cleared, then set");
        assert_eq!(u16::from_le_bytes([tmode[1][2], tmode[1][3]]), 2 | 1 << 8);
        assert_eq!(le_i32(tmode[1], 4), 473_977_419);
        assert_eq!(le_u32(tmode[1], 20), 15000);
        let rates: Vec<(u8, u8, u8)> = base.transport.written.iter().filter(|(c, i, _)| (*c, *i) == CFG_MSG).map(|(_, _, p)| (p[0], p[1], p[2])).collect();
        assert!(rates.ends_with(&[(CLASS_NAV, 0x3b, 0), (0xf5, 0x05, 5), (0xf5, 0x4d, 1), (0xf5, 0x57, 1), (0xf5, 0xe6, 1), (0xf5, 0x61, 1), (0xf5, 0x7f, 1)]));
    }

    #[test]
    fn nothing_answering_fails_the_configuration() {
        let device = Receiver::new(1, true, "00190000", Vec::new());
        let mut base = UbxBase::new(device, survey());
        assert!(!base.configure());
        assert_eq!(base.transport.bauds, PROBE_BAUD_RATES);
    }

    #[test]
    fn satellites_and_boards_decode_like_the_px4_driver() {
        let sv = |gnss: u8, id: u8, flags: u8| [gnss, id, 40, 30, 90, 0, 0, 0, flags, 0, 0, 0];
        let payload: Vec<u8> = [vec![0, 0, 0, 0, 1, 3, 0, 0], sv(0, 7, 1).to_vec(), sv(2, 11, 0).to_vec(), sv(6, 40, 1).to_vec()].concat();
        let (count, list) = nav_sat(&payload);
        assert_eq!(count, 3);
        assert_eq!(list.iter().map(|s| (s.svid, s.used, s.azimuth_raw)).collect::<Vec<_>>(), [(7, true, 63), (221, false, 63), (255, true, 63)]);
        let mut legacy = Receiver::new(0, false, "00190000", vec!["FWVER=HPGL1L5 1.40", "MOD=ZED-F9P"]);
        assert_eq!(board_of(&legacy.mon_ver()), Board::UBlox9F9pL1L5, "the firmware string names the L1/L5 part first");
        legacy.hardware = "000A0000";
        assert_eq!(board_of(&legacy.mon_ver()), Board::UBlox10);
    }
}
