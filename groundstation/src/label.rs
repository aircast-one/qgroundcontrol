use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];

const ACRONYMS: &[&str] = &[
    "ADSB", "AGL", "AMSL", "APM", "ESC", "GCS", "GPS", "ID", "IMU", "NMEA", "PX4", "RC", "RTK", "RTSP", "TCP", "UDP", "UVC", "VTOL", "UTM", "SBS", "3D",
];

pub fn humanise(identifier: &str) -> String {
    let chars: Vec<char> = identifier.chars().collect();
    let spelt_acronym = |from: usize, at: usize| -> bool {
        ACRONYMS.contains(&chars[from..at + 1].iter().collect::<String>().to_uppercase().as_str())
    };
    let starts_word = |start: usize, i: usize| -> bool {
        let (c, previous) = (chars[i], chars[i - 1]);
        let next = chars.get(i + 1).copied();
        (c.is_uppercase() && previous.is_lowercase())
            || (c.is_uppercase() && previous.is_uppercase() && next.is_some_and(|n| n.is_lowercase()))
            || (c.is_uppercase() && previous.is_numeric() && !spelt_acronym(start, i))
            || (c.is_numeric() && previous.is_alphabetic() && !previous.is_uppercase() && !spelt_acronym(start, i))
    };
    let boundaries: Vec<usize> = (1..chars.len())
        .fold((0usize, Vec::new()), |(start, found), i| match starts_word(start, i) {
            true => (i, [found, vec![i]].concat()),
            false => (start, found),
        })
        .1;
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
        assert_eq!(humanise("PX4Params"), "PX4 Params", "nothing split at an uppercase letter whose predecessor was a digit, so an identifier that began with an acronym carrying one came out unsplit - this assertion used to pin that as the answer");
        assert_eq!(humanise("px4HiddenFlightModesMultiRotor"), "PX4 Hidden Flight Modes Multi Rotor", "the digit started a word, so px4 was never a chunk the acronym table could see, and the rest never split again: six rows of Settings read Px 4Hidden Flight Modes Multi Rotor");
        assert_eq!(humanise("batt1Monitor"), "Batt 1 Monitor", "and the same shape with an INDEX rather than a name has to keep splitting - batt is no acronym, so the digit still starts a word");
        assert_eq!(humanise("baro2Id"), "Baro 2 ID");
        assert_eq!(humanise("rpm1"), "Rpm 1", "a terminal digit has nothing after it, so the uppercase-after-digit rule cannot fire and an operator still numbers motors 1 to 4");
    }

    #[test]
    fn empty_and_single_words_pass_through_capitalised() {
        assert_eq!(humanise(""), "");
        assert_eq!(humanise("heading"), "Heading");
        assert_eq!(humanise("GPS"), "GPS");
    }

    #[test]
    fn the_view_answers_without_asking_the_backend_anything() {
        struct Forbidden;
        impl crate::router::Backend for Forbidden {
            fn get(&self, path: &str) -> String { panic!("view.label read {path}") }
            fn get_fields(&self, path: &str, _f: &str) -> String { panic!("view.label read {path}") }
            fn set(&self, path: &str, _v: &str) -> String { panic!("view.label wrote {path}") }
            fn invoke(&self, path: &str, _a: &str) -> String { panic!("view.label invoked {path}") }
            fn watch(&self, _p: &[String]) { panic!("view.label watched something") }
        }

        assert_eq!(super::label_view(&Forbidden, &["gpsLock".to_string()])["value"], "GPS Lock");
        assert!(
            super::DEPS.is_empty(),
            "the empty dep list is only honest while the compute is pure, and two things downstream rest on that: nothing would ever wake a consumer of this view, and the macOS head memoises humanise output in Labels.cache with no invalidation. A backend read added here goes stale in both places and neither says so"
        );
    }
}
