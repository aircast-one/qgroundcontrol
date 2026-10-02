use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex, PoisonError};

use regex::Regex;
use serde_json::{Value, json};

use crate::router::Backend;

const MAX_QUEUED: usize = 64;
const MUTED_PATH: &str = "settings.appSettings.audioMuted";
const VOLUME_PATH: &str = "settings.appSettings.audioVolume";
const DEFAULT_VOLUME: f64 = 100.0;

const ABBREVIATIONS: &[(&str, &str)] = &[
    ("ERR", "error"),
    ("POSCTL", "Position Control"),
    ("ALTCTL", "Altitude Control"),
    ("AUTO_RTL", "auto return to launch"),
    ("RTL", "return To launch"),
    ("ACCEL", "accelerometer"),
    ("RC_MAP_MODE_SW", "RC mode switch"),
    ("REJ", "rejected"),
    ("WP", "waypoint"),
    ("CMD", "command"),
    ("COMPID", "component eye dee"),
    ("PARAMS", "parameters"),
    ("ID", "I.D."),
    ("ADSB", "A.D.S.B."),
    ("EKF", "E.K.F."),
    ("PREARM", "pre arm"),
    ("PITOT", "pee toe"),
    ("SERVOX_FUNCTION", "Servo X Function"),
];

static NEGATIVE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-\s*(\d)").expect("negative number pattern"));
static DECIMAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([0-9]+)\.([0-9]+)").expect("decimal pattern"));
static METERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([0-9]*\.?[0-9])\s?m([^A-Za-z]|$)").expect("meters pattern"));
static MILLISECONDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([0-9]+)ms").expect("milliseconds pattern"));

struct Queue {
    next: u64,
    spoken: VecDeque<(u64, String, f64)>,
}

static QUEUE: Mutex<Queue> = Mutex::new(Queue { next: 1, spoken: VecDeque::new() });

fn replace_abbreviations(text: &str) -> String {
    text.split(' ')
        .map(|word| ABBREVIATIONS.iter().find(|(short, _)| *short == word.to_uppercase()).map_or(word, |(_, long)| long))
        .collect::<Vec<_>>()
        .join(" ")
}

fn replace_decimal_points(text: &str) -> String {
    std::iter::successors(Some(text.to_string()), |current| DECIMAL.is_match(current).then(|| DECIMAL.replacen(current, 1, "$1 point $2").into_owned()))
        .last()
        .unwrap_or_default()
}

fn plural(count: i64, word: &str) -> String {
    format!("{count} {word}{}", if count > 1 { "s" } else { "" })
}

fn milliseconds_text(number: i64) -> String {
    match number < 60_000 {
        true => {
            let (seconds, ms) = (number / 1000, number % 1000);
            plural(seconds, "second") + &if ms > 0 { format!(" and {ms} millisecond") } else { String::new() }
        }
        false => {
            let (minutes, seconds) = (number / 60_000, (number % 60_000) / 1000);
            plural(minutes, "minute") + &if seconds > 0 { format!(" and {}", plural(seconds, "second")) } else { String::new() }
        }
    }
}

fn convert_milliseconds(text: &str) -> String {
    let Some(found) = MILLISECONDS.captures(text) else { return text.to_string() };
    match found[1].parse::<i64>() {
        Ok(number) if number >= 1000 => text.replace(&found[0], &milliseconds_text(number)),
        _ => text.to_string(),
    }
}

pub fn fix_text(text: &str) -> String {
    let negatives = NEGATIVE.replace_all(&replace_abbreviations(text), "negative $1").into_owned();
    let meters = METERS.replace_all(&replace_decimal_points(&negatives), "$1 meters$2").into_owned();
    convert_milliseconds(&meters)
}

fn setting(path: &str) -> Option<Value> {
    crate::settingsstore::raw_setting(path)
}

fn volume() -> f64 {
    setting(VOLUME_PATH).and_then(|v| v.as_f64()).unwrap_or(DEFAULT_VOLUME).clamp(0.0, 100.0)
}

fn silenced() -> bool {
    setting(MUTED_PATH).and_then(|v| v.as_bool()).unwrap_or(false) || volume() <= 0.0
}

pub fn say(text: &str) {
    if silenced() {
        return;
    }
    enqueue(&fix_text(text), volume() / 100.0);
}

#[cfg(test)]
thread_local! {
    static HEARD_HERE: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn enqueue(text: &str, volume: f64) {
    if text.trim().is_empty() {
        return;
    }
    #[cfg(test)]
    HEARD_HERE.with(|heard| heard.borrow_mut().push(text.to_string()));
    let mut queue = QUEUE.lock().unwrap_or_else(PoisonError::into_inner);
    let sequence = queue.next;
    queue.next += 1;
    queue.spoken.push_back((sequence, text.to_string(), volume));
    if queue.spoken.len() > MAX_QUEUED {
        queue.spoken.pop_front();
    }
}

#[cfg(test)]
pub fn spoken_lines() -> Vec<String> {
    HEARD_HERE.with(|heard| heard.borrow().clone())
}

pub fn vehicle_prefix(id: u8, vehicles: usize) -> String {
    match vehicles > 1 {
        true => format!("Vehicle {id} "),
        false => String::new(),
    }
}

pub fn speech_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let after: u64 = args.first().and_then(|a| a.trim().parse().ok()).unwrap_or(0);
    let queue = QUEUE.lock().unwrap_or_else(PoisonError::into_inner);
    json!({
        "kind": "object",
        "class": "Speech",
        "last": queue.next - 1,
        "muted": silenced(),
        "lines": queue.spoken.iter().filter(|(sequence, _, _)| *sequence > after).map(|(sequence, text, volume)| json!({ "sequence": sequence, "text": text, "volume": volume })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoBackend;
    impl Backend for NoBackend {
        fn get(&self, _path: &str) -> String { String::new() }
        fn get_fields(&self, _path: &str, _fields: &str) -> String { String::new() }
        fn set(&self, _path: &str, _value: &str) -> String { String::new() }
        fn invoke(&self, _path: &str, _args: &str) -> String { String::new() }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn spoken_text_is_fixed_up_the_way_audio_output_does() {
        assert_eq!(fix_text("PreArm: EKF not ready"), "PreArm: E.K.F. not ready");
        assert_eq!(fix_text("ALTCTL flight mode"), "Altitude Control flight mode");
        assert_eq!(fix_text("altitude -12.5 m"), "altitude negative 12 point 5 meters");
        assert_eq!(fix_text("timeout 2500ms"), "timeout 2 seconds and 500 millisecond");
        assert_eq!(fix_text("waited 125000ms"), "waited 2 minutes and 5 seconds");
        assert_eq!(fix_text("short 900ms"), "short 900ms", "under a second stays as written");
        assert_eq!(fix_text("1.2.3"), "1 point 2 point 3");
    }

    #[test]
    fn lines_are_numbered_and_read_after_a_sequence() {
        enqueue("first line", 1.0);
        enqueue("  ", 1.0);
        enqueue("second line", 0.5);
        let view = speech_view(&NoBackend, &[]);
        let lines = view["lines"].as_array().unwrap();
        let first = lines.iter().find(|l| l["text"] == "first line").unwrap()["sequence"].as_u64().unwrap();
        let after = speech_view(&NoBackend, &[first.to_string()]);
        assert!(after["lines"].as_array().unwrap().iter().any(|l| l["text"] == "second line" && l["volume"] == 0.5));
        assert!(!after["lines"].as_array().unwrap().iter().any(|l| l["text"] == "first line"));
        assert_eq!(vehicle_prefix(2, 2), "Vehicle 2 ");
        assert_eq!(vehicle_prefix(2, 1), "");
    }
}
