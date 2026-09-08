use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];

const ACRONYMS: &[&str] = &[
    "ADSB", "AGL", "AMSL", "APM", "ESC", "GCS", "GPS", "ID", "IMU", "NMEA", "PX4", "RC", "RTK", "RTSP", "TCP", "UDP", "UVC", "VTOL", "UTM", "SBS", "3D",
];

pub fn humanise(identifier: &str) -> String {
    let chars: Vec<char> = identifier.chars().collect();
    let starts_word = |i: usize| -> bool {
        let (c, previous) = (chars[i], chars[i - 1]);
        let next = chars.get(i + 1).copied();
        (c.is_uppercase() && previous.is_lowercase())
            || (c.is_uppercase() && previous.is_uppercase() && next.is_some_and(|n| n.is_lowercase()))
            || (c.is_numeric() && previous.is_alphabetic() && !previous.is_uppercase())
    };
    let boundaries: Vec<usize> = (1..chars.len()).filter(|i| starts_word(*i)).collect();
    let starts = std::iter::once(0).chain(boundaries.iter().copied());
    let ends = boundaries.iter().copied().chain(std::iter::once(chars.len()));
    starts
        .zip(ends)
        .map(|(a, b)| chars[a..b].iter().collect::<String>())
        .map(|word| match ACRONYMS.contains(&word.to_uppercase().as_str()) {
            true => word.to_uppercase(),
            false => capitalise(&word),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalise(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

pub fn label_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let identifier = args.first().cloned().unwrap_or_default();
    json!({ "kind": "value", "value": humanise(&identifier) })
}

#[cfg(test)]
mod tests {
    use super::humanise;

    #[test]
    fn splits_camel_case_and_keeps_acronyms_whole() {
        assert_eq!(humanise("altitudeRelative"), "Altitude Relative");
        assert_eq!(humanise("RemoteID"), "Remote ID");
        assert_eq!(humanise("ADSBVehicleManager"), "ADSB Vehicle Manager");
        assert_eq!(humanise("gpsRawInt"), "GPS Raw Int");
        assert_eq!(humanise("rtkSettings"), "RTK Settings");
    }

    #[test]
    fn digits_stay_with_their_word_unless_they_follow_a_lowercase_letter() {
        assert_eq!(humanise("view3D"), "View 3D");
        assert_eq!(humanise("h264"), "H 264");
        assert_eq!(humanise("PX4Params"), "PX4Params");
    }

    #[test]
    fn empty_and_single_words_pass_through_capitalised() {
        assert_eq!(humanise(""), "");
        assert_eq!(humanise("heading"), "Heading");
        assert_eq!(humanise("GPS"), "GPS");
    }
}
