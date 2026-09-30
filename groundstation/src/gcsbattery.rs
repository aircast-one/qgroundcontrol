use serde_json::{Value, json};

use crate::router::Backend;

const CRITICAL_PERCENT: i64 = 10;
const LOW_PERCENT: i64 = 25;

pub fn state(level: i64, charging: bool) -> &'static str {
    match () {
        _ if charging => "charging",
        _ if level <= CRITICAL_PERCENT => "critical",
        _ if level <= LOW_PERCENT => "low",
        _ => "normal",
    }
}

pub fn gcs_battery_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let level = args.first().and_then(|a| a.parse::<i64>().ok()).filter(|l| (0..=100).contains(l));
    let charging = args.get(1).is_some_and(|a| a == "true" || a == "1");
    match level {
        None => crate::read::refused("view.gcsBattery needs the charge in percent and whether it is charging, as view.gcsBattery(54,false)"),
        Some(level) => json!({
            "kind": "object",
            "class": "GcsBattery",
            "state": state(level, charging),
            "levelText": format!("{level}%"),
            "stateText": if charging { "Charging" } else { "On battery" },
            "heading": "Ground Station",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_follow_the_toolbar() {
        assert_eq!(state(5, true), "charging", "charging wins over a low charge");
        assert_eq!(state(10, false), "critical");
        assert_eq!(state(25, false), "low");
        assert_eq!(state(26, false), "normal");
    }
}
