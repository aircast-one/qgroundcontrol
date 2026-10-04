#![allow(deprecated)]
use mavlink::dialects::ardupilotmega::*;
use num_traits::FromPrimitive;

use crate::mocklink::Out;

pub const MOCK_CAMERA: u8 = 100;
const MOCK_CAMERA2: u8 = 101;
const STREAMS: u8 = 2;
const STORAGE_TOTAL_MIB: f32 = 16384.0;
const STORAGE_FREE_MIB: f32 = 8192.0;
const SINGLE_SHOT_MS: u64 = 500;
const IMAGE_IDLE: u8 = 0;
const IMAGE_IN_PROGRESS: u8 = 1;
const IMAGE_INTERVAL_CAPTURE: u8 = 3;
const DRIFT: f32 = 0.05;
const DEFAULT_TRACKING_INTERVAL_US: i64 = -1;
const CAMERA_INFORMATION: u32 = 259;
const CAMERA_SETTINGS: u32 = 260;
const STORAGE_INFORMATION: u32 = 261;
const CAMERA_CAPTURE_STATUS: u32 = 262;
const VIDEO_STREAM_INFORMATION: u32 = 269;
const VIDEO_STREAM_STATUS: u32 = 270;
const CAMERA_TRACKING_IMAGE_STATUS: u32 = 275;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    None,
    RtpUdpH264,
    RtpUdpH265,
    RtspH264,
    MpegTsUdp,
    MpegTsTcp,
}

pub const STREAM_KINDS: [(StreamKind, &str); 6] = [
    (StreamKind::None, "Disabled"),
    (StreamKind::RtpUdpH264, "RTP/UDP H.264"),
    (StreamKind::RtpUdpH265, "RTP/UDP H.265"),
    (StreamKind::RtspH264, "RTSP (H.264)"),
    (StreamKind::MpegTsUdp, "MPEG-TS (UDP)"),
    (StreamKind::MpegTsTcp, "MPEG-TS (TCP)"),
];

impl StreamKind {
    pub fn from_index(index: i64) -> StreamKind {
        usize::try_from(index).ok().and_then(|at| STREAM_KINDS.get(at)).map_or(StreamKind::None, |(kind, _)| *kind)
    }

    pub fn index(self) -> i64 {
        STREAM_KINDS.iter().position(|(kind, _)| *kind == self).unwrap_or(0) as i64
    }

    pub fn port(self) -> u16 {
        match self {
            StreamKind::RtpUdpH265 => 5601,
            StreamKind::RtspH264 => 8554,
            _ => 5600,
        }
    }

    fn wire(self) -> (VideoStreamType, VideoStreamEncoding) {
        match self {
            StreamKind::RtpUdpH265 => (VideoStreamType::VIDEO_STREAM_TYPE_RTPUDP, VideoStreamEncoding::VIDEO_STREAM_ENCODING_H265),
            StreamKind::RtspH264 => (VideoStreamType::VIDEO_STREAM_TYPE_RTSP, VideoStreamEncoding::VIDEO_STREAM_ENCODING_H264),
            StreamKind::MpegTsUdp => (VideoStreamType::VIDEO_STREAM_TYPE_MPEG_TS, VideoStreamEncoding::VIDEO_STREAM_ENCODING_H264),
            StreamKind::MpegTsTcp => (VideoStreamType::VIDEO_STREAM_TYPE_TCP_MPEG, VideoStreamEncoding::VIDEO_STREAM_ENCODING_H264),
            _ => (VideoStreamType::VIDEO_STREAM_TYPE_RTPUDP, VideoStreamEncoding::VIDEO_STREAM_ENCODING_H264),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Video {
    pub requested: StreamKind,
    pub served: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct Tracking {
    mode: CameraTrackingMode,
    point: (f32, f32, f32),
    rect: (f32, f32, f32, f32),
    anchor: (f32, f32),
    started_ms: u64,
    interval_us: i64,
    last_sent_ms: Option<u64>,
}

impl Tracking {
    fn idle() -> Tracking {
        Tracking { mode: CameraTrackingMode::CAMERA_TRACKING_MODE_NONE, point: (0.0, 0.0, 0.0), rect: (0.0, 0.0, 0.0, 0.0), anchor: (0.0, 0.0), started_ms: 0, interval_us: DEFAULT_TRACKING_INTERVAL_US, last_sent_ms: None }
    }

    fn drifted(&self, now_ms: u64) -> Tracking {
        let elapsed = now_ms.saturating_sub(self.started_ms) as f64 / 1000.0;
        let (dx, dy) = (DRIFT * (elapsed * 0.7).sin() as f32, DRIFT * (elapsed * 1.1).sin() as f32);
        let (ax, ay) = self.anchor;
        match self.mode {
            CameraTrackingMode::CAMERA_TRACKING_MODE_POINT => Tracking { point: ((ax + dx).clamp(0.0, 1.0), (ay + dy).clamp(0.0, 1.0), self.point.2), ..self.clone() },
            _ => {
                let (half_w, half_h) = ((self.rect.2 - self.rect.0) / 2.0, (self.rect.3 - self.rect.1) / 2.0);
                let (cx, cy) = ((ax + dx).clamp(half_w, 1.0 - half_w), (ay + dy).clamp(half_h, 1.0 - half_h));
                Tracking { rect: (cx - half_w, cy - half_h, cx + half_w, cy + half_h), ..self.clone() }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Camera {
    component: u8,
    flags: CameraCapFlags,
    mode: CameraMode,
    recording: bool,
    images: i32,
    zoom: f32,
    focus: f32,
    image_status: u8,
    interval: f32,
    shot_started_ms: Option<u64>,
    tracking: Tracking,
}

impl Camera {
    fn new(component: u8, flags: CameraCapFlags) -> Camera {
        let video_first = flags.contains(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_VIDEO_STREAM | CameraCapFlags::CAMERA_CAP_FLAGS_HAS_MODES);
        Camera {
            component,
            flags,
            mode: if video_first { CameraMode::CAMERA_MODE_VIDEO } else { CameraMode::CAMERA_MODE_IMAGE },
            recording: false,
            images: 0,
            zoom: 1.0,
            focus: 0.0,
            image_status: IMAGE_IDLE,
            interval: 0.0,
            shot_started_ms: None,
            tracking: Tracking::idle(),
        }
    }

    fn has(&self, flag: CameraCapFlags) -> bool {
        self.flags.contains(flag)
    }

    fn reset_capture(&mut self) {
        self.image_status = IMAGE_IDLE;
        self.interval = 0.0;
        self.shot_started_ms = None;
    }
}

pub struct Cameras {
    cameras: Vec<Camera>,
    video: Video,
}

pub struct Where {
    pub lat: i32,
    pub lon: i32,
    pub alt_mm: i32,
}

fn chars<const N: usize>(text: &str) -> mavlink::types::CharArray<N> {
    crate::mavout::chars(text)
}

impl Cameras {
    pub fn new(video: Video) -> Cameras {
        let full = CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_VIDEO
            | CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_IMAGE
            | CameraCapFlags::CAMERA_CAP_FLAGS_HAS_MODES
            | CameraCapFlags::CAMERA_CAP_FLAGS_HAS_VIDEO_STREAM
            | CameraCapFlags::CAMERA_CAP_FLAGS_CAN_CAPTURE_IMAGE_IN_VIDEO_MODE
            | CameraCapFlags::CAMERA_CAP_FLAGS_HAS_BASIC_ZOOM
            | CameraCapFlags::CAMERA_CAP_FLAGS_HAS_TRACKING_POINT
            | CameraCapFlags::CAMERA_CAP_FLAGS_HAS_TRACKING_RECTANGLE;
        Cameras { cameras: vec![Camera::new(MOCK_CAMERA, full), Camera::new(MOCK_CAMERA2, CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_IMAGE)], video }
    }

    fn find(&mut self, component: u8) -> Option<&mut Camera> {
        self.cameras.iter_mut().find(|c| c.component == component)
    }

    pub fn heartbeats(&self) -> Vec<Out> {
        self.cameras
            .iter()
            .map(|camera| (camera.component, MavMessage::HEARTBEAT(HEARTBEAT_DATA { mavtype: MavType::MAV_TYPE_CAMERA, autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID, system_status: MavState::MAV_STATE_ACTIVE, mavlink_version: 3, ..Default::default() })))
            .collect()
    }

    pub fn tick(&mut self, now_ms: u64, at: &Where) -> Vec<Out> {
        self.cameras
            .iter_mut()
            .flat_map(|camera| {
                let shot_done = camera.shot_started_ms.is_some_and(|started| now_ms.saturating_sub(started) >= SINGLE_SHOT_MS);
                let shot = shot_done
                    .then(|| {
                        camera.images += 1;
                        camera.reset_capture();
                        vec![image_captured(camera, at), capture_status(camera)]
                    })
                    .unwrap_or_default();
                let due = camera.tracking.mode != CameraTrackingMode::CAMERA_TRACKING_MODE_NONE
                    && camera.tracking.interval_us > 0
                    && camera.tracking.last_sent_ms.is_none_or(|last| now_ms.saturating_sub(last) >= (camera.tracking.interval_us as u64).div_ceil(1000));
                let tracked = due
                    .then(|| {
                        camera.tracking = Tracking { last_sent_ms: Some(now_ms), ..camera.tracking.drifted(now_ms) };
                        vec![tracking_status(camera)]
                    })
                    .unwrap_or_default();
                shot.into_iter().chain(tracked).collect::<Vec<_>>()
            })
            .collect()
    }

    pub fn command(&mut self, now_ms: u64, from: (u8, u8), component: u8, command: MavCmd, params: [f32; 7]) -> Option<Vec<Out>> {
        let video = self.video.clone();
        let camera = self.find(component)?;
        let ack = |result: MavResult| camera_ack(component, from, command, result);
        let replies = match command {
            MavCmd::MAV_CMD_REQUEST_MESSAGE => {
                let asked = params[0] as u32;
                let stream = params[1] as u8;
                let streams = |send: fn(&Camera, &Video, u8) -> Out| -> Vec<Out> {
                    match stream {
                        0 => (1..=STREAMS).map(|s| send(camera, &video, s)).collect(),
                        s => vec![send(camera, &video, s)],
                    }
                };
                let has_stream = camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_VIDEO_STREAM);
                match asked {
                    CAMERA_INFORMATION => vec![information(camera), ack(MavResult::MAV_RESULT_ACCEPTED)],
                    CAMERA_SETTINGS => vec![settings(camera), ack(MavResult::MAV_RESULT_ACCEPTED)],
                    STORAGE_INFORMATION => vec![storage(camera), ack(MavResult::MAV_RESULT_ACCEPTED)],
                    CAMERA_CAPTURE_STATUS => vec![capture_status(camera), ack(MavResult::MAV_RESULT_ACCEPTED)],
                    VIDEO_STREAM_INFORMATION if has_stream => streams(stream_information).into_iter().chain([ack(MavResult::MAV_RESULT_ACCEPTED)]).collect(),
                    VIDEO_STREAM_STATUS if has_stream => streams(stream_status).into_iter().chain([ack(MavResult::MAV_RESULT_ACCEPTED)]).collect(),
                    _ => vec![ack(MavResult::MAV_RESULT_DENIED)],
                }
            }
            MavCmd::MAV_CMD_REQUEST_CAMERA_INFORMATION => vec![information(camera), ack(MavResult::MAV_RESULT_ACCEPTED)],
            MavCmd::MAV_CMD_REQUEST_CAMERA_SETTINGS => vec![settings(camera), ack(MavResult::MAV_RESULT_ACCEPTED)],
            MavCmd::MAV_CMD_REQUEST_STORAGE_INFORMATION => vec![storage(camera), ack(MavResult::MAV_RESULT_ACCEPTED)],
            MavCmd::MAV_CMD_REQUEST_CAMERA_CAPTURE_STATUS => vec![capture_status(camera), ack(MavResult::MAV_RESULT_ACCEPTED)],
            MavCmd::MAV_CMD_REQUEST_VIDEO_STREAM_INFORMATION | MavCmd::MAV_CMD_REQUEST_VIDEO_STREAM_STATUS => {
                let send: fn(&Camera, &Video, u8) -> Out = if command == MavCmd::MAV_CMD_REQUEST_VIDEO_STREAM_INFORMATION { stream_information } else { stream_status };
                match (camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_VIDEO_STREAM), params[0] as u8) {
                    (false, _) => vec![ack(MavResult::MAV_RESULT_DENIED)],
                    (true, 0) => (1..=STREAMS).map(|s| send(camera, &video, s)).chain([ack(MavResult::MAV_RESULT_ACCEPTED)]).collect(),
                    (true, s) => vec![send(camera, &video, s), ack(MavResult::MAV_RESULT_ACCEPTED)],
                }
            }
            MavCmd::MAV_CMD_SET_CAMERA_MODE => {
                let asked = CameraMode::from_u8(params[1] as u8).filter(|m| matches!(m, CameraMode::CAMERA_MODE_IMAGE | CameraMode::CAMERA_MODE_VIDEO));
                let streams = camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_VIDEO_STREAM);
                let supported = |mode: CameraMode| match mode {
                    CameraMode::CAMERA_MODE_IMAGE => camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_IMAGE) || streams,
                    _ => camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_VIDEO) || streams,
                };
                match asked.filter(|mode| camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_MODES) && supported(*mode)) {
                    Some(mode) => {
                        camera.mode = mode;
                        vec![ack(MavResult::MAV_RESULT_ACCEPTED), settings(camera)]
                    }
                    None => vec![ack(MavResult::MAV_RESULT_DENIED)],
                }
            }
            MavCmd::MAV_CMD_IMAGE_START_CAPTURE => {
                let blocked = camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_MODES) && camera.mode == CameraMode::CAMERA_MODE_VIDEO && !camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_CAN_CAPTURE_IMAGE_IN_VIDEO_MODE);
                match camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_IMAGE) && !blocked {
                    false => vec![ack(MavResult::MAV_RESULT_DENIED)],
                    true => {
                        let (interval, count) = (params[1], params[2] as i32);
                        match interval > 0.0 {
                            true => {
                                camera.image_status = IMAGE_INTERVAL_CAPTURE;
                                camera.interval = interval;
                                camera.images += count.max(1);
                                camera.shot_started_ms = None;
                            }
                            false => {
                                camera.image_status = IMAGE_IN_PROGRESS;
                                camera.interval = 0.0;
                                camera.shot_started_ms = Some(now_ms);
                            }
                        }
                        vec![ack(MavResult::MAV_RESULT_ACCEPTED), capture_status(camera)]
                    }
                }
            }
            MavCmd::MAV_CMD_IMAGE_STOP_CAPTURE => match camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_IMAGE) {
                true => {
                    camera.reset_capture();
                    vec![ack(MavResult::MAV_RESULT_ACCEPTED), capture_status(camera)]
                }
                false => vec![ack(MavResult::MAV_RESULT_DENIED)],
            },
            MavCmd::MAV_CMD_VIDEO_START_CAPTURE => {
                let blocked = camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_MODES) && camera.mode == CameraMode::CAMERA_MODE_IMAGE && !camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_CAN_CAPTURE_VIDEO_IN_IMAGE_MODE);
                match camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_VIDEO) && !blocked {
                    true => {
                        camera.recording = true;
                        vec![ack(MavResult::MAV_RESULT_ACCEPTED), capture_status(camera)]
                    }
                    false => vec![ack(MavResult::MAV_RESULT_DENIED)],
                }
            }
            MavCmd::MAV_CMD_VIDEO_STOP_CAPTURE => match camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_VIDEO) {
                true => {
                    camera.recording = false;
                    vec![ack(MavResult::MAV_RESULT_ACCEPTED), capture_status(camera)]
                }
                false => vec![ack(MavResult::MAV_RESULT_DENIED)],
            },
            MavCmd::MAV_CMD_STORAGE_FORMAT => {
                camera.images = 0;
                camera.reset_capture();
                vec![ack(MavResult::MAV_RESULT_ACCEPTED), storage(camera)]
            }
            MavCmd::MAV_CMD_SET_CAMERA_ZOOM => match camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_BASIC_ZOOM) {
                true => {
                    if params[0] as u32 == CameraZoomType::ZOOM_TYPE_RANGE as u32 {
                        camera.zoom = params[1];
                    }
                    vec![ack(MavResult::MAV_RESULT_ACCEPTED)]
                }
                false => vec![ack(MavResult::MAV_RESULT_DENIED)],
            },
            MavCmd::MAV_CMD_SET_CAMERA_FOCUS => vec![ack(MavResult::MAV_RESULT_DENIED)],
            MavCmd::MAV_CMD_RESET_CAMERA_SETTINGS => {
                camera.mode = CameraMode::CAMERA_MODE_IMAGE;
                camera.zoom = 1.0;
                camera.focus = 0.0;
                camera.reset_capture();
                vec![ack(MavResult::MAV_RESULT_ACCEPTED), settings(camera)]
            }
            MavCmd::MAV_CMD_CAMERA_TRACK_POINT => match camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_TRACKING_POINT) {
                true => {
                    camera.tracking = Tracking { mode: CameraTrackingMode::CAMERA_TRACKING_MODE_POINT, point: (params[0], params[1], params[2]), anchor: (params[0], params[1]), started_ms: now_ms, ..camera.tracking.clone() };
                    vec![ack(MavResult::MAV_RESULT_ACCEPTED)]
                }
                false => vec![ack(MavResult::MAV_RESULT_DENIED)],
            },
            MavCmd::MAV_CMD_CAMERA_TRACK_RECTANGLE => match camera.has(CameraCapFlags::CAMERA_CAP_FLAGS_HAS_TRACKING_RECTANGLE) {
                true => {
                    camera.tracking = Tracking {
                        mode: CameraTrackingMode::CAMERA_TRACKING_MODE_RECTANGLE,
                        rect: (params[0], params[1], params[2], params[3]),
                        anchor: ((params[0] + params[2]) / 2.0, (params[1] + params[3]) / 2.0),
                        started_ms: now_ms,
                        ..camera.tracking.clone()
                    };
                    vec![ack(MavResult::MAV_RESULT_ACCEPTED)]
                }
                false => vec![ack(MavResult::MAV_RESULT_DENIED)],
            },
            MavCmd::MAV_CMD_CAMERA_STOP_TRACKING => {
                camera.tracking = Tracking { mode: CameraTrackingMode::CAMERA_TRACKING_MODE_NONE, interval_us: DEFAULT_TRACKING_INTERVAL_US, last_sent_ms: None, ..camera.tracking.clone() };
                vec![ack(MavResult::MAV_RESULT_ACCEPTED)]
            }
            MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL => match params[0] as u32 {
                CAMERA_TRACKING_IMAGE_STATUS => {
                    camera.tracking = Tracking { interval_us: params[1] as i64, last_sent_ms: None, ..camera.tracking.clone() };
                    vec![ack(MavResult::MAV_RESULT_ACCEPTED)]
                }
                _ => vec![ack(MavResult::MAV_RESULT_UNSUPPORTED)],
            },
            _ => return None,
        };
        Some(replies)
    }
}

fn camera_ack(component: u8, from: (u8, u8), command: MavCmd, result: MavResult) -> Out {
    (component, MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command, result, progress: 0, result_param2: 0, target_system: from.0, target_component: from.1 }))
}

fn information(camera: &Camera) -> Out {
    let bytes = |text: &str| -> [u8; 32] { std::array::from_fn(|at| text.as_bytes().get(at).copied().unwrap_or(0)) };
    let model = format!("MockCam {}", camera.component - MOCK_CAMERA + 1);
    (camera.component, MavMessage::CAMERA_INFORMATION(CAMERA_INFORMATION_DATA { flags: camera.flags, vendor_name: bytes("MockLink"), model_name: bytes(&model), resolution_h: 1920, resolution_v: 1080, ..Default::default() }))
}

fn settings(camera: &Camera) -> Out {
    (camera.component, MavMessage::CAMERA_SETTINGS(CAMERA_SETTINGS_DATA { mode_id: camera.mode, zoomLevel: camera.zoom, focusLevel: camera.focus, ..Default::default() }))
}

fn storage(camera: &Camera) -> Out {
    (camera.component, MavMessage::STORAGE_INFORMATION(STORAGE_INFORMATION_DATA {
        storage_id: 1,
        storage_count: 1,
        status: StorageStatus::STORAGE_STATUS_READY,
        total_capacity: STORAGE_TOTAL_MIB,
        used_capacity: STORAGE_TOTAL_MIB - STORAGE_FREE_MIB,
        available_capacity: STORAGE_FREE_MIB,
        read_speed: f32::NAN,
        write_speed: f32::NAN,
        mavtype: StorageType::STORAGE_TYPE_SD,
        ..Default::default()
    }))
}

fn capture_status(camera: &Camera) -> Out {
    (camera.component, MavMessage::CAMERA_CAPTURE_STATUS(CAMERA_CAPTURE_STATUS_DATA {
        image_status: camera.image_status,
        video_status: u8::from(camera.recording),
        image_interval: camera.interval,
        available_capacity: STORAGE_FREE_MIB,
        image_count: camera.images,
        ..Default::default()
    }))
}

fn image_captured(camera: &Camera, at: &Where) -> Out {
    (camera.component, MavMessage::CAMERA_IMAGE_CAPTURED(CAMERA_IMAGE_CAPTURED_DATA {
        camera_id: camera.component - MOCK_CAMERA + 1,
        lat: at.lat,
        lon: at.lon,
        alt: at.alt_mm,
        relative_alt: at.alt_mm,
        q: [1.0, 0.0, 0.0, 0.0],
        image_index: camera.images,
        capture_result: MavBool::MAV_BOOL_TRUE,
        ..Default::default()
    }))
}

fn stream_flags(video: &Video) -> VideoStreamStatusFlags {
    match (video.requested, &video.served) {
        (StreamKind::None, _) | (_, Some(_)) => VideoStreamStatusFlags::VIDEO_STREAM_STATUS_FLAGS_RUNNING,
        _ => VideoStreamStatusFlags::empty(),
    }
}

fn stream_information(camera: &Camera, video: &Video, stream: u8) -> Out {
    let (kind, uri) = match (&video.served, video.requested) {
        (Some(uri), requested) => (requested, uri.clone()),
        (None, StreamKind::None) => (StreamKind::RtpUdpH264, "udp://127.0.0.1:5600".to_string()),
        (None, _) => (StreamKind::RtpUdpH264, String::new()),
    };
    let (mavtype, encoding) = kind.wire();
    (camera.component, MavMessage::VIDEO_STREAM_INFORMATION(VIDEO_STREAM_INFORMATION_DATA {
        framerate: 30.0,
        bitrate: 2000,
        flags: stream_flags(video),
        resolution_h: 1280,
        resolution_v: 720,
        rotation: 0,
        hfov: 70,
        stream_id: stream,
        count: STREAMS,
        mavtype,
        name: chars(&format!("Stream {}-{stream}", camera.component - MOCK_CAMERA + 1)),
        uri: chars(&uri),
        encoding,
        camera_device_id: 0,
    }))
}

fn stream_status(camera: &Camera, video: &Video, stream: u8) -> Out {
    (camera.component, MavMessage::VIDEO_STREAM_STATUS(VIDEO_STREAM_STATUS_DATA { framerate: 30.0, bitrate: 2000, flags: stream_flags(video), resolution_h: 1280, resolution_v: 720, rotation: 0, hfov: 70, stream_id: stream, camera_device_id: 0 }))
}

fn tracking_status(camera: &Camera) -> Out {
    let t = &camera.tracking;
    (camera.component, MavMessage::CAMERA_TRACKING_IMAGE_STATUS(CAMERA_TRACKING_IMAGE_STATUS_DATA {
        point_x: t.point.0,
        point_y: t.point.1,
        radius: t.point.2,
        rec_top_x: t.rect.0,
        rec_top_y: t.rect.1,
        rec_bottom_x: t.rect.2,
        rec_bottom_y: t.rect.3,
        tracking_status: CameraTrackingStatusFlags::CAMERA_TRACKING_STATUS_FLAGS_ACTIVE,
        tracking_mode: t.mode,
        target_data: CameraTrackingTargetData::CAMERA_TRACKING_TARGET_DATA_EMBEDDED,
        camera_device_id: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GCS: (u8, u8) = (255, 190);
    const HERE: Where = Where { lat: 1, lon: 2, alt_mm: 3 };

    fn cameras(requested: StreamKind, served: Option<&str>) -> Cameras {
        Cameras::new(Video { requested, served: served.map(str::to_string) })
    }

    fn acked(replies: &[Out]) -> Option<MavResult> {
        replies.iter().find_map(|(_, m)| match m {
            MavMessage::COMMAND_ACK(a) => Some(a.result),
            _ => None,
        })
    }

    #[test]
    fn two_cameras_answer_and_others_are_left_to_the_autopilot() {
        let mut set = cameras(StreamKind::None, None);
        assert_eq!(set.heartbeats().iter().map(|(c, _)| *c).collect::<Vec<_>>(), vec![MOCK_CAMERA, MOCK_CAMERA2]);
        let info = set.command(0, GCS, MOCK_CAMERA2, MavCmd::MAV_CMD_REQUEST_MESSAGE, [259.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]).unwrap();
        assert!(matches!(&info[0], (MOCK_CAMERA2, MavMessage::CAMERA_INFORMATION(i)) if i.flags == CameraCapFlags::CAMERA_CAP_FLAGS_CAPTURE_IMAGE));
        assert!(set.command(0, GCS, 102, MavCmd::MAV_CMD_REQUEST_MESSAGE, [259.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]).is_none());
        assert!(set.command(0, GCS, MOCK_CAMERA, MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, [1.0; 7]).is_none());
    }

    #[test]
    fn a_single_shot_completes_half_a_second_later_with_an_image_captured() {
        let mut set = cameras(StreamKind::None, None);
        let started = set.command(1000, GCS, MOCK_CAMERA, MavCmd::MAV_CMD_IMAGE_START_CAPTURE, [0.0; 7]).unwrap();
        assert_eq!(acked(&started), Some(MavResult::MAV_RESULT_ACCEPTED));
        assert!(set.tick(1400, &HERE).is_empty());
        let done = set.tick(1500, &HERE);
        assert!(matches!(&done[0], (_, MavMessage::CAMERA_IMAGE_CAPTURED(c)) if c.image_index == 1 && c.lat == 1));
        let video = set.command(0, GCS, MOCK_CAMERA2, MavCmd::MAV_CMD_VIDEO_START_CAPTURE, [0.0; 7]).unwrap();
        assert_eq!(acked(&video), Some(MavResult::MAV_RESULT_DENIED), "the second camera only takes photos");
    }

    #[test]
    fn the_served_stream_is_advertised_and_a_failed_server_clears_the_running_flag() {
        let mut served = cameras(StreamKind::RtspH264, Some("rtsp://127.0.0.1:8554/test"));
        let info = served.command(0, GCS, MOCK_CAMERA, MavCmd::MAV_CMD_REQUEST_MESSAGE, [269.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]).unwrap();
        assert!(matches!(&info[0], (_, MavMessage::VIDEO_STREAM_INFORMATION(s)) if s.mavtype == VideoStreamType::VIDEO_STREAM_TYPE_RTSP && s.uri.to_str().unwrap().starts_with("rtsp://") && !s.flags.is_empty()));
        let mut failed = cameras(StreamKind::RtpUdpH265, None);
        let both = failed.command(0, GCS, MOCK_CAMERA, MavCmd::MAV_CMD_REQUEST_VIDEO_STREAM_INFORMATION, [0.0; 7]).unwrap();
        assert_eq!(both.len(), 3);
        assert!(matches!(&both[0], (_, MavMessage::VIDEO_STREAM_INFORMATION(s)) if s.flags.is_empty() && s.uri.to_str().unwrap().trim_end_matches('\0').is_empty()));
    }

    #[test]
    fn tracking_reports_at_the_asked_interval_and_stops_on_request() {
        let mut set = cameras(StreamKind::None, None);
        set.command(0, GCS, MOCK_CAMERA, MavCmd::MAV_CMD_CAMERA_TRACK_RECTANGLE, [0.4, 0.4, 0.6, 0.6, 0.0, 0.0, 0.0]);
        assert!(set.tick(100, &HERE).is_empty(), "no interval asked yet");
        set.command(0, GCS, MOCK_CAMERA, MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL, [275.0, 200_000.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(set.tick(200, &HERE).len(), 1);
        assert!(set.tick(300, &HERE).is_empty());
        assert_eq!(set.tick(400, &HERE).len(), 1);
        set.command(0, GCS, MOCK_CAMERA, MavCmd::MAV_CMD_CAMERA_STOP_TRACKING, [0.0; 7]);
        assert!(set.tick(1000, &HERE).is_empty());
    }
}
