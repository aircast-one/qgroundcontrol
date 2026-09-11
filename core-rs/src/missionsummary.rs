use serde_json::{Value, json};

use crate::read::{integer, object, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &["plan.missionController", "settings.unitsSettings.horizontalDistanceUnits"];

const FEET_PER_METRE: f64 = 3.2808399;
const HORIZONTAL_UNITS_FEET: f64 = 0.0;
const METRES_PER_KILOMETRE: f64 = 1000.0;
const FEET_PER_MILE: f64 = 5280.0;
const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 3600;
const UNKNOWN: &str = "\u{2014}";

pub fn distance_text(metres: f64, imperial: bool) -> String {
    if !metres.is_finite() || metres < 0.0 {
        return UNKNOWN.to_string();
    }
    match imperial {
        true => {
            let feet = metres * FEET_PER_METRE;
            match feet >= FEET_PER_MILE {
                true => format!("{:.2} mi", feet / FEET_PER_MILE),
                false => format!("{feet:.0} ft"),
            }
        }
        false => match metres >= METRES_PER_KILOMETRE {
            true => format!("{:.2} km", metres / METRES_PER_KILOMETRE),
            false => format!("{metres:.0} m"),
        },
    }
}

pub fn duration_text(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return UNKNOWN.to_string();
    }
    let whole = seconds.round() as i64;
    let hours = whole / SECONDS_PER_HOUR;
    let minutes = (whole % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let remainder = whole % SECONDS_PER_MINUTE;
    match hours {
        0 => format!("{minutes}:{remainder:02}"),
        _ => format!("{hours}:{minutes:02}:{remainder:02}"),
    }
}

pub fn summary_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let imperial = value_number(&backend.get("settings.unitsSettings.horizontalDistanceUnits.rawValue")) == Some(HORIZONTAL_UNITS_FEET);
    let mission = object(&backend.get_fields(
        "plan.missionController",
        "containsItems,missionTotalDistance,missionPlannedDistance,missionTime,missionHoverDistance,missionCruiseDistance,missionHoverTime,missionCruiseTime,missionMaxTelemetry,batteriesRequired,minAMSLAltitude,maxAMSLAltitude",
    ));
    let has_items = mission.get("containsItems").and_then(Value::as_bool).unwrap_or(false);
    let metres = |key: &str| mission.get(key).and_then(Value::as_f64).filter(|value| value.is_finite() && *value >= 0.0);
    let seconds = |key: &str| mission.get(key).and_then(Value::as_f64).filter(|value| value.is_finite() && *value >= 0.0);

    let total = metres("missionTotalDistance");
    let batteries = integer(&mission, "batteriesRequired").filter(|count| *count > 0);
    let rows: Vec<Value> = vec![
        row("Distance", total.map(|value| distance_text(value, imperial))),
        row("Planned", metres("missionPlannedDistance").map(|value| distance_text(value, imperial))),
        row("Time", seconds("missionTime").map(duration_text)),
        row("Hover", metres("missionHoverDistance").map(|value| distance_text(value, imperial))),
        row("Cruise", metres("missionCruiseDistance").map(|value| distance_text(value, imperial))),
        row("Furthest from launch", metres("missionMaxTelemetry").map(|value| distance_text(value, imperial))),
        row("Batteries", batteries.map(|count| count.to_string())),
    ];

    json!({
        "kind": "object",
        "class": "MissionSummary",
        "available": has_items,
        "imperial": imperial,
        "rows": rows.into_iter().filter(|row| row["value"] != Value::Null).collect::<Vec<_>>(),
        "distanceMetres": total,
        "timeSeconds": seconds("missionTime"),
        "batteriesRequired": batteries,
        "altitudeRange": altitude_range(&mission, imperial),
        "reason": match has_items {
            true => "",
            false => "This plan has no items yet.",
        },
    })
}

fn row(label: &str, value: Option<String>) -> Value {
    json!({ "label": label, "value": value })
}

fn altitude_text(metres: f64, imperial: bool) -> String {
    match metres < 0.0 {
        true => format!("-{}", distance_text(-metres, imperial)),
        false => distance_text(metres, imperial),
    }
}

fn altitude_range(mission: &Value, imperial: bool) -> Value {
    let read = |key: &str| mission.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
    match (read("minAMSLAltitude"), read("maxAMSLAltitude")) {
        (Some(low), Some(high)) if high >= low => json!({
            "lowest": low,
            "highest": high,
            "text": format!("{} to {}", altitude_text(low, imperial), altitude_text(high, imperial)),
        }),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_distance_reads_in_the_unit_the_operator_chose() {
        assert_eq!(distance_text(750.0, false), "750 m");
        assert_eq!(distance_text(1000.0, false), "1.00 km", "a kilometre is a kilometre rather than a thousand metres");
        assert_eq!(distance_text(2500.0, false), "2.50 km");
        assert_eq!(distance_text(100.0, true), "328 ft");
        assert_eq!(distance_text(2000.0, true), "1.24 mi");
        assert_eq!(distance_text(-1.0, false), UNKNOWN, "a negative distance is a number that was never computed, not a distance");
        assert_eq!(distance_text(f64::NAN, false), UNKNOWN);
    }

    #[test]
    fn a_duration_reads_as_a_clock_rather_than_a_count_of_seconds() {
        assert_eq!(duration_text(0.0), "0:00");
        assert_eq!(duration_text(9.0), "0:09");
        assert_eq!(duration_text(75.0), "1:15");
        assert_eq!(duration_text(3600.0), "1:00:00", "an hour long mission gains an hours field rather than reading sixty minutes");
        assert_eq!(duration_text(3661.0), "1:01:01");
        assert_eq!(duration_text(-5.0), UNKNOWN);
    }

    struct Plan(Value, f64);

    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.unitsSettings.horizontalDistanceUnits.rawValue" => json!({ "kind": "value", "value": self.1 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "plan.missionController" => self.0.to_string(),
                _ => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn flown() -> Value {
        json!({
            "kind": "object",
            "containsItems": true,
            "missionTotalDistance": 1500.0,
            "missionPlannedDistance": 1400.0,
            "missionTime": 185.0,
            "missionHoverDistance": 200.0,
            "missionCruiseDistance": 1300.0,
            "missionMaxTelemetry": 640.0,
            "batteriesRequired": 2,
            "minAMSLAltitude": 480.0,
            "maxAMSLAltitude": 530.0,
        })
    }

    fn labelled(view: &Value, label: &str) -> Option<String> {
        view["rows"].as_array().unwrap().iter().find(|row| row["label"] == label).map(|row| row["value"].as_str().unwrap().to_string())
    }

    #[test]
    fn a_flown_plan_reports_what_it_will_cost_to_fly() {
        let view = summary_view(&Plan(flown(), 1.0), &[]);
        assert_eq!(view["available"], true);
        assert_eq!(labelled(&view, "Distance").unwrap(), "1.50 km");
        assert_eq!(labelled(&view, "Time").unwrap(), "3:05");
        assert_eq!(labelled(&view, "Furthest from launch").unwrap(), "640 m");
        assert_eq!(labelled(&view, "Batteries").unwrap(), "2");
        assert_eq!(view["altitudeRange"]["text"], "480 m to 530 m");
        assert_eq!(view["distanceMetres"], 1500.0, "the raw metres travel too, so a head that wants to draw a bar is not parsing a string back");
    }

    #[test]
    fn the_same_plan_reads_in_feet_when_that_is_what_was_chosen() {
        let view = summary_view(&Plan(flown(), 0.0), &[]);
        assert_eq!(view["imperial"], true);
        assert_eq!(labelled(&view, "Distance").unwrap(), "4921 ft", "fifteen hundred metres is under a mile, so it reads in feet");
        let longer = summary_view(&Plan({ let mut plan = flown(); plan["missionTotalDistance"] = json!(3000.0); plan }, 0.0), &[]);
        assert_eq!(labelled(&longer, "Distance").unwrap(), "1.86 mi");
        assert_eq!(labelled(&view, "Time").unwrap(), "3:05", "time is not a unit the operator chooses");
        assert_eq!(view["distanceMetres"], 1500.0, "the raw number stays metric whichever way it is drawn");
    }

    #[test]
    fn a_row_the_controller_did_not_compute_is_left_out_rather_than_shown_as_zero() {
        let mut sparse = flown();
        sparse["missionHoverDistance"] = json!(-1.0);
        sparse["batteriesRequired"] = json!(0);
        let view = summary_view(&Plan(sparse, 1.0), &[]);
        assert!(labelled(&view, "Hover").is_none(), "minus one is what this controller answers when it has not worked something out, and drawing it as a distance would be a lie");
        assert!(labelled(&view, "Batteries").is_none(), "no battery model means no answer, which is not the same as needing no batteries");
        assert!(labelled(&view, "Distance").is_some());
    }

    #[test]
    fn an_empty_plan_says_so_rather_than_reporting_a_mission_of_nothing() {
        let empty = json!({ "kind": "object", "containsItems": false, "missionTotalDistance": 0.0, "missionTime": 0.0 });
        let view = summary_view(&Plan(empty, 1.0), &[]);
        assert_eq!(view["available"], false);
        assert_eq!(view["reason"], "This plan has no items yet.");
    }

    #[test]
    fn a_mission_below_sea_level_reads_as_a_depth_rather_than_as_nothing() {
        let mut low = flown();
        low["minAMSLAltitude"] = json!(-390.0);
        low["maxAMSLAltitude"] = json!(-340.0);
        let view = summary_view(&Plan(low, 1.0), &[]);
        assert_eq!(view["altitudeRange"]["text"], "-390 m to -340 m", "the Dead Sea is four hundred metres down and a plan flown over it still has altitudes");
        assert_eq!(view["altitudeRange"]["lowest"], -390.0);
    }

    #[test]
    fn an_altitude_range_the_wrong_way_round_is_no_range_at_all() {
        let mut wrong = flown();
        wrong["minAMSLAltitude"] = json!(530.0);
        wrong["maxAMSLAltitude"] = json!(480.0);
        assert_eq!(summary_view(&Plan(wrong, 1.0), &[])["altitudeRange"], Value::Null);
        let mut absent = flown();
        absent["maxAMSLAltitude"] = json!(f64::NAN.to_string());
        assert_eq!(summary_view(&Plan(absent, 1.0), &[])["altitudeRange"], Value::Null);
    }
}
