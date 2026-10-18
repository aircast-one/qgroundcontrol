use crate::modes::VehicleClass;

pub const REASON_WINDOW_MS: u64 = 2000;
const PREFIXES: [&str; 5] = [
    "PreArm: ",
    "Arm: ",
    "Arming denied: ",
    "Disarming denied: ",
    "Preflight Fail: ",
];
const RESPONSES: [&str; 3] = ["Arm: ", "Arming denied: ", "Disarming denied: "];
const DISARM_RESPONSE: &str = "Disarming denied: ";
const MODE_NOT_ARMABLE: &str = "mode not armable";
pub const REPLY_WAIT_MS: u64 = 800;
const NO_SUCH_MODE: &str = "No such mode ";
const MODE_CHANGE_TO: &str = "Mode change to ";
const MODE_REPLIES: [&str; 3] = [NO_SUCH_MODE, MODE_CHANGE_TO, "Flight mode change failed"];
const NOT_IN_FIRMWARE: &str = "this aircraft's firmware doesn't have it";

const PREARM: &str = "PreArm:";

pub fn told_elsewhere(text: &str) -> bool {
    text.starts_with(PREARM) || is_response_text(text) || is_mode_reply(text)
}

pub fn is_mode_reply(text: &str) -> bool {
    MODE_REPLIES.iter().any(|prefix| text.starts_with(prefix))
}

pub fn missing_mode(text: &str) -> Option<u32> {
    text.strip_prefix(NO_SUCH_MODE)?.trim().parse().ok()
}

fn mode_reason(text: &str, asked_mode: u32) -> Option<String> {
    match missing_mode(text) {
        Some(missing) => (missing == asked_mode).then(|| NOT_IN_FIRMWARE.to_string()),
        None => text
            .strip_prefix(MODE_CHANGE_TO)
            .and_then(|rest| rest.split_once(": ").or_else(|| rest.split_once(", ")))
            .map(|(_, reason)| sentence(reason.trim().trim_end_matches('.'))),
    }
}

pub fn mode_change(
    asked: &str,
    asked_mode: u32,
    unanswered: bool,
    reasons: &[String],
    current: &str,
) -> String {
    let still = match current.trim() {
        "" => String::new(),
        mode => format!(" Still in {mode}."),
    };
    match (
        unanswered,
        reasons
            .iter()
            .rev()
            .find_map(|text| mode_reason(text, asked_mode)),
    ) {
        (true, _) => format!(
            "The aircraft didn't answer the switch to {asked}. Check the link and try again."
        ),
        (false, Some(reason)) => format!("Can't switch to {asked}: {reason}.{still}"),
        (false, None) => format!("The aircraft refused {asked}.{still}"),
    }
}

pub fn is_arming_text(text: &str) -> bool {
    PREFIXES.iter().any(|prefix| text.starts_with(prefix))
}

pub fn is_response_text(text: &str) -> bool {
    RESPONSES.iter().any(|prefix| text.starts_with(prefix))
}

pub fn from_text(text: &str, mode: &str, class: VehicleClass) -> Option<String> {
    is_response_text(text).then(|| {
        arming(
            !text.starts_with(DISARM_RESPONSE),
            false,
            &[text.to_string()],
            mode,
            class,
        )
    })
}

fn stripped(text: &str) -> &str {
    PREFIXES
        .iter()
        .find_map(|prefix| text.strip_prefix(prefix))
        .unwrap_or(text)
        .trim()
        .trim_end_matches('.')
}

fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(second)) if !second.is_uppercase() => {
            first.to_lowercase().chain(text.chars().skip(1)).collect()
        }
        _ => text.to_string(),
    }
}

fn modes_to_try(class: VehicleClass) -> Option<&'static str> {
    match class {
        VehicleClass::MultiRotor => Some("Loiter or Guided"),
        VehicleClass::FixedWing => Some("Manual or FBWA"),
        VehicleClass::Rover => Some("Manual or Hold"),
        _ => None,
    }
}

pub fn arming(
    arming: bool,
    unanswered: bool,
    reasons: &[String],
    mode: &str,
    class: VehicleClass,
) -> String {
    let verb = if arming { "arm" } else { "disarm" };
    let mode = if mode.trim().is_empty() {
        "this".to_string()
    } else {
        mode.trim().to_string()
    };
    let more = match reasons.len() {
        0 | 1 => String::new(),
        n => format!(", and {} more", n - 1),
    };
    match (unanswered, reasons.first().map(|reason| stripped(reason))) {
        (true, _) => {
            format!("The aircraft didn't answer the {verb} command. Check the link and try again.")
        }
        (false, Some(reason)) if arming && reason.to_lowercase().ends_with(MODE_NOT_ARMABLE) => {
            match modes_to_try(class) {
                Some(modes) => format!("Can't arm in {mode} mode. Switch to {modes} first."),
                None => format!("Can't arm in {mode} mode. Choose another flight mode first."),
            }
        }
        (false, Some(reason)) => format!("Can't {verb}: {}{more}.", sentence(reason)),
        (false, None) => format!("The aircraft refused to {verb}. The readiness list may say why."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| text.to_string()).collect()
    }

    #[test]
    fn a_mode_that_cannot_arm_says_which_modes_can() {
        assert_eq!(
            arming(
                true,
                false,
                &said(&["Arm: Mode not armable"]),
                "Land",
                VehicleClass::MultiRotor
            ),
            "Can't arm in Land mode. Switch to Loiter or Guided first."
        );
        assert_eq!(
            arming(
                true,
                false,
                &said(&["Arm: Mode not armable"]),
                "RTL",
                VehicleClass::FixedWing
            ),
            "Can't arm in RTL mode. Switch to Manual or FBWA first."
        );
        assert_eq!(
            arming(
                true,
                false,
                &said(&["Arm: LAND mode not armable"]),
                "Land",
                VehicleClass::MultiRotor
            ),
            "Can't arm in Land mode. Switch to Loiter or Guided first.",
            "ArduCopter 4.5 names the mode in its own capitals; the pilot reads the name the app shows"
        );
        assert_eq!(
            arming(
                true,
                false,
                &said(&["Arm: Mode not armable"]),
                "",
                VehicleClass::Other
            ),
            "Can't arm in this mode. Choose another flight mode first."
        );
    }

    #[test]
    fn the_autopilots_own_reason_is_kept_in_plain_words() {
        assert_eq!(
            arming(
                true,
                false,
                &said(&["Arm: Throttle (RC3) is not neutral"]),
                "Stabilize",
                VehicleClass::MultiRotor
            ),
            "Can't arm: throttle (RC3) is not neutral."
        );
        assert_eq!(
            arming(
                true,
                false,
                &said(&["PreArm: Compass not calibrated", "PreArm: GPS 1: Bad fix"]),
                "Loiter",
                VehicleClass::MultiRotor
            ),
            "Can't arm: compass not calibrated, and 1 more."
        );
        assert_eq!(
            arming(
                true,
                false,
                &said(&["PreArm: EKF3 waiting for GPS config data."]),
                "Loiter",
                VehicleClass::MultiRotor
            ),
            "Can't arm: EKF3 waiting for GPS config data.",
            "an acronym keeps its capitals"
        );
        assert_eq!(
            arming(
                false,
                false,
                &said(&["Disarming denied: not landed"]),
                "Loiter",
                VehicleClass::MultiRotor
            ),
            "Can't disarm: not landed."
        );
    }

    #[test]
    fn with_no_reason_it_still_says_what_happened() {
        assert_eq!(
            arming(true, false, &[], "Loiter", VehicleClass::MultiRotor),
            "The aircraft refused to arm. The readiness list may say why."
        );
        assert_eq!(
            arming(
                true,
                true,
                &said(&["Arm: Mode not armable"]),
                "Land",
                VehicleClass::MultiRotor
            ),
            "The aircraft didn't answer the arm command. Check the link and try again."
        );
    }

    #[test]
    fn a_refused_mode_change_says_why_and_what_the_aircraft_is_still_doing() {
        assert_eq!(
            mode_change("Autoland", 26, false, &said(&["No such mode 26"]), "RTL"),
            "Can't switch to Autoland: this aircraft's firmware doesn't have it. Still in RTL."
        );
        assert_eq!(
            mode_change(
                "Auto",
                3,
                false,
                &said(&["Mode change to AUTO failed: requires position"]),
                "Stabilize"
            ),
            "Can't switch to Auto: requires position. Still in Stabilize."
        );
        assert_eq!(
            mode_change(
                "Guided",
                15,
                false,
                &said(&["Mode change to GUIDED denied, in fence recovery"]),
                "RTL"
            ),
            "Can't switch to Guided: in fence recovery. Still in RTL."
        );
        assert_eq!(
            mode_change(
                "Cruise",
                7,
                false,
                &said(&["Flight mode change failed"]),
                "FBW A"
            ),
            "The aircraft refused Cruise. Still in FBW A."
        );
        assert_eq!(
            mode_change("Loiter", 12, false, &[], ""),
            "The aircraft refused Loiter."
        );
        assert_eq!(
            mode_change("Loiter", 12, true, &[], "RTL"),
            "The aircraft didn't answer the switch to Loiter. Check the link and try again."
        );
    }

    #[test]
    fn a_refusal_is_explained_by_its_own_reply_not_one_from_a_moment_before() {
        assert_eq!(
            mode_change("Thermal", 24, false, &said(&["No such mode 26"]), "RTL"),
            "The aircraft refused Thermal. Still in RTL.",
            "the plane said that about Autoland"
        );
        assert_eq!(
            mode_change(
                "Auto",
                10,
                false,
                &said(&[
                    "Mode change to AUTO failed: no mission",
                    "Mode change to AUTO failed: requires position"
                ]),
                "Manual"
            ),
            "Can't switch to Auto: requires position. Still in Manual.",
            "the newest reply answers the newest request"
        );
    }

    #[test]
    fn the_autopilots_reply_to_a_mode_change_is_recognised_and_a_missing_mode_is_named() {
        assert!(
            is_mode_reply("No such mode 26")
                && is_mode_reply("Mode change to AUTO failed: requires position")
                && is_mode_reply("Flight mode change failed")
        );
        assert!(!is_mode_reply("Arm: Mode not armable"));
        assert_eq!(
            (
                missing_mode("No such mode 26"),
                missing_mode("Mode change to AUTO failed: x")
            ),
            (Some(26), None)
        );
    }

    #[test]
    fn only_arming_messages_are_taken_as_the_reason() {
        assert!(
            is_arming_text("PreArm: Compass not calibrated")
                && is_arming_text("Arm: Mode not armable")
                && is_arming_text("Arming denied: Flight mode not allowed")
        );
        assert!(!is_arming_text("EKF3 IMU0 is using GPS"));
    }

    #[test]
    fn a_reply_to_an_arm_attempt_is_worded_whoever_tried_but_a_repeating_prearm_line_is_not() {
        assert_eq!(
            from_text("Arm: Mode not armable", "Land", VehicleClass::MultiRotor).as_deref(),
            Some("Can't arm in Land mode. Switch to Loiter or Guided first."),
            "an RC-switch arm is refused the same way"
        );
        assert_eq!(
            from_text(
                "Disarming denied: Not landed",
                "Loiter",
                VehicleClass::MultiRotor
            )
            .as_deref(),
            Some("Can't disarm: not landed.")
        );
        assert_eq!(
            from_text(
                "PreArm: Compass not calibrated",
                "Loiter",
                VehicleClass::MultiRotor
            ),
            None,
            "ArduPilot repeats these every 30 s while not ready; the readiness list shows them"
        );
    }
}
