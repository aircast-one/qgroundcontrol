use crate::modes::VehicleClass;

pub const REASON_WINDOW_MS: u64 = 2000;
const PREFIXES: [&str; 5] = ["PreArm: ", "Arm: ", "Arming denied: ", "Disarming denied: ", "Preflight Fail: "];
const RESPONSES: [&str; 3] = ["Arm: ", "Arming denied: ", "Disarming denied: "];
const DISARM_RESPONSE: &str = "Disarming denied: ";
const MODE_NOT_ARMABLE: &str = "mode not armable";
pub const REPLY_WAIT_MS: u64 = 800;

pub fn is_arming_text(text: &str) -> bool {
    PREFIXES.iter().any(|prefix| text.starts_with(prefix))
}

pub fn is_response_text(text: &str) -> bool {
    RESPONSES.iter().any(|prefix| text.starts_with(prefix))
}

pub fn from_text(text: &str, mode: &str, class: VehicleClass) -> Option<String> {
    is_response_text(text).then(|| arming(!text.starts_with(DISARM_RESPONSE), false, &[text.to_string()], mode, class))
}

fn stripped(text: &str) -> &str {
    PREFIXES.iter().find_map(|prefix| text.strip_prefix(prefix)).unwrap_or(text).trim().trim_end_matches('.')
}

fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(second)) if !second.is_uppercase() => first.to_lowercase().chain(text.chars().skip(1)).collect(),
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

pub fn arming(arming: bool, unanswered: bool, reasons: &[String], mode: &str, class: VehicleClass) -> String {
    let verb = if arming { "arm" } else { "disarm" };
    let mode = if mode.trim().is_empty() { "this".to_string() } else { mode.trim().to_string() };
    let more = match reasons.len() {
        0 | 1 => String::new(),
        n => format!(", and {} more", n - 1),
    };
    match (unanswered, reasons.first().map(|reason| stripped(reason))) {
        (true, _) => format!("The aircraft didn't answer the {verb} command. Check the link and try again."),
        (false, Some(reason)) if arming && reason.to_lowercase().ends_with(MODE_NOT_ARMABLE) => match modes_to_try(class) {
            Some(modes) => format!("Can't arm in {mode} mode. Switch to {modes} first."),
            None => format!("Can't arm in {mode} mode. Choose another flight mode first."),
        },
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
        assert_eq!(arming(true, false, &said(&["Arm: Mode not armable"]), "Land", VehicleClass::MultiRotor), "Can't arm in Land mode. Switch to Loiter or Guided first.");
        assert_eq!(arming(true, false, &said(&["Arm: Mode not armable"]), "RTL", VehicleClass::FixedWing), "Can't arm in RTL mode. Switch to Manual or FBWA first.");
        assert_eq!(arming(true, false, &said(&["Arm: LAND mode not armable"]), "Land", VehicleClass::MultiRotor), "Can't arm in Land mode. Switch to Loiter or Guided first.", "ArduCopter 4.5 names the mode in its own capitals; the pilot reads the name the app shows");
        assert_eq!(arming(true, false, &said(&["Arm: Mode not armable"]), "", VehicleClass::Other), "Can't arm in this mode. Choose another flight mode first.");
    }

    #[test]
    fn the_autopilots_own_reason_is_kept_in_plain_words() {
        assert_eq!(arming(true, false, &said(&["Arm: Throttle (RC3) is not neutral"]), "Stabilize", VehicleClass::MultiRotor), "Can't arm: throttle (RC3) is not neutral.");
        assert_eq!(arming(true, false, &said(&["PreArm: Compass not calibrated", "PreArm: GPS 1: Bad fix"]), "Loiter", VehicleClass::MultiRotor), "Can't arm: compass not calibrated, and 1 more.");
        assert_eq!(arming(true, false, &said(&["PreArm: EKF3 waiting for GPS config data."]), "Loiter", VehicleClass::MultiRotor), "Can't arm: EKF3 waiting for GPS config data.", "an acronym keeps its capitals");
        assert_eq!(arming(false, false, &said(&["Disarming denied: not landed"]), "Loiter", VehicleClass::MultiRotor), "Can't disarm: not landed.");
    }

    #[test]
    fn with_no_reason_it_still_says_what_happened() {
        assert_eq!(arming(true, false, &[], "Loiter", VehicleClass::MultiRotor), "The aircraft refused to arm. The readiness list may say why.");
        assert_eq!(arming(true, true, &said(&["Arm: Mode not armable"]), "Land", VehicleClass::MultiRotor), "The aircraft didn't answer the arm command. Check the link and try again.");
    }

    #[test]
    fn only_arming_messages_are_taken_as_the_reason() {
        assert!(is_arming_text("PreArm: Compass not calibrated") && is_arming_text("Arm: Mode not armable") && is_arming_text("Arming denied: Flight mode not allowed"));
        assert!(!is_arming_text("EKF3 IMU0 is using GPS"));
    }

    #[test]
    fn a_reply_to_an_arm_attempt_is_worded_whoever_tried_but_a_repeating_prearm_line_is_not() {
        assert_eq!(from_text("Arm: Mode not armable", "Land", VehicleClass::MultiRotor).as_deref(), Some("Can't arm in Land mode. Switch to Loiter or Guided first."), "an RC-switch arm is refused the same way");
        assert_eq!(from_text("Disarming denied: Not landed", "Loiter", VehicleClass::MultiRotor).as_deref(), Some("Can't disarm: not landed."));
        assert_eq!(from_text("PreArm: Compass not calibrated", "Loiter", VehicleClass::MultiRotor), None, "ArduPilot repeats these every 30 s while not ready; the readiness list shows them");
    }
}
