use mavlink::dialects::ardupilotmega::*;
use mavlink::MavHeader;
use std::sync::OnceLock;

pub const OVERRIDE: &str = "QGC_CORE_SAMPLE_TLOG";

const START_US: u64 = 1_700_000_000_000_000;
const SECONDS: u64 = 60;
const TICK_US: u64 = 100_000;

pub fn bytes() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| match std::env::var_os(OVERRIDE) {
        Some(path) => std::fs::read(&path).unwrap_or_else(|e| panic!("{OVERRIDE}={}: {e}", path.to_string_lossy())),
        None => synthesize(),
    })
}

pub fn path() -> String {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Some(path) = std::env::var_os(OVERRIDE) {
            return path.to_string_lossy().into_owned();
        }
        let path = std::env::temp_dir().join(format!("qgc-core-sample-{}.tlog", std::process::id()));
        std::fs::write(&path, bytes()).expect("the synthesized sample log written to the temp directory");
        path.to_string_lossy().into_owned()
    })
    .clone()
}

struct Recorder {
    out: Vec<u8>,
    sequences: [u8; 256],
}

impl Recorder {
    fn v2(&mut self, at_us: u64, system_id: u8, component_id: u8, message: MavMessage) {
        let header = self.header(system_id, component_id);
        let mut frame = Vec::new();
        mavlink::write_v2_msg(&mut frame, header, &message).expect("a v2 frame");
        self.out.extend(crate::tlog::record(at_us, &frame));
    }

    fn v1(&mut self, at_us: u64, system_id: u8, component_id: u8, message: MavMessage) {
        let header = self.header(system_id, component_id);
        let mut frame = Vec::new();
        mavlink::write_v1_msg(&mut frame, header, &message).expect("a v1 frame");
        self.out.extend(crate::tlog::record(at_us, &frame));
    }

    fn header(&mut self, system_id: u8, component_id: u8) -> MavHeader {
        let sequence = self.sequences[system_id as usize];
        self.sequences[system_id as usize] = sequence.wrapping_add(1);
        MavHeader { system_id, component_id, sequence }
    }
}

fn synthesize() -> Vec<u8> {
    let mut log = Recorder { out: Vec::new(), sequences: [0; 256] };
    for tick in 0..SECONDS * 1_000_000 / TICK_US {
        let at = START_US + tick * TICK_US;
        let t = tick as f32 * TICK_US as f32 / 1e6;
        let (lat, lon) = (473_977_420 + (t * 50.0) as i32, 85_455_940 + (t * 30.0) as i32);
        if tick % 10 == 0 {
            log.v2(at, 1, 1, MavMessage::HEARTBEAT(vehicle_heartbeat()));
            log.v1(at + 5, 255, 190, MavMessage::HEARTBEAT(gcs_heartbeat()));
            log.v2(at + 10, 1, 1, MavMessage::SYS_STATUS(SYS_STATUS_DATA { voltage_battery: 15_600, current_battery: 1_250, battery_remaining: 80, ..Default::default() }));
            log.v2(at + 20, 1, 1, MavMessage::BATTERY_STATUS(battery(t)));
            log.v2(at + 30, 1, 1, MavMessage::SCALED_PRESSURE(SCALED_PRESSURE_DATA { time_boot_ms: (t * 1000.0) as u32, press_abs: 1013.25, temperature: 2_150, ..Default::default() }));
            log.v2(at + 40, 1, 1, MavMessage::WIND(WIND_DATA { direction: 135.0, speed: 3.5, speed_z: 0.0 }));
        }
        if tick % 2 == 0 {
            log.v2(at + 50, 1, 1, MavMessage::GPS_RAW_INT(gps(t, lat, lon)));
            log.v2(at + 60, 1, 1, MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA { time_boot_ms: (t * 1000.0) as u32, lat, lon, alt: 488_000, relative_alt: 20_000, hdg: ((t * 600.0) as u16) % 36_000, ..Default::default() }));
            log.v2(at + 70, 1, 1, MavMessage::VFR_HUD(VFR_HUD_DATA { airspeed: 5.0, groundspeed: 5.2, alt: 488.0, climb: 0.1, heading: ((t * 6.0) as i16) % 360, throttle: 45 }));
            log.v2(at + 80, 1, 1, MavMessage::LOCAL_POSITION_NED(LOCAL_POSITION_NED_DATA { time_boot_ms: (t * 1000.0) as u32, x: t, y: t / 2.0, z: -20.0, ..Default::default() }));
        }
        log.v2(at + 90, 1, 1, MavMessage::ATTITUDE(ATTITUDE_DATA { time_boot_ms: (t * 1000.0) as u32, roll: 0.02, pitch: -0.03, yaw: (t * 0.1) % (2.0 * std::f32::consts::PI) - std::f32::consts::PI, ..Default::default() }));
    }
    log.out
}

fn vehicle_heartbeat() -> HEARTBEAT_DATA {
    HEARTBEAT_DATA {
        custom_mode: 5,
        mavtype: MavType::MAV_TYPE_QUADROTOR,
        autopilot: MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
        base_mode: MavModeFlag::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED | MavModeFlag::MAV_MODE_FLAG_STABILIZE_ENABLED,
        system_status: MavState::MAV_STATE_STANDBY,
        mavlink_version: 3,
    }
}

fn gcs_heartbeat() -> HEARTBEAT_DATA {
    HEARTBEAT_DATA { mavtype: MavType::MAV_TYPE_GCS, autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID, mavlink_version: 3, ..Default::default() }
}

fn battery(t: f32) -> BATTERY_STATUS_DATA {
    let cell = 3_950 - (t * 2.0) as u16;
    let mut voltages = [u16::MAX; 10];
    voltages[..4].fill(cell);
    BATTERY_STATUS_DATA { current_consumed: (t * 5.0) as i32, temperature: 3_000, voltages, current_battery: 1_250, battery_remaining: 80, ..Default::default() }
}

fn gps(t: f32, lat: i32, lon: i32) -> GPS_RAW_INT_DATA {
    GPS_RAW_INT_DATA {
        time_usec: (t * 1e6) as u64,
        lat,
        lon,
        alt: 488_000,
        eph: 90,
        epv: 140,
        vel: 520,
        cog: 9_000,
        fix_type: GpsFixType::GPS_FIX_TYPE_3D_FIX,
        satellites_visible: 14,
        ..Default::default()
    }
}
