#[allow(deprecated)]
use mavlink::dialects::ardupilotmega::{COMMAND_INT_DATA, COMMAND_LONG_DATA, FILE_TRANSFER_PROTOCOL_DATA, MISSION_ACK_DATA, MISSION_COUNT_DATA, MISSION_ITEM_DATA, MISSION_ITEM_INT_DATA, MISSION_REQUEST_INT_DATA, MISSION_REQUEST_LIST_DATA, MavCmd, MavMissionResult, MavMissionType, MavFrame, MavMessage, MavParamType, PARAM_REQUEST_LIST_DATA, PARAM_REQUEST_READ_DATA, PARAM_SET_DATA, PositionTargetTypemask, SET_POSITION_TARGET_LOCAL_NED_DATA};
use mavlink::types::CharArray;
use mavlink::{MAVLinkV2MessageRaw, MavHeader, MavlinkVersion, MessageData};
use num_traits::FromPrimitive;
use std::sync::atomic::{AtomicU8, Ordering};

pub const GCS_SYSTEM: u8 = 255;
pub const GCS_COMPONENT: u8 = 190;
pub const GUIDED_ITEM_CURRENT: u8 = 2;
pub const MISSION: MavMissionType = MavMissionType::MAV_MISSION_TYPE_MISSION;

static SEQUENCE: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, PartialEq)]
pub enum Outbound {
    CommandLong { target: (u8, u8), command: u16, params: [f64; 7] },
    CommandInt { target: (u8, u8), command: u16, frame: u8, params: [f64; 7], x: i32, y: i32 },
    SetMode { system: u8, base_mode: u8, custom_mode: u32 },
    PositionTargetLocalNed { target: (u8, u8), frame: u8, type_mask: u16, x: f64, y: f64, z: f64 },
    GuidedMissionItem { target: (u8, u8), latitude: f64, longitude: f64, altitude_relative: f64 },
    ParamRequestList { target: (u8, u8) },
    ParamRequestRead { target: (u8, u8), name: Option<String>, index: i16 },
    ParamSet { target: (u8, u8), name: String, bits: f32, param_type: u8 },
    Ftp { target: (u8, u8), payload: [u8; 251] },
    MissionRequestList { target: (u8, u8) },
    MissionRequestInt { target: (u8, u8), seq: u16 },
    MissionCount { target: (u8, u8), count: u16 },
    MissionItemInt { target: (u8, u8), item: crate::plantransfer::Item },
    MissionAck { target: (u8, u8), result: u8 },
}

pub fn command_known(command: u16) -> bool {
    MavCmd::from_u32(command as u32).is_some()
}

pub fn frame_known(frame: u8) -> bool {
    MavFrame::from_u8(frame).is_some()
}

pub fn param_id(name: &str) -> CharArray<16> {
    let bytes = name.as_bytes();
    CharArray::from(std::array::from_fn(|i| bytes.get(i).copied().unwrap_or(0)))
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
        Outbound::SetMode { .. } => None,
        Outbound::ParamRequestList { target } => Some(MavMessage::PARAM_REQUEST_LIST(PARAM_REQUEST_LIST_DATA { target_system: target.0, target_component: target.1 })),
        Outbound::ParamRequestRead { target, name, index } => Some(MavMessage::PARAM_REQUEST_READ(PARAM_REQUEST_READ_DATA { param_index: if name.is_some() { -1 } else { *index }, target_system: target.0, target_component: target.1, param_id: param_id(name.as_deref().unwrap_or("")) })),
        Outbound::MissionRequestList { target } => Some(MavMessage::MISSION_REQUEST_LIST(MISSION_REQUEST_LIST_DATA { target_system: target.0, target_component: target.1, mission_type: MISSION })),
        Outbound::MissionRequestInt { target, seq } => Some(MavMessage::MISSION_REQUEST_INT(MISSION_REQUEST_INT_DATA { seq: *seq, target_system: target.0, target_component: target.1, mission_type: MISSION })),
        Outbound::MissionCount { target, count } => Some(MavMessage::MISSION_COUNT(MISSION_COUNT_DATA { count: *count, target_system: target.0, target_component: target.1, mission_type: MISSION, opaque_id: 0 })),
        Outbound::MissionAck { target, result } => Some(MavMessage::MISSION_ACK(MISSION_ACK_DATA { target_system: target.0, target_component: target.1, mavtype: MavMissionResult::from_u8(*result)?, mission_type: MISSION, opaque_id: 0 })),
        Outbound::MissionItemInt { target, item } => {
            let scale = |v: f64| if item.frame == crate::plantransfer::FRAME_MISSION { v as i32 } else { (v * 1e7).round() as i32 };
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
                mission_type: MISSION,
            }))
        }
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
    let header = MavHeader { system_id: GCS_SYSTEM, component_id: GCS_COMPONENT, sequence };
    let mut raw = MAVLinkV2MessageRaw::new();
    match send {
        Outbound::SetMode { system, base_mode, custom_mode } => raw.serialize_message_data(header, &SetModeBits { system: *system, base_mode: *base_mode, custom_mode: *custom_mode }),
        other => raw.serialize_message(header, &message(other)?),
    }
    Some(raw.raw_bytes().to_vec())
}

pub fn encode_next(send: &Outbound) -> Option<Vec<u8>> {
    encode(SEQUENCE.fetch_add(1, Ordering::Relaxed), send)
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use mavlink::{ReadVersion, read_versioned_msg};

    fn decode(bytes: &[u8]) -> (MavHeader, MavMessage) {
        read_versioned_msg::<MavMessage, _>(&mut mavlink::peek_reader::PeekReader::new(bytes), ReadVersion::Single(MavlinkVersion::V2)).unwrap()
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
}
