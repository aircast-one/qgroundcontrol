use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use base64::Engine;
use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];
pub const OPTICAL_FLOW_SCREEN: &str = "opticalFlow";
const MAX_IMAGE_BYTES: u32 = 1024 * 1024;
const ENCAPSULATED_BYTES: usize = 253;
const IMG_RAW8U: u8 = 2;
const IMG_RAW32U: u8 = 3;
const IMG_PGM: u8 = 4;

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
    pub raw: bool,
    pub width: u16,
    pub height: u16,
    pub bytes: Vec<u8>,
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
            let image = match self.handshake.kind {
                IMG_RAW8U | IMG_RAW32U => Some(Image { raw: true, width: self.handshake.width, height: self.handshake.height, bytes }),
                IMG_PGM => pgm(&bytes),
                _ => Some(Image { raw: false, width: self.handshake.width, height: self.handshake.height, bytes }),
            };
            self.image = image.or(self.image.take());
            self.index += 1;
        }
    }
}

static PGM_HEADER: std::sync::LazyLock<regex::bytes::Regex> = std::sync::LazyLock::new(|| regex::bytes::Regex::new(r"^P5\s+(\d+)\s+(\d+)\s+\d+\s").expect("pgm header"));

pub fn pgm(bytes: &[u8]) -> Option<Image> {
    let header = PGM_HEADER.captures(bytes)?;
    let number = |at: usize| std::str::from_utf8(header.get(at)?.as_bytes()).ok()?.parse::<u16>().ok();
    let (width, height) = (number(1)?, number(2)?);
    let count = usize::from(width) * usize::from(height);
    let pixels = bytes.get(header.get(0)?.end()..)?;
    (pixels.len() >= count).then(|| Image { raw: true, width, height, bytes: pixels[..count].to_vec() })
}

static RECEIVERS: Mutex<BTreeMap<u8, Receiver>> = Mutex::new(BTreeMap::new());

pub fn on_handshake(vehicle: u8, handshake: Handshake) {
    RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).entry(vehicle).or_default().on_handshake(handshake);
}

pub fn on_data(vehicle: u8, seqnr: u16, data: &[u8]) {
    RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).entry(vehicle).or_default().on_data(seqnr, data);
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
            "raw": image.raw,
            "width": image.width,
            "height": image.height,
            "data": base64::engine::general_purpose::STANDARD.encode(&image.bytes),
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_is_reassembled_from_its_packets_like_image_protocol_manager() {
        let mut receiver = Receiver::default();
        receiver.on_handshake(Handshake { kind: IMG_RAW8U, size: 5, width: 5, height: 1, packets: 2, payload: 3 });
        receiver.on_data(1, &[4, 5, 99]);
        assert!(receiver.image.is_none());
        receiver.on_data(0, &[1, 2, 3]);
        assert_eq!(receiver.image, Some(Image { raw: true, width: 5, height: 1, bytes: vec![1, 2, 3, 4, 5] }), "the last packet is clamped to the declared size");
        assert_eq!(receiver.index, 1);
    }

    #[test]
    fn a_pgm_image_is_served_as_its_grey_pixels() {
        assert_eq!(pgm(b"P5\n2 2\n255\n\x01\x02\x03\x04"), Some(Image { raw: true, width: 2, height: 2, bytes: vec![1, 2, 3, 4] }));
        assert_eq!(pgm(b"P6\n1 1\n255\n\x01"), None);
        assert_eq!(pgm(b"P5\n2 2\n255\n\x01"), None, "short pixel data is not an image");
    }

    #[test]
    fn bad_handshakes_and_stray_packets_are_dropped() {
        let mut receiver = Receiver::default();
        receiver.on_data(0, &[1]);
        receiver.on_handshake(Handshake { kind: 0, size: MAX_IMAGE_BYTES + 1, width: 1, height: 1, packets: 1, payload: 10 });
        receiver.on_data(0, &[1]);
        receiver.on_handshake(Handshake { kind: 0, size: 10, width: 1, height: 1, packets: 1, payload: 254 });
        receiver.on_data(0, &[1]);
        receiver.on_handshake(Handshake { kind: 0, size: 4, width: 2, height: 2, packets: 1, payload: 4 });
        receiver.on_data(9, &[1, 2, 3, 4]);
        assert_eq!((receiver.index, receiver.image.is_none()), (0, true), "nothing completes from an invalid handshake or a packet past the end");
        receiver.on_data(0, &[1, 2, 3, 4]);
        assert_eq!(receiver.image.as_ref().map(|i| i.raw), Some(false), "a JPEG arrives as encoded bytes");
    }
}
