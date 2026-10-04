use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use base64::Engine;
use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];
pub const OPTICAL_FLOW_SCREEN: &str = "opticalFlow";
const MAX_IMAGE_BYTES: u32 = 1024 * 1024;
const ENCAPSULATED_BYTES: usize = 253;
const QIMAGE_ALLOCATION_LIMIT: u64 = 256 * 1024 * 1024;
const IMG_JPEG: u8 = 0;
const IMG_BMP: u8 = 1;
const IMG_RAW8U: u8 = 2;
const IMG_RAW32U: u8 = 3;
const IMG_PGM: u8 = 4;
const IMG_PNG: u8 = 5;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Handshake {
    pub kind: u8,
    pub size: u32,
    pub width: u16,
    pub height: u16,
    pub packets: u16,
    pub payload: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Receiver {
    handshake: Handshake,
    bytes: Vec<u8>,
    pub index: u32,
    pub image: Option<Image>,
}

impl Receiver {
    pub fn on_handshake(&mut self, handshake: Handshake) {
        self.bytes.clear();
        let valid = handshake.size > 0 && handshake.payload > 0 && handshake.packets > 0 && handshake.size <= MAX_IMAGE_BYTES && usize::from(handshake.payload) <= ENCAPSULATED_BYTES;
        self.handshake = if valid { handshake } else { Handshake::default() };
        if valid {
            self.bytes = vec![0; handshake.size as usize];
        }
    }

    pub fn on_data(&mut self, seqnr: u16, data: &[u8]) {
        if self.handshake.packets == 0 {
            return;
        }
        let position = u32::from(seqnr) * u32::from(self.handshake.payload);
        if position >= self.handshake.size {
            return;
        }
        let count = (u32::from(self.handshake.payload)).min(self.handshake.size - position) as usize;
        let start = position as usize;
        self.bytes[start..start + count].copy_from_slice(&data[..count.min(data.len())]);
        self.handshake.packets -= 1;
        if self.handshake.packets == 0 {
            let bytes = std::mem::take(&mut self.bytes);
            self.image = decoded(&self.handshake, &bytes);
            self.index += 1;
        }
    }
}

fn limited<R: std::io::BufRead + std::io::Seek>(mut reader: image::ImageReader<R>) -> image::ImageReader<R> {
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(QIMAGE_ALLOCATION_LIMIT);
    reader.limits(limits);
    reader
}

fn decoded(handshake: &Handshake, bytes: &[u8]) -> Option<Image> {
    let encoded = match handshake.kind {
        IMG_RAW8U | IMG_RAW32U => [format!("P5\n{} {}\n255\n", handshake.width, handshake.height).as_bytes(), bytes].concat(),
        IMG_JPEG | IMG_BMP | IMG_PGM | IMG_PNG => bytes.to_vec(),
        _ => return None,
    };
    let reader = image::ImageReader::new(std::io::Cursor::new(encoded)).with_guessed_format().ok()?;
    let image = limited(reader).decode().ok().filter(|image| image.width() > 0 && image.height() > 0)?.flipv().into_rgba8();
    Some(Image { width: image.width(), height: image.height(), rgba: image.into_raw() })
}

static RECEIVERS: Mutex<BTreeMap<u8, Receiver>> = Mutex::new(BTreeMap::new());

pub fn on_handshake(vehicle: u8, handshake: Handshake) {
    RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).entry(vehicle).or_default().on_handshake(handshake);
}

pub fn on_data(vehicle: u8, seqnr: u16, data: &[u8]) {
    RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).entry(vehicle).or_default().on_data(seqnr, data);
}

pub fn forget(vehicle: u8) {
    RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(&vehicle);
}

pub fn image_index(vehicle: u8) -> u32 {
    RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).get(&vehicle).map_or(0, |r| r.index)
}

pub fn optical_flow_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = crate::hub::lock().active_id();
    let receivers = RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner);
    let receiver = vehicle.and_then(|id| receivers.get(&id));
    let image = receiver.and_then(|r| r.image.as_ref());
    json!({
        "kind": "object",
        "class": "OpticalFlow",
        "index": receiver.map_or(0, |r| r.index),
        "image": image.map(|image| json!({
            "width": image.width,
            "height": image.height,
            "data": base64::engine::general_purpose::STANDARD.encode(&image.rgba),
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(values: &[u8]) -> Vec<u8> {
        values.iter().flat_map(|v| [*v, *v, *v, 255]).collect()
    }

    #[test]
    fn an_image_is_reassembled_from_its_packets_and_flipped_like_qgc_image_provider() {
        let mut receiver = Receiver::default();
        receiver.on_handshake(Handshake { kind: IMG_RAW8U, size: 5, width: 2, height: 2, packets: 2, payload: 3 });
        receiver.on_data(1, &[4, 5, 99]);
        assert!(receiver.image.is_none());
        receiver.on_data(0, &[1, 2, 3]);
        assert_eq!(receiver.image, Some(Image { width: 2, height: 2, rgba: grey(&[3, 4, 1, 2]) }), "the last packet is clamped to the declared size, extra bytes are ignored and rows come out bottom-up");
        assert_eq!(receiver.index, 1);
    }

    #[test]
    fn raw_frames_are_read_through_a_pgm_header_like_image_protocol_manager() {
        let raw = |kind, width, height, bytes: &[u8]| decoded(&Handshake { kind, size: bytes.len() as u32, width, height, packets: 0, payload: 1 }, bytes);
        assert_eq!(raw(IMG_RAW32U, 2, 1, &[7, 8, 9, 10]), Some(Image { width: 2, height: 1, rgba: grey(&[7, 8]) }));
        assert_eq!(raw(IMG_RAW8U, 2, 2, &[1, 2, 3]), None, "short pixel data fails to load");
        assert_eq!(raw(IMG_RAW8U, 0, 2, &[1, 2]), None, "a zero-sized frame fails to load");
    }

    #[test]
    fn encoded_frames_are_sniffed_whatever_type_they_declare() {
        let encoded = |kind, bytes: &[u8]| decoded(&Handshake { kind, size: bytes.len() as u32, width: 0, height: 0, packets: 0, payload: 1 }, bytes);
        let pgm = b"P5\n# comment\n1 2\n15\n\x0f\x00";
        assert_eq!(encoded(IMG_PGM, pgm), Some(Image { width: 1, height: 2, rgba: grey(&[0, 255]) }), "maxval scales and comments are skipped like QImage's PPM reader");
        assert_eq!(encoded(IMG_JPEG, pgm), encoded(IMG_PGM, pgm), "QImage::loadFromData ignores the declared type");
        assert_eq!(encoded(IMG_PNG, b"not an image"), None);
        assert_eq!(encoded(6, pgm), None, "an unsupported image type gives no image");
    }

    #[test]
    fn a_failed_image_replaces_the_previous_one_and_still_counts() {
        let mut receiver = Receiver::default();
        receiver.on_handshake(Handshake { kind: IMG_RAW8U, size: 1, width: 1, height: 1, packets: 1, payload: 1 });
        receiver.on_data(0, &[9]);
        receiver.on_handshake(Handshake { kind: IMG_PNG, size: 1, width: 1, height: 1, packets: 1, payload: 1 });
        receiver.on_data(0, &[9]);
        assert_eq!((receiver.index, receiver.image.is_none()), (2, true), "QGCImageProvider stores the null QImage");
    }

    #[test]
    fn bad_handshakes_and_stray_packets_are_dropped() {
        let mut receiver = Receiver::default();
        receiver.on_data(0, &[1]);
        receiver.on_handshake(Handshake { kind: 0, size: MAX_IMAGE_BYTES + 1, width: 1, height: 1, packets: 1, payload: 10 });
        receiver.on_data(0, &[1]);
        receiver.on_handshake(Handshake { kind: 0, size: 10, width: 1, height: 1, packets: 1, payload: 254 });
        receiver.on_data(0, &[1]);
        receiver.on_handshake(Handshake { kind: IMG_RAW8U, size: 4, width: 2, height: 2, packets: 1, payload: 4 });
        receiver.on_data(9, &[1, 2, 3, 4]);
        assert_eq!((receiver.index, receiver.image.is_none()), (0, true), "nothing completes from an invalid handshake or a packet past the end");
        receiver.on_data(0, &[1, 2, 3, 4]);
        assert_eq!(receiver.index, 1);
    }

    #[test]
    fn a_removed_vehicle_starts_again_at_index_zero_like_a_new_image_protocol_manager() {
        on_handshake(201, Handshake { kind: IMG_RAW8U, size: 1, width: 1, height: 1, packets: 1, payload: 1 });
        on_data(201, 0, &[1]);
        assert_eq!(image_index(201), 1);
        forget(201);
        assert_eq!(image_index(201), 0);
    }
}
