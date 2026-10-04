#[allow(deprecated)]
use mavlink::dialects::ardupilotmega::{COMMAND_INT_DATA, COMMAND_LONG_DATA, FILE_TRANSFER_PROTOCOL_DATA, LOGGING_ACK_DATA, MavOdidCategoryEu, MavOdidClassEu, MavOdidClassificationType, MavOdidDescType, MavOdidIdType, MavOdidOperatorIdType, MavOdidOperatorLocationType, MavOdidUaType, OPEN_DRONE_ID_BASIC_ID_DATA, OPEN_DRONE_ID_OPERATOR_ID_DATA, OPEN_DRONE_ID_SELF_ID_DATA, OPEN_DRONE_ID_SYSTEM_DATA, MISSION_ACK_DATA, MISSION_CLEAR_ALL_DATA, REQUEST_EVENT_DATA, MISSION_COUNT_DATA, MISSION_ITEM_DATA, MISSION_ITEM_INT_DATA, MISSION_REQUEST_INT_DATA, MISSION_REQUEST_LIST_DATA, COMMAND_ACK_DATA, MavCmd, MavMissionResult, MavMissionType, MavFrame, MavMessage, MavParamType, MavResult, PARAM_EXT_REQUEST_LIST_DATA, PARAM_MAP_RC_DATA, PARAM_EXT_REQUEST_READ_DATA, PARAM_EXT_SET_DATA, MavParamExtType, PARAM_REQUEST_LIST_DATA, PARAM_REQUEST_READ_DATA, PARAM_SET_DATA, PositionTargetTypemask, RC_CHANNELS_OVERRIDE_DATA, LOG_ERASE_DATA, LOG_REQUEST_DATA_DATA, LOG_REQUEST_END_DATA, LOG_REQUEST_LIST_DATA, MANUAL_CONTROL_DATA, SET_POSITION_TARGET_LOCAL_NED_DATA, SERIAL_CONTROL_DATA, SerialControlDev, SerialControlFlag, SYSTEM_TIME_DATA, PING_DATA, TERRAIN_DATA_DATA};
use mavlink::types::CharArray;
use mavlink::{MAVLinkV2MessageRaw, MavHeader, MavlinkVersion, MessageData};
use num_traits::FromPrimitive;
use std::sync::atomic::{AtomicU8, Ordering};

const ACCEL_CAL_ACK_COMMAND: MavCmd = MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS;

pub const DEFAULT_GCS_SYSTEM: u8 = 255;

pub fn gcs_system() -> u8 {
    crate::settingsstore::raw_setting("settings.mavlinkSettings.gcsMavlinkSystemID").and_then(|v| v.as_u64()).and_then(|id| u8::try_from(id).ok()).filter(|id| *id != 0).unwrap_or(DEFAULT_GCS_SYSTEM)
}

pub fn for_us(target_system: u8) -> bool {
    target_system == 0 || target_system == gcs_system()
}
pub const GCS_COMPONENT: u8 = 190;
pub const GUIDED_ITEM_CURRENT: u8 = 2;
fn plan_type(plan: u8) -> Option<MavMissionType> {
    MavMissionType::from_u8(plan)
}

static SEQUENCE: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, PartialEq)]
pub enum Outbound {
    CommandLong { target: (u8, u8), command: u16, params: [f64; 7] },
    GpsGlobalOrigin { system: u8, latitude: f64, longitude: f64, altitude: f64 },
    CommandLongTry { target: (u8, u8), command: u16, params: [f64; 7], confirmation: u8 },
    CommandInt { target: (u8, u8), command: u16, frame: u8, params: [f64; 7], x: i32, y: i32 },
    SetMode { system: u8, base_mode: u8, custom_mode: u32 },
    RawCommandLong { target: (u8, u8), command: u16, params: [f64; 7] },
    PositionTargetLocalNed { target: (u8, u8), frame: u8, type_mask: u16, x: f64, y: f64, z: f64 },
    GuidedMissionItem { target: (u8, u8), latitude: f64, longitude: f64, altitude_relative: f64 },
    ParamRequestList { target: (u8, u8) },
    ParamRequestRead { target: (u8, u8), name: Option<String>, index: i16 },
    ParamSet { target: (u8, u8), name: String, bits: f32, param_type: u8 },
    Ftp { target: (u8, u8), payload: [u8; 251] },
    MissionRequestList { target: (u8, u8), plan: u8 },
    MissionClearAll { target: (u8, u8), plan: u8 },
    RequestEvent { target: (u8, u8), sequence: u16 },
    MissionRequestInt { target: (u8, u8), plan: u8, seq: u16 },
    MissionCount { target: (u8, u8), plan: u8, count: u16 },
    MissionItemInt { target: (u8, u8), plan: u8, item: crate::plantransfer::Item },
    MissionAck { target: (u8, u8), plan: u8, result: u8 },
    Odid { target: (u8, u8), message: crate::remoteid::Message },
    LoggingAck { target: (u8, u8), sequence: u16 },
    AccelCalAck,
    SystemTime { time_unix_usec: u64 },
    Ping { time_usec: u64, seq: u32, target: (u8, u8) },
    TerrainData { lat: i32, lon: i32, grid_spacing: u16, gridbit: u8, data: [i16; 16] },
    RcOverride { target: (u8, u8), channels: [u16; 18] },
    ManualControl { target: u8, x: i16, y: i16, z: i16, r: i16 },
    JoystickManualControl { target: u8, x: i16, y: i16, z: i16, r: i16, buttons: u16, buttons2: u16, enabled_extensions: u8, extensions: [i16; 8] },
    LogRequestList { target: (u8, u8), start: u16, end: u16 },
    LogRequestData { target: (u8, u8), id: u16, offset: u32, count: u32 },
    LogRequestEnd { target: (u8, u8) },
    ParamExtRequestList { target: (u8, u8) },
    ParamMapRc { target: (u8, u8), id: String, index: i16, tuning: u8, center: f32, scale: f32, min: f32, max: f32 },
    ParamExtRequestRead { target: (u8, u8), id: String },
    ParamExtSet { target: (u8, u8), id: String, value: [u8; 128], param_type: u8 },
    LogErase { target: (u8, u8) },
    ShellData { target: (u8, u8), data: Vec<u8> },
    GpsRtcmData { data: mavlink::dialects::ardupilotmega::GPS_RTCM_DATA_DATA },
    FollowTarget { data: mavlink::dialects::ardupilotmega::FOLLOW_TARGET_DATA },
    GimbalAttitudeRates { target: (u8, u8), flags: u32, device_id: u8, pitch_rate: f32, yaw_rate: f32 },
    GlobalPositionInt { data: mavlink::dialects::ardupilotmega::GLOBAL_POSITION_INT_DATA },
    SetupSigning { data: mavlink::dialects::ardupilotmega::SETUP_SIGNING_DATA },
    RequestDataStream { target: (u8, u8), stream: u8, rate: u16 },
    GcsHeartbeat,
}

pub fn chars<const N: usize>(text: &str) -> CharArray<N> {
    let bytes = text.as_bytes();
    CharArray::from(std::array::from_fn(|i| bytes.get(i).copied().unwrap_or(0)))
}

pub fn command_known(command: u16) -> bool {
    MavCmd::from_u32(command as u32).is_some()
}

pub fn frame_known(frame: u8) -> bool {
    MavFrame::from_u8(frame).is_some()
}

pub fn param_id(name: &str) -> CharArray<16> {
    chars(name)
}

pub struct CommandLongBits {
    pub target: (u8, u8),
    pub command: u16,
    pub params: [f32; 7],
}

impl MessageData for CommandLongBits {
    type Message = MavMessage;
    const ID: u32 = 76;
    const NAME: &'static str = "COMMAND_LONG";
    const EXTRA_CRC: u8 = 152;
    const ENCODED_LEN: usize = 33;

    fn ser(&self, _version: MavlinkVersion, payload: &mut [u8]) -> usize {
        self.params.iter().enumerate().for_each(|(i, p)| payload[i * 4..i * 4 + 4].copy_from_slice(&p.to_le_bytes()));
        payload[28..30].copy_from_slice(&self.command.to_le_bytes());
        payload[30] = self.target.0;
        payload[31] = self.target.1;
        payload[32] = 0;
        Self::ENCODED_LEN
    }

    fn deser(_version: MavlinkVersion, input: &[u8]) -> Result<Self, mavlink::error::ParserError> {
        let mut payload = [0u8; 33];
        payload[..input.len().min(33)].copy_from_slice(&input[..input.len().min(33)]);
        let param = |i: usize| f32::from_le_bytes([payload[i * 4], payload[i * 4 + 1], payload[i * 4 + 2], payload[i * 4 + 3]]);
        Ok(CommandLongBits { target: (payload[30], payload[31]), command: u16::from_le_bytes([payload[28], payload[29]]), params: std::array::from_fn(param) })
    }
}

pub struct CommandIntBits {
    pub target: (u8, u8),
    pub command: u16,
    pub frame: u8,
    pub params: [f32; 4],
    pub x: i32,
    pub y: i32,
    pub z: f32,
}

impl MessageData for CommandIntBits {
    type Message = MavMessage;
    const ID: u32 = 75;
    const NAME: &'static str = "COMMAND_INT";
    const EXTRA_CRC: u8 = 158;
    const ENCODED_LEN: usize = 35;

    fn ser(&self, _version: MavlinkVersion, payload: &mut [u8]) -> usize {
        self.params.iter().enumerate().for_each(|(i, p)| payload[i * 4..i * 4 + 4].copy_from_slice(&p.to_le_bytes()));
        payload[16..20].copy_from_slice(&self.x.to_le_bytes());
        payload[20..24].copy_from_slice(&self.y.to_le_bytes());
        payload[24..28].copy_from_slice(&self.z.to_le_bytes());
        payload[28..30].copy_from_slice(&self.command.to_le_bytes());
        payload[30] = self.target.0;
        payload[31] = self.target.1;
        payload[32] = self.frame;
        payload[33] = 0;
        payload[34] = 0;
        Self::ENCODED_LEN
    }

    fn deser(_version: MavlinkVersion, input: &[u8]) -> Result<Self, mavlink::error::ParserError> {
        let mut payload = [0u8; 35];
        payload[..input.len().min(35)].copy_from_slice(&input[..input.len().min(35)]);
        let float = |at: usize| f32::from_le_bytes([payload[at], payload[at + 1], payload[at + 2], payload[at + 3]]);
        let int = |at: usize| i32::from_le_bytes([payload[at], payload[at + 1], payload[at + 2], payload[at + 3]]);
        Ok(CommandIntBits { target: (payload[30], payload[31]), command: u16::from_le_bytes([payload[28], payload[29]]), frame: payload[32], params: std::array::from_fn(|i| float(i * 4)), x: int(16), y: int(20), z: float(24) })
    }
}

pub struct SetModeBits {
    pub system: u8,
    pub base_mode: u8,
    pub custom_mode: u32,
}

impl MessageData for SetModeBits {
    type Message = MavMessage;
    const ID: u32 = 11;
    const NAME: &'static str = "SET_MODE";
    const EXTRA_CRC: u8 = 89;
    const ENCODED_LEN: usize = 6;

    fn ser(&self, _version: MavlinkVersion, payload: &mut [u8]) -> usize {
        payload[..4].copy_from_slice(&self.custom_mode.to_le_bytes());
        payload[4] = self.system;
        payload[5] = self.base_mode;
        Self::ENCODED_LEN
    }

    fn deser(_version: MavlinkVersion, input: &[u8]) -> Result<Self, mavlink::error::ParserError> {
        let mut payload = [0u8; 6];
        payload[..input.len().min(6)].copy_from_slice(&input[..input.len().min(6)]);
        Ok(SetModeBits { custom_mode: u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]), system: payload[4], base_mode: payload[5] })
    }
}

#[allow(deprecated)]
pub fn message(send: &Outbound) -> Option<MavMessage> {
    match send {
        Outbound::CommandLongTry { target, command, params, confirmation } => match message(&Outbound::CommandLong { target: *target, command: *command, params: *params })? {
            MavMessage::COMMAND_LONG(data) => Some(MavMessage::COMMAND_LONG(COMMAND_LONG_DATA { confirmation: *confirmation, ..data })),
            other => Some(other),
        },
        Outbound::CommandLong { target, command, params: p } => Some(MavMessage::COMMAND_LONG(COMMAND_LONG_DATA {
            param1: p[0] as f32,
            param2: p[1] as f32,
            param3: p[2] as f32,
            param4: p[3] as f32,
            param5: p[4] as f32,
            param6: p[5] as f32,
            param7: p[6] as f32,
            command: MavCmd::from_u32(*command as u32)?,
            target_system: target.0,
            target_component: target.1,
            confirmation: 0,
        })),
        Outbound::CommandInt { target, command, frame: f, params: p, x, y } => Some(MavMessage::COMMAND_INT(COMMAND_INT_DATA {
            param1: p[0] as f32,
            param2: p[1] as f32,
            param3: p[2] as f32,
            param4: p[3] as f32,
            x: *x,
            y: *y,
            z: p[6] as f32,
            command: MavCmd::from_u32(*command as u32)?,
            target_system: target.0,
            target_component: target.1,
            frame: MavFrame::from_u8(*f)?,
            current: 0,
            autocontinue: 0,
        })),
        Outbound::SetMode { .. } | Outbound::RawCommandLong { .. } => None,
        Outbound::ParamRequestList { target } => Some(MavMessage::PARAM_REQUEST_LIST(PARAM_REQUEST_LIST_DATA { target_system: target.0, target_component: target.1 })),
        Outbound::ParamRequestRead { target, name, index } => Some(MavMessage::PARAM_REQUEST_READ(PARAM_REQUEST_READ_DATA { param_index: if name.is_some() { -1 } else { *index }, target_system: target.0, target_component: target.1, param_id: param_id(name.as_deref().unwrap_or("")) })),
        Outbound::MissionRequestList { target, plan } => Some(MavMessage::MISSION_REQUEST_LIST(MISSION_REQUEST_LIST_DATA { target_system: target.0, target_component: target.1, mission_type: plan_type(*plan)? })),
        Outbound::MissionClearAll { target, plan } => Some(MavMessage::MISSION_CLEAR_ALL(MISSION_CLEAR_ALL_DATA { target_system: target.0, target_component: target.1, mission_type: plan_type(*plan)? })),
        Outbound::RequestEvent { target, sequence } => Some(MavMessage::REQUEST_EVENT(REQUEST_EVENT_DATA { first_sequence: *sequence, last_sequence: *sequence, target_system: target.0, target_component: target.1 })),
        Outbound::MissionRequestInt { target, plan, seq } => Some(MavMessage::MISSION_REQUEST_INT(MISSION_REQUEST_INT_DATA { seq: *seq, target_system: target.0, target_component: target.1, mission_type: plan_type(*plan)? })),
        Outbound::MissionCount { target, plan, count } => Some(MavMessage::MISSION_COUNT(MISSION_COUNT_DATA { count: *count, target_system: target.0, target_component: target.1, mission_type: plan_type(*plan)?, opaque_id: 0 })),
        Outbound::MissionAck { target, plan, result } => Some(MavMessage::MISSION_ACK(MISSION_ACK_DATA { target_system: target.0, target_component: target.1, mavtype: MavMissionResult::from_u8(*result)?, mission_type: plan_type(*plan)?, opaque_id: 0 })),
        Outbound::MissionItemInt { target, plan, item } => {
            let scale = |v: f64| if item.frame == crate::plantransfer::FRAME_MISSION { v as i32 } else { (v * 1e7) as i32 };
            Some(MavMessage::MISSION_ITEM_INT(MISSION_ITEM_INT_DATA {
                param1: item.params[0] as f32,
                param2: item.params[1] as f32,
                param3: item.params[2] as f32,
                param4: item.params[3] as f32,
                x: scale(item.params[4]),
                y: scale(item.params[5]),
                z: item.params[6] as f32,
                seq: item.seq,
                command: MavCmd::from_u32(item.command as u32)?,
                target_system: target.0,
                target_component: target.1,
                frame: MavFrame::from_u8(item.frame)?,
                current: u8::from(item.current),
                autocontinue: u8::from(item.auto_continue),
                mission_type: plan_type(*plan)?,
            }))
        }
        Outbound::Odid { target, message } => Some(match message {
            crate::remoteid::Message::System { location_type, classification_type, latitude, longitude, area_count, area_radius, category_eu, class_eu, altitude, timestamp_2019, .. } => MavMessage::OPEN_DRONE_ID_SYSTEM(OPEN_DRONE_ID_SYSTEM_DATA {
                operator_latitude: *latitude,
                operator_longitude: *longitude,
                area_ceiling: -1000.0,
                area_floor: -1000.0,
                operator_altitude_geo: *altitude,
                timestamp: *timestamp_2019,
                area_count: *area_count,
                area_radius: *area_radius,
                target_system: target.0,
                target_component: target.1,
                id_or_mac: [0; 20],
                operator_location_type: MavOdidOperatorLocationType::from_u32(*location_type)?,
                classification_type: MavOdidClassificationType::from_u32(*classification_type)?,
                category_eu: MavOdidCategoryEu::from_u32(*category_eu)?,
                class_eu: MavOdidClassEu::from_u32(*class_eu)?,
            }),
            crate::remoteid::Message::BasicId { id_type, ua_type, uas_id } => MavMessage::OPEN_DRONE_ID_BASIC_ID(OPEN_DRONE_ID_BASIC_ID_DATA {
                target_system: target.0,
                target_component: target.1,
                id_or_mac: [0; 20],
                id_type: MavOdidIdType::from_u32(*id_type)?,
                ua_type: MavOdidUaType::from_u32(*ua_type)?,
                uas_id: std::array::from_fn(|i| uas_id.as_bytes().get(i).copied().unwrap_or(0)),
            }),
            crate::remoteid::Message::SelfId { description_type, description } => MavMessage::OPEN_DRONE_ID_SELF_ID(OPEN_DRONE_ID_SELF_ID_DATA {
                target_system: target.0,
                target_component: target.1,
                id_or_mac: [0; 20],
                description_type: MavOdidDescType::from_u32(*description_type)?,
                description: chars(description),
            }),
            crate::remoteid::Message::OperatorId { id_type, operator_id } => MavMessage::OPEN_DRONE_ID_OPERATOR_ID(OPEN_DRONE_ID_OPERATOR_ID_DATA {
                target_system: target.0,
                target_component: target.1,
                id_or_mac: [0; 20],
                operator_id_type: MavOdidOperatorIdType::from_u32(*id_type)?,
                operator_id: chars(operator_id),
            }),
        }),
        Outbound::GpsRtcmData { data } => Some(MavMessage::GPS_RTCM_DATA(data.clone())),
        Outbound::GpsGlobalOrigin { system, latitude, longitude, altitude } => Some(MavMessage::SET_GPS_GLOBAL_ORIGIN(mavlink::dialects::ardupilotmega::SET_GPS_GLOBAL_ORIGIN_DATA {
            latitude: (latitude * 1e7) as i32,
            longitude: (longitude * 1e7) as i32,
            altitude: (altitude * 1e3) as i32,
            target_system: *system,
            time_usec: 0,
        })),
        Outbound::FollowTarget { data } => Some(MavMessage::FOLLOW_TARGET(data.clone())),
        Outbound::GimbalAttitudeRates { target, flags, device_id, pitch_rate, yaw_rate } => Some(MavMessage::GIMBAL_MANAGER_SET_ATTITUDE(mavlink::dialects::ardupilotmega::GIMBAL_MANAGER_SET_ATTITUDE_DATA {
            flags: mavlink::dialects::ardupilotmega::GimbalManagerFlags::from_bits_truncate(*flags),
            q: [f32::NAN; 4],
            angular_velocity_x: f32::NAN,
            angular_velocity_y: *pitch_rate,
            angular_velocity_z: *yaw_rate,
            target_system: target.0,
            target_component: target.1,
            gimbal_device_id: *device_id,
        })),
        Outbound::GlobalPositionInt { data } => Some(MavMessage::GLOBAL_POSITION_INT(data.clone())),
        Outbound::RequestDataStream { target, stream, rate } => Some(MavMessage::REQUEST_DATA_STREAM(mavlink::dialects::ardupilotmega::REQUEST_DATA_STREAM_DATA {
            req_message_rate: *rate,
            target_system: target.0,
            target_component: target.1,
            req_stream_id: *stream,
            start_stop: 1,
        })),
        Outbound::SetupSigning { data } => Some(MavMessage::SETUP_SIGNING(data.clone())),
        Outbound::JoystickManualControl { target, x, y, z, r, buttons, buttons2, enabled_extensions, extensions: e } => Some(MavMessage::MANUAL_CONTROL(MANUAL_CONTROL_DATA {
            x: *x,
            y: *y,
            z: *z,
            r: *r,
            buttons: *buttons,
            target: *target,
            buttons2: *buttons2,
            enabled_extensions: *enabled_extensions,
            s: e[0],
            t: e[1],
            aux1: e[2],
            aux2: e[3],
            aux3: e[4],
            aux4: e[5],
            aux5: e[6],
            aux6: e[7],
        })),
        Outbound::ManualControl { target, x, y, z, r } => Some(MavMessage::MANUAL_CONTROL(MANUAL_CONTROL_DATA { x: *x, y: *y, z: *z, r: *r, target: *target, ..MANUAL_CONTROL_DATA::default() })),
        Outbound::RcOverride { target, channels: c } => Some(MavMessage::RC_CHANNELS_OVERRIDE(RC_CHANNELS_OVERRIDE_DATA {
            chan1_raw: c[0],
            chan2_raw: c[1],
            chan3_raw: c[2],
            chan4_raw: c[3],
            chan5_raw: c[4],
            chan6_raw: c[5],
            chan7_raw: c[6],
            chan8_raw: c[7],
            target_system: target.0,
            target_component: target.1,
            chan9_raw: c[8],
            chan10_raw: c[9],
            chan11_raw: c[10],
            chan12_raw: c[11],
            chan13_raw: c[12],
            chan14_raw: c[13],
            chan15_raw: c[14],
            chan16_raw: c[15],
            chan17_raw: c[16],
            chan18_raw: c[17],
        })),
        Outbound::LogRequestList { target, start, end } => Some(MavMessage::LOG_REQUEST_LIST(LOG_REQUEST_LIST_DATA { start: *start, end: *end, target_system: target.0, target_component: target.1 })),
        Outbound::LogRequestData { target, id, offset, count } => Some(MavMessage::LOG_REQUEST_DATA(LOG_REQUEST_DATA_DATA { ofs: *offset, count: *count, id: *id, target_system: target.0, target_component: target.1 })),
        Outbound::LogRequestEnd { target } => Some(MavMessage::LOG_REQUEST_END(LOG_REQUEST_END_DATA { target_system: target.0, target_component: target.1 })),
        Outbound::ParamMapRc { target, id, index, tuning, center, scale, min, max } => Some(MavMessage::PARAM_MAP_RC(PARAM_MAP_RC_DATA {
            param_value0: *center,
            scale: *scale,
            param_value_min: *min,
            param_value_max: *max,
            param_index: *index,
            target_system: target.0,
            target_component: target.1,
            param_id: chars(id),
            parameter_rc_channel_index: *tuning,
        })),
        Outbound::ParamExtRequestList { target } => Some(MavMessage::PARAM_EXT_REQUEST_LIST(PARAM_EXT_REQUEST_LIST_DATA { target_system: target.0, target_component: target.1 })),
        Outbound::ParamExtRequestRead { target, id } => Some(MavMessage::PARAM_EXT_REQUEST_READ(PARAM_EXT_REQUEST_READ_DATA { param_index: -1, target_system: target.0, target_component: target.1, param_id: chars(id) })),
        Outbound::ParamExtSet { target, id, value, param_type } => Some(MavMessage::PARAM_EXT_SET(PARAM_EXT_SET_DATA {
            target_system: target.0,
            target_component: target.1,
            param_id: chars(id),
            param_value: CharArray::from(*value),
            param_type: MavParamExtType::from_u8(*param_type)?,
        })),
        Outbound::ShellData { target, data } => {
            let mut padded = [0u8; 70];
            padded[..data.len().min(70)].copy_from_slice(&data[..data.len().min(70)]);
            Some(MavMessage::SERIAL_CONTROL(SERIAL_CONTROL_DATA {
                baudrate: 0,
                timeout: 0,
                device: SerialControlDev::SERIAL_CONTROL_DEV_SHELL,
                flags: SerialControlFlag::SERIAL_CONTROL_FLAG_EXCLUSIVE | SerialControlFlag::SERIAL_CONTROL_FLAG_RESPOND | SerialControlFlag::SERIAL_CONTROL_FLAG_MULTI,
                count: data.len().min(70) as u8,
                data: padded,
                target_system: target.0,
                target_component: target.1,
            }))
        }
        Outbound::GcsHeartbeat => Some(MavMessage::HEARTBEAT(mavlink::dialects::ardupilotmega::HEARTBEAT_DATA {
            custom_mode: 0,
            mavtype: mavlink::dialects::ardupilotmega::MavType::MAV_TYPE_GCS,
            autopilot: mavlink::dialects::ardupilotmega::MavAutopilot::MAV_AUTOPILOT_INVALID,
            base_mode: mavlink::dialects::ardupilotmega::MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED | mavlink::dialects::ardupilotmega::MavModeFlag::MAV_MODE_FLAG_MANUAL_INPUT_ENABLED,
            system_status: mavlink::dialects::ardupilotmega::MavState::MAV_STATE_ACTIVE,
            mavlink_version: 3,
        })),
        Outbound::LogErase { target } => Some(MavMessage::LOG_ERASE(LOG_ERASE_DATA { target_system: target.0, target_component: target.1 })),
        Outbound::TerrainData { lat, lon, grid_spacing, gridbit, data } => Some(MavMessage::TERRAIN_DATA(TERRAIN_DATA_DATA { lat: *lat, lon: *lon, grid_spacing: *grid_spacing, data: *data, gridbit: *gridbit })),
        Outbound::Ping { time_usec, seq, target } => Some(MavMessage::PING(PING_DATA { time_usec: *time_usec, seq: *seq, target_system: target.0, target_component: target.1 })),
        Outbound::SystemTime { time_unix_usec } => Some(MavMessage::SYSTEM_TIME(SYSTEM_TIME_DATA { time_unix_usec: *time_unix_usec, time_boot_ms: 0 })),
        Outbound::AccelCalAck => Some(MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: ACCEL_CAL_ACK_COMMAND, result: MavResult::MAV_RESULT_TEMPORARILY_REJECTED, progress: 0, result_param2: 0, target_system: 0, target_component: 0 })),
        Outbound::LoggingAck { target, sequence } => Some(MavMessage::LOGGING_ACK(LOGGING_ACK_DATA { sequence: *sequence, target_system: target.0, target_component: target.1 })),
        Outbound::Ftp { target, payload } => Some(MavMessage::FILE_TRANSFER_PROTOCOL(FILE_TRANSFER_PROTOCOL_DATA { target_network: 0, target_system: target.0, target_component: target.1, payload: *payload })),
        Outbound::ParamSet { target, name, bits, param_type } => Some(MavMessage::PARAM_SET(PARAM_SET_DATA { param_value: *bits, target_system: target.0, target_component: target.1, param_id: param_id(name), param_type: MavParamType::from_u8(*param_type)? })),
        Outbound::PositionTargetLocalNed { target, frame: f, type_mask, x, y, z } => Some(MavMessage::SET_POSITION_TARGET_LOCAL_NED(SET_POSITION_TARGET_LOCAL_NED_DATA {
            x: *x as f32,
            y: *y as f32,
            z: *z as f32,
            type_mask: PositionTargetTypemask::from_bits_retain(*type_mask),
            target_system: target.0,
            target_component: target.1,
            coordinate_frame: MavFrame::from_u8(*f)?,
            ..Default::default()
        })),
        Outbound::GuidedMissionItem { target, latitude, longitude, altitude_relative } => Some(MavMessage::MISSION_ITEM(MISSION_ITEM_DATA {
            x: *latitude as f32,
            y: *longitude as f32,
            z: *altitude_relative as f32,
            seq: 0,
            command: MavCmd::MAV_CMD_NAV_WAYPOINT,
            target_system: target.0,
            target_component: target.1,
            frame: MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT,
            current: GUIDED_ITEM_CURRENT,
            autocontinue: 1,
            ..Default::default()
        })),
    }
}

pub fn encode(sequence: u8, send: &Outbound) -> Option<Vec<u8>> {
    let header = MavHeader { system_id: gcs_system(), component_id: GCS_COMPONENT, sequence };
    let mut raw = MAVLinkV2MessageRaw::new();
    match send {
        Outbound::SetMode { system, base_mode, custom_mode } => raw.serialize_message_data(header, &SetModeBits { system: *system, base_mode: *base_mode, custom_mode: *custom_mode }),
        Outbound::RawCommandLong { target, command, params } => raw.serialize_message_data(header, &CommandLongBits { target: *target, command: *command, params: params.map(|p| p as f32) }),
        Outbound::CommandInt { target, command, frame, params, x, y } if message(send).is_none() => raw.serialize_message_data(header, &CommandIntBits { target: *target, command: *command, frame: *frame, params: [params[0], params[1], params[2], params[3]].map(|p| p as f32), x: *x, y: *y, z: params[6] as f32 }),
        other => raw.serialize_message(header, &message(other)?),
    }
    Some(raw.raw_bytes().to_vec())
}

pub fn encode_next(send: &Outbound) -> Option<Vec<u8>> {
    encode(SEQUENCE.fetch_add(1, Ordering::Relaxed), send)
}

const X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_MCRF4XX);
const V1_MAGIC: u8 = 0xFE;
const V2_MAGIC: u8 = 0xFD;
const V2_SIGNED: u8 = 0x01;
const SIGNATURE_BYTES: usize = 13;

struct FrameAt {
    sequence: usize,
    system: usize,
    end: usize,
    signed: bool,
}

fn frame_at(bytes: &[u8]) -> Option<FrameAt> {
    match bytes {
        [V1_MAGIC, len, ..] => Some(FrameAt { sequence: 2, system: 3, end: *len as usize + 8, signed: false }),
        [V2_MAGIC, len, incompat, ..] => {
            let signed = incompat & V2_SIGNED != 0;
            Some(FrameAt { sequence: 4, system: 5, end: *len as usize + 12 + if signed { SIGNATURE_BYTES } else { 0 }, signed })
        }
        _ => None,
    }
    .filter(|at| at.end <= bytes.len())
}

fn resequenced(frame: &[u8], at: usize, sequence: u8) -> Option<Vec<u8>> {
    let crc_at = frame.len() - 2;
    let old = u16::from_le_bytes([frame[crc_at], frame[crc_at + 1]]);
    let digested = |body: &[u8]| {
        let mut digest = X25.digest();
        digest.update(&body[1..]);
        digest
    };
    let finished = |digest: &crc::Digest<'_, u16>, extra: u8| {
        let mut digest = digest.clone();
        digest.update(&[extra]);
        digest.finalize()
    };
    let sent = digested(&frame[..crc_at]);
    let extra = (0..=u8::MAX).find(|extra| finished(&sent, *extra) == old)?;
    let body: Vec<u8> = frame[..crc_at].iter().enumerate().map(|(i, b)| if i == at { sequence } else { *b }).collect();
    let crc = finished(&digested(&body), extra);
    Some([body, crc.to_le_bytes().to_vec()].concat())
}

pub fn restamped(bytes: &[u8], system: u8, next: &mut impl FnMut() -> u8) -> Vec<u8> {
    let Some(at) = frame_at(bytes) else { return bytes.to_vec() };
    let (frame, rest) = bytes.split_at(at.end);
    let ours = frame[at.system] == system && !at.signed;
    let head = ours.then(|| resequenced(frame, at.sequence, next())).flatten().unwrap_or_else(|| frame.to_vec());
    [head, restamped(rest, system, next)].concat()
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use mavlink::{ReadVersion, read_versioned_msg};

    fn decode(bytes: &[u8]) -> (MavHeader, MavMessage) {
        read_versioned_msg::<MavMessage, _>(&mut mavlink::peek_reader::PeekReader::new(bytes), ReadVersion::Single(MavlinkVersion::V2)).unwrap()
    }

    fn counter(from: u8) -> impl FnMut() -> u8 {
        let mut next = from;
        move || {
            let sequence = next;
            next = next.wrapping_add(1);
            sequence
        }
    }

    #[test]
    fn each_link_numbers_the_station_frames_it_carries_like_a_qgc_mavlink_channel() {
        let beat = encode(200, &Outbound::GcsHeartbeat).unwrap();
        let ping = encode(201, &Outbound::Ping { time_usec: 1, seq: 2, target: (1, 1) }).unwrap();
        let both = [beat.clone(), ping.clone()].concat();
        let sent = restamped(&both, DEFAULT_GCS_SYSTEM, &mut counter(7));
        let (first, message) = decode(&sent);
        let (second, _) = decode(&sent[beat.len()..]);
        assert_eq!((first.sequence, second.sequence), (7, 8));
        assert!(matches!(message, MavMessage::HEARTBEAT(_)), "the checksum is rebuilt, so the frame still decodes");
        assert_eq!(decode(&restamped(&beat, DEFAULT_GCS_SYSTEM, &mut counter(0))).0.sequence, 0, "another link keeps its own count");
    }

    #[test]
    fn forwarded_signed_and_unparsed_bytes_pass_through_untouched() {
        let vehicle = {
            let mut raw = MAVLinkV2MessageRaw::new();
            raw.serialize_message(MavHeader { system_id: 1, component_id: 1, sequence: 42 }, &MavMessage::HEARTBEAT(Default::default()));
            raw.raw_bytes().to_vec()
        };
        let mut never = || -> u8 { panic!("only station frames take a number") };
        assert_eq!(restamped(&vehicle, DEFAULT_GCS_SYSTEM, &mut never), vehicle);
        let signed = [encode(3, &Outbound::GcsHeartbeat).unwrap(), vec![0; SIGNATURE_BYTES]].concat();
        let signed: Vec<u8> = signed.iter().enumerate().map(|(i, b)| if i == 2 { b | V2_SIGNED } else { *b }).collect();
        assert_eq!(restamped(&signed, DEFAULT_GCS_SYSTEM, &mut never), signed);
        assert_eq!(restamped(&[1, 2, 3], DEFAULT_GCS_SYSTEM, &mut never), vec![1, 2, 3]);
    }

    #[test]
    fn a_command_the_dialect_cannot_decode_still_leaves_with_a_valid_checksum() {
        let raw = encode(9, &Outbound::CommandInt { target: (1, 1), command: 611, frame: 0, params: [0.0; 7], x: 0, y: 0 }).unwrap();
        let sent = restamped(&raw, DEFAULT_GCS_SYSTEM, &mut counter(1));
        let expected: Vec<u8> = raw.iter().enumerate().map(|(i, b)| if i == 4 { 1 } else { *b }).collect();
        assert_eq!(sent[..sent.len() - 2], expected[..expected.len() - 2]);
        let mut buffer = [0u8; 280];
        buffer[..sent.len()].copy_from_slice(&sent);
        assert!(MAVLinkV2MessageRaw::from_bytes_unparsed(buffer).has_valid_crc::<MavMessage>());
    }

    #[test]
    fn a_mission_coordinate_is_truncated_toward_zero_as_qt_passes_it_to_an_int32() {
        let item = crate::plantransfer::Item { seq: 1, frame: 3, command: 16, current: false, auto_continue: true, params: [0.0, 0.0, 0.0, f64::NAN, 47.63311996, -35.36193628, 20.0] };
        let bytes = encode(0, &Outbound::MissionItemInt { target: (1, 1), plan: crate::plantransfer::PLAN_MISSION, item }).unwrap();
        let MavMessage::MISSION_ITEM_INT(sent) = decode(&bytes).1 else { panic!() };
        assert_eq!((sent.x, sent.y), (476331199, -353619362), "PlanManager hands param5 * 1e7 to mission_item_int_pack's int32_t, so 476331199.6 goes out as ...199, never rounded to ...200 (captured from Qt against SITL)");
    }

    #[test]
    fn commands_and_targets_decode_to_what_was_sent() {
        assert_eq!(encode(0, &Outbound::CommandLong { target: (1, 1), command: 65000, params: [0.0; 7] }), None, "a command the dialect cannot name is not sent");
        let long = encode(3, &Outbound::CommandLong { target: (1, 1), command: 22, params: [-1.0, 0.0, 0.0, f64::NAN, f64::NAN, f64::NAN, 515.0] }).unwrap();
        let (header, message) = decode(&long);
        assert_eq!((header.system_id, header.component_id, header.sequence), (255, 190, 3));
        let MavMessage::COMMAND_LONG(c) = message else { panic!() };
        assert_eq!((c.command, c.target_system, c.param1, c.param7), (MavCmd::MAV_CMD_NAV_TAKEOFF, 1, -1.0, 515.0));
        assert!(c.param4.is_nan());
        let int = encode(4, &Outbound::CommandInt { target: (1, 1), command: 192, frame: 0, params: [-1.0, 1.0, 0.0, f64::NAN, 47.4, 8.5, 500.0], x: 474000000, y: 85000000 }).unwrap();
        let MavMessage::COMMAND_INT(c) = decode(&int).1 else { panic!() };
        assert_eq!((c.command, c.x, c.y, c.z, c.frame), (MavCmd::MAV_CMD_DO_REPOSITION, 474000000, 85000000, 500.0, MavFrame::MAV_FRAME_GLOBAL));
        let target = encode(5, &Outbound::PositionTargetLocalNed { target: (1, 1), frame: 7, type_mask: 0xFFF8, x: 0.0, y: 0.0, z: -3.0 }).unwrap();
        let MavMessage::SET_POSITION_TARGET_LOCAL_NED(t) = decode(&target).1 else { panic!() };
        assert_eq!((t.type_mask.bits(), t.z, t.coordinate_frame), (0xFFF8, -3.0, MavFrame::MAV_FRAME_LOCAL_OFFSET_NED));
        let item = encode(6, &Outbound::GuidedMissionItem { target: (1, 1), latitude: 47.4, longitude: 8.5, altitude_relative: 20.0 }).unwrap();
        let MavMessage::MISSION_ITEM(i) = decode(&item).1 else { panic!() };
        assert_eq!((i.seq, i.current, i.autocontinue, i.frame, i.z), (0, 2, 1, MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT, 20.0));
        let MavMessage::PARAM_REQUEST_LIST(l) = decode(&encode(7, &Outbound::ParamRequestList { target: (1, 0) }).unwrap()).1 else { panic!() };
        assert_eq!((l.target_system, l.target_component), (1, 0));
        let MavMessage::PARAM_REQUEST_READ(r) = decode(&encode(8, &Outbound::ParamRequestRead { target: (1, 1), name: Some("WPNAV_SPEED".into()), index: 0 }).unwrap()).1 else { panic!() };
        assert_eq!((r.param_index, r.param_id.to_str().unwrap()), (-1, "WPNAV_SPEED"));
        let MavMessage::PARAM_SET(p) = decode(&encode(9, &Outbound::ParamSet { target: (1, 1), name: "RTL_ALT".into(), bits: 1500.0, param_type: 9 }).unwrap()).1 else { panic!() };
        assert_eq!((p.param_value, p.param_type), (1500.0, MavParamType::MAV_PARAM_TYPE_REAL32));
    }

    #[test]
    fn set_mode_carries_raw_base_mode_bits_the_dialect_enum_cannot_name() {
        let bytes = encode(9, &Outbound::SetMode { system: 1, base_mode: 0x81, custom_mode: 0x0402_0000 }).unwrap();
        assert_eq!(bytes[0], 0xFD);
        assert_eq!(bytes[1], 6, "the payload keeps its full length because the last byte is not zero");
        assert_eq!(&bytes[7..10], &[11, 0, 0]);
        assert_eq!(&bytes[10..16], &[0, 0, 2, 4, 1, 0x81]);
        let expected = mavlink::calculate_crc(&bytes[1..16], 89);
        assert_eq!(u16::from_le_bytes([bytes[16], bytes[17]]), expected);
    }
    #[test]
    fn every_command_and_frame_the_core_can_send_is_the_one_the_dialect_names() {
        use mavlink::dialects::ardupilotmega::{MavCmd, MavFrame};

        [
            (crate::gimbal::CMD_DO_GIMBAL_MANAGER_PITCHYAW, MavCmd::MAV_CMD_DO_GIMBAL_MANAGER_PITCHYAW),
            (crate::gimbal::CMD_DO_GIMBAL_MANAGER_CONFIGURE, MavCmd::MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE),
            (crate::sensorcal::CMD_PREFLIGHT_CALIBRATION, MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION),
            (crate::sensorcal::CMD_DO_START_MAG_CAL, MavCmd::MAV_CMD_DO_START_MAG_CAL),
            (crate::sensorcal::CMD_DO_CANCEL_MAG_CAL, MavCmd::MAV_CMD_DO_CANCEL_MAG_CAL),
            (crate::sensorcal::CMD_ACCELCAL_VEHICLE_POS, MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS),
            (crate::structurescan::CMD_DO_SET_ROI_WPNEXT_OFFSET, MavCmd::MAV_CMD_DO_SET_ROI_WPNEXT_OFFSET),
            (crate::structurescan::CMD_DO_SET_ROI_NONE, MavCmd::MAV_CMD_DO_SET_ROI_NONE),
            (crate::surveyitems::CMD_NAV_WAYPOINT, MavCmd::MAV_CMD_NAV_WAYPOINT),
            (crate::surveyitems::CMD_DO_SET_CAM_TRIGG_DIST, MavCmd::MAV_CMD_DO_SET_CAM_TRIGG_DIST),
            (crate::plantransfer::CMD_FENCE_RETURN_POINT, MavCmd::MAV_CMD_NAV_FENCE_RETURN_POINT),
            (crate::plantransfer::CMD_FENCE_POLYGON_INCLUSION, MavCmd::MAV_CMD_NAV_FENCE_POLYGON_VERTEX_INCLUSION),
            (crate::plantransfer::CMD_FENCE_POLYGON_EXCLUSION, MavCmd::MAV_CMD_NAV_FENCE_POLYGON_VERTEX_EXCLUSION),
        ]
        .iter()
        .for_each(|(number, named)| {
            assert_eq!(MavCmd::from_u32(*number as u32), Some(*named), "{number} is what goes on the wire and {named:?} is what it is meant to mean");
        });

        [
            (crate::surveyitems::FRAME_GLOBAL, MavFrame::MAV_FRAME_GLOBAL),
            (crate::surveyitems::FRAME_MISSION, MavFrame::MAV_FRAME_MISSION),
            (crate::surveyitems::FRAME_GLOBAL_RELATIVE_ALT, MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT),
            (crate::surveyitems::FRAME_GLOBAL_TERRAIN_ALT, MavFrame::MAV_FRAME_GLOBAL_TERRAIN_ALT),
        ]
        .iter()
        .for_each(|(number, named)| {
            assert_eq!(MavFrame::from_u8(*number), Some(*named), "a wrong frame reinterprets an altitude rather than rejecting it, which is the kind of mistake that flies");
        });
    }

    #[test]
    fn every_message_id_the_core_asks_for_is_the_one_the_dialect_assigns() {
        use mavlink::dialects::ardupilotmega::{AUTOPILOT_VERSION_DATA, CAMERA_CAPTURE_STATUS_DATA, CAMERA_INFORMATION_DATA, CAMERA_SETTINGS_DATA, COMPONENT_INFORMATION_DATA, COMPONENT_METADATA_DATA, GIMBAL_DEVICE_ATTITUDE_STATUS_DATA, GIMBAL_MANAGER_INFORMATION_DATA, GIMBAL_MANAGER_STATUS_DATA, STORAGE_INFORMATION_DATA, VIDEO_STREAM_INFORMATION_DATA, VIDEO_STREAM_STATUS_DATA};

        [
            (crate::cameraproto::MSG_CAMERA_INFORMATION, CAMERA_INFORMATION_DATA::ID, "CAMERA_INFORMATION"),
            (crate::cameraproto::MSG_CAMERA_SETTINGS, CAMERA_SETTINGS_DATA::ID, "CAMERA_SETTINGS"),
            (crate::cameraproto::MSG_STORAGE_INFORMATION, STORAGE_INFORMATION_DATA::ID, "STORAGE_INFORMATION"),
            (crate::cameraproto::MSG_CAMERA_CAPTURE_STATUS, CAMERA_CAPTURE_STATUS_DATA::ID, "CAMERA_CAPTURE_STATUS"),
            (crate::cameraproto::MSG_VIDEO_STREAM_INFORMATION, VIDEO_STREAM_INFORMATION_DATA::ID, "VIDEO_STREAM_INFORMATION"),
            (crate::cameraproto::MSG_VIDEO_STREAM_STATUS, VIDEO_STREAM_STATUS_DATA::ID, "VIDEO_STREAM_STATUS"),
            (crate::compmeta::MSG_COMPONENT_METADATA, COMPONENT_METADATA_DATA::ID, "COMPONENT_METADATA"),
            (crate::compmeta::MSG_COMPONENT_INFORMATION, COMPONENT_INFORMATION_DATA::ID, "COMPONENT_INFORMATION"),
            (crate::connect::MSG_AUTOPILOT_VERSION, AUTOPILOT_VERSION_DATA::ID, "AUTOPILOT_VERSION"),
            (crate::gimbal::MSG_GIMBAL_MANAGER_INFORMATION, GIMBAL_MANAGER_INFORMATION_DATA::ID, "GIMBAL_MANAGER_INFORMATION"),
            (crate::gimbal::MSG_GIMBAL_MANAGER_STATUS, GIMBAL_MANAGER_STATUS_DATA::ID, "GIMBAL_MANAGER_STATUS"),
            (crate::gimbal::MSG_GIMBAL_DEVICE_ATTITUDE_STATUS, GIMBAL_DEVICE_ATTITUDE_STATUS_DATA::ID, "GIMBAL_DEVICE_ATTITUDE_STATUS"),
        ]
        .iter()
        .for_each(|(ours, dialect, name)| {
            assert_eq!(ours, dialect, "the core asks for {name} by number, and asking for the wrong one waits forever for a message nothing will send");
        });
    }

    #[test]
    fn the_bit_and_enum_constants_agree_with_the_dialect_too() {
        use mavlink::dialects::ardupilotmega::{GimbalDeviceCapFlags, MavAutopilot, MavMissionType, MavProtocolCapability, MavResult, MavType};

        assert_eq!(crate::connect::CAP_MISSION_INT, MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_MISSION_INT.bits() as u64);
        assert_eq!(crate::connect::CAP_COMMAND_INT, MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_COMMAND_INT.bits() as u64);
        assert_eq!(crate::connect::CAP_MAVLINK2, MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_MAVLINK2.bits() as u64);
        assert_eq!(crate::connect::CAP_MISSION_FENCE, MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_MISSION_FENCE.bits() as u64);
        assert_eq!(crate::gimbal::CAP_HAS_RETRACT, GimbalDeviceCapFlags::GIMBAL_DEVICE_CAP_FLAGS_HAS_RETRACT.bits() as u32);
        assert_eq!(crate::gimbal::CAP_HAS_YAW_LOCK, GimbalDeviceCapFlags::GIMBAL_DEVICE_CAP_FLAGS_HAS_YAW_LOCK.bits() as u32);

        assert_eq!(crate::plantransfer::PLAN_MISSION, MavMissionType::MAV_MISSION_TYPE_MISSION as u8);
        assert_eq!(crate::plantransfer::PLAN_FENCE, MavMissionType::MAV_MISSION_TYPE_FENCE as u8);
        assert_eq!(crate::plantransfer::PLAN_RALLY, MavMissionType::MAV_MISSION_TYPE_RALLY as u8);

        assert_eq!(crate::mavcmd::RESULT_ACCEPTED, MavResult::MAV_RESULT_ACCEPTED as u8);
        assert_eq!(crate::cameraproto::RESULT_TEMPORARILY_REJECTED, MavResult::MAV_RESULT_TEMPORARILY_REJECTED as u8);
        assert_eq!(crate::cameraproto::RESULT_DENIED, MavResult::MAV_RESULT_DENIED as u8);
        assert_eq!(crate::cameraproto::RESULT_UNSUPPORTED, MavResult::MAV_RESULT_UNSUPPORTED as u8);

        assert_eq!(crate::modes::AUTOPILOT_ARDUPILOT, MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA as u8);
        assert_eq!(crate::modes::AUTOPILOT_PX4, MavAutopilot::MAV_AUTOPILOT_PX4 as u8);
        assert_eq!(crate::hub::TYPE_GCS, MavType::MAV_TYPE_GCS as u8);
        assert_eq!(crate::hub::TYPE_ONBOARD_CONTROLLER, MavType::MAV_TYPE_ONBOARD_CONTROLLER as u8);
        assert_eq!(crate::hub::TYPE_GIMBAL, MavType::MAV_TYPE_GIMBAL as u8);
        assert_eq!(crate::hub::TYPE_ADSB, MavType::MAV_TYPE_ADSB as u8);
    }

    #[test]
    fn a_constant_defined_in_two_modules_has_the_same_value_in_both() {
        const DELIBERATELY_PER_MODULE: &[(&str, &str)] = &[
            ("ACK_TIMEOUT_MS", "a command ack and a mission item ack wait for different protocols"),
            ("MAX_RETRY", "ftp and command retries are three, mission transfer allows five"),
            ("STALE_MS", "how long a reading stays fresh is per subsystem"),
            ("STALE_AFTER_MS", "same, and gpsrtk derives its own from its receive budget"),
            ("CONNECT_TIMEOUT", "a tcp connect and a feed reconnect are different waits"),
            ("DEPS", "every view declares its own; the name is a convention rather than a shared value"),
            ("FIELDS", "same, the field list a module asks the bridge for"),
            ("CAMERA_DEPS", "same"),
            ("BUNDLED", "each module bundles the QGC file it parses"),
            ("UNKNOWN", "each module's own sentinel for a value it could not read"),
        ];

        let defined: Vec<(String, String, String)> = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|e| e == "rs"))
            .filter_map(|path| {
                let source = std::fs::read_to_string(&path).ok()?;
                let module = path.file_stem()?.to_string_lossy().to_string();
                Some(source
                    .split("#[cfg(test)]")
                    .next()?
                    .lines()
                    .filter_map(|line| {
                        let rest = line.strip_prefix("pub const ").or_else(|| line.strip_prefix("const "))?;
                        let (name, tail) = rest.split_once(':')?;
                        let value = tail.split_once('=')?.1.strip_suffix(';')?.trim().to_string();
                        name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_').then(|| (name.to_string(), module.clone(), value))
                    })
                    .collect::<Vec<_>>())
            })
            .flatten()
            .collect();

        assert!(defined.len() > 200, "only {} constants parsed, so a clean result would mean nothing", defined.len());

        let disagreeing: Vec<String> = defined
            .iter()
            .filter(|(name, _, _)| !DELIBERATELY_PER_MODULE.iter().any(|(excused, _)| excused == name))
            .filter_map(|(name, module, value)| {
                let others: Vec<&(String, String, String)> = defined.iter().filter(|(n, m, _)| n == name && m != module).collect();
                others
                    .iter()
                    .find(|(_, _, other)| other != value)
                    .map(|(_, other_module, other)| format!("{name} is {value} in {module} and {other} in {other_module}"))
            })
            .collect();

        assert!(
            disagreeing.is_empty(),
            "one of the copies is checked against MAVLink or a QGC header and the others ride on agreeing with it, so a divergence silently unpins whichever copy is not the checked one: {}",
            disagreeing.join("; ")
        );
    }

    #[test]
    fn the_camera_parameter_types_and_the_aliased_enums_resolve_too() {
        use mavlink::dialects::ardupilotmega::{MavAutopilot, MavParamExtType, MavType};

        [
            (crate::cameradef::PARAM_EXT_TYPE_UINT8, MavParamExtType::MAV_PARAM_EXT_TYPE_UINT8),
            (crate::cameradef::PARAM_EXT_TYPE_INT8, MavParamExtType::MAV_PARAM_EXT_TYPE_INT8),
            (crate::cameradef::PARAM_EXT_TYPE_UINT16, MavParamExtType::MAV_PARAM_EXT_TYPE_UINT16),
            (crate::cameradef::PARAM_EXT_TYPE_INT16, MavParamExtType::MAV_PARAM_EXT_TYPE_INT16),
            (crate::cameradef::PARAM_EXT_TYPE_UINT32, MavParamExtType::MAV_PARAM_EXT_TYPE_UINT32),
            (crate::cameradef::PARAM_EXT_TYPE_INT32, MavParamExtType::MAV_PARAM_EXT_TYPE_INT32),
            (crate::cameradef::PARAM_EXT_TYPE_UINT64, MavParamExtType::MAV_PARAM_EXT_TYPE_UINT64),
            (crate::cameradef::PARAM_EXT_TYPE_INT64, MavParamExtType::MAV_PARAM_EXT_TYPE_INT64),
            (crate::cameradef::PARAM_EXT_TYPE_REAL32, MavParamExtType::MAV_PARAM_EXT_TYPE_REAL32),
            (crate::cameradef::PARAM_EXT_TYPE_REAL64, MavParamExtType::MAV_PARAM_EXT_TYPE_REAL64),
            (crate::cameradef::PARAM_EXT_TYPE_CUSTOM, MavParamExtType::MAV_PARAM_EXT_TYPE_CUSTOM),
        ]
        .iter()
        .for_each(|(ours, named)| {
            assert_eq!(*ours, *named as u8, "this decides how a camera definition parameter's bytes are read, so a wrong one misreads the setting rather than refusing it");
        });

        assert_eq!(crate::debugapi::AUTOPILOT_ARDUPILOTMEGA, MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA as i64, "same value as modes::AUTOPILOT_ARDUPILOT under a different name, so the duplicate-name check cannot see it");
        assert_eq!(crate::debugapi::AUTOPILOT_PX4, MavAutopilot::MAV_AUTOPILOT_PX4 as i64);
        assert_eq!(crate::debugapi::VEHICLE_TYPE_QUADROTOR, MavType::MAV_TYPE_QUADROTOR as i64);
    }

}
