use serde_json::{Value, json};

use crate::read::{Unit, integer, object, value_number};
use crate::router::Backend;

// "plan.missionController" names an object rather than a property, so the watcher has nothing to
// bind to and every change reached this view through the re-read that only runs while the event
// loop is idle. The fields it actually reads each carry a notify signal.
pub const DEPS: &[&str] = &[
    "plan.missionController.containsItems",
    "plan.missionController.missionTotalDistance",
    "plan.missionController.missionPlannedDistance",
    "plan.missionController.missionTime",
    "plan.missionController.missionHoverDistance",
    "plan.missionController.missionCruiseDistance",
    "plan.missionController.missionHoverTime",
    "plan.missionController.missionCruiseTime",
    "plan.missionController.missionMaxTelemetry",
    "plan.missionController.minAMSLAltitude",
    "plan.missionController.maxAMSLAltitude",
    "settings.unitsSettings.horizontalDistanceUnits",
    // The duration depends on the vehicle class and on the two speeds it is flown at, and all four
    // change without the plan changing: a different airframe connects, or the operator edits the
    // offline editing speeds in settings.
    "plan.controllerVehicle.multiRotor",
    "plan.controllerVehicle.vtol",
    "settings.appSettings.offlineEditingHoverSpeed",
    "settings.appSettings.offlineEditingCruiseSpeed",
    "settings.appSettings.offlineEditingAscentSpeed",
];

const FEET_PER_METRE: f64 = 3.2808399;
const HORIZONTAL_UNITS_FEET: f64 = 0.0;
const METRES_PER_KILOMETRE: f64 = 1000.0;
const FEET_PER_MILE: f64 = 5280.0;
const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 3600;
const UNKNOWN: &str = "\u{2014}";

pub fn imperial(backend: &dyn Backend) -> bool {
    value_number(&backend.get("settings.unitsSettings.horizontalDistanceUnits.rawValue")) == Some(HORIZONTAL_UNITS_FEET)
}

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

fn point_of(value: &Value) -> Option<(f64, f64)> {
    let at = value.as_object()?;
    Some((at.get("latitude")?.as_f64()?, at.get("longitude")?.as_f64()?))
}

fn flag_of(item: &Value, key: &str) -> bool {
    item.get(key).and_then(Value::as_bool) == Some(true)
}

// One walk, used by both figures. The two guards it carries are the two that have gone missing
// separately: flownLeg lives inside a filter and survives being copied, endsRoute truncates the
// collection before iteration and does not. Sharing the walk is what stops the next figure written
// beside these from carrying one and leaving the other - the fix the macOS head reached from
// "two places that must agree will eventually disagree" and this one reached from "the guard
// outside the expression's shape is the one that gets dropped".
fn flown(items: &[Value]) -> impl Iterator<Item = &Value> {
    let ends = items.iter().position(|item| flag_of(item, "endsRoute"));
    let reached = match ends {
        Some(at) => &items[..=at],
        None => items,
    };
    reached.iter().filter(|item| flag_of(item, "flownLeg"))
}

// MissionController measures each leg from the previous item's exit to this one's entry and adds a
// pattern's own path on top. Two things it does not do: it does not fly to an item that only
// carries a position, and it does not continue past the item that ends the route - anything after
// a landing or a return is uploaded and never reached. Both questions are already answered on
// every item, by flownLeg and endsRoute, and they are separate questions: a return to launch ends
// the route while being no leg at all.
pub fn flown_distance(items: &[Value]) -> f64 {
    flown(items).fold((0.0, None), |(total, previous), item| {
            let pattern = item.get("patternDistance").and_then(Value::as_f64).unwrap_or(0.0);
            let Some(entry) = item.get("coordinate").and_then(point_of) else {
                return (total, previous);
            };
            let leg = previous.map_or(0.0, |from| crate::surveygrid::distance_between(from, entry));
            let exit = item.get("exitCoordinate").and_then(point_of).unwrap_or(entry);
            (total + leg + pattern, Some(exit))
        })
        .0
}

// What the controller calls the mission's maximum telemetry distance. Two terms, and the second
// is not what its name suggests: for every flown item it takes the distance from the launch point
// to that item, and for a pattern it also folds in greatestDistanceTo(exitCoordinate) - the
// pattern's own greatest internal span, measured from its exit rather than from home.
//
// That second term is a category error in the original: it mixes a span with a set of distances
// from a fixed point, so a large survey near home can raise a figure that means "furthest from
// launch". Reproduced exactly anyway, because a port that quietly improves its source can never be
// checked against it. Recorded in the plan as a question to settle deliberately.
//
// The walk stops where the route ends, for the same reason the distance walk does: an item after a
// return to launch is uploaded and never reached, so it is nowhere the vehicle gets to.
pub fn telemetry_terms(items: &[Value]) -> Vec<(i64, f64, f64)> {
    let Some(home) = items.first().and_then(|item| item.get("coordinate")).and_then(point_of) else {
        return Vec::new();
    };
    flown(items)
        .filter(|item| item.get("kind").and_then(Value::as_str) != Some("settings"))
        .map(|item| {
            let sequence = item.get("sequence").and_then(Value::as_i64).unwrap_or(-1);
            let Some(entry) = item.get("coordinate").and_then(point_of) else { return (sequence, 0.0, 0.0) };
            let exit = item.get("exitCoordinate").and_then(point_of).unwrap_or(entry);
            let span = item
                .get("geometry")
                .and_then(|shape| shape.get("transects"))
                .and_then(Value::as_array)
                .map(|points| points.iter().filter_map(point_of).map(|at| crate::surveygrid::distance_between(exit, at)).fold(0.0, f64::max))
                .unwrap_or(0.0);
            (sequence, crate::surveygrid::distance_between(home, entry), span)
        })
        .collect()
}

pub fn max_telemetry_distance(items: &[Value]) -> f64 {
    telemetry_terms(items).iter().map(|(_, from_home, span)| from_home.max(*span)).fold(0.0, f64::max)
}

// How long the mission takes. Every leg is flown at one of two speeds and the vehicle class picks
// which: a multirotor hovers the whole way, anything else cruises. Each item can also add a delay
// of its own - a loiter, a camera pause - which is time nothing else accounts for.
//
// A VTOL is not attempted and answers None rather than a plausible number. MissionController flips
// vtolMode between multirotor and fixed wing as the walk passes a transition item, so the speed
// changes partway through a mission; a single-speed answer would be right for the legs before the
// transition and quietly wrong after it. That is the shape refused twice already today.
pub fn flown_seconds(items: &[Value], hover: f64, cruise: f64, ascent: f64, multirotor: bool, vtol: bool) -> Option<f64> {
    if vtol {
        return None;
    }
    let speed = match multirotor {
        true => hover,
        false => cruise,
    };
    if !speed.is_finite() || speed <= 0.0 {
        return None;
    }
    let delays: f64 = flown(items).filter_map(|item| item.get("extraSeconds").and_then(Value::as_f64)).filter(|extra| extra.is_finite()).sum();
    Some(flown_distance(items) / speed + delays + climb(items, ascent, multirotor))
}

// A rotor takes off straight up, so the height it climbs to is time the horizontal walk does not
// account for - MissionController special-cases it at the ascent speed rather than the hover
// speed. Absent from the first version of this and worth exactly 16.667 s on the default plan:
// fifty metres at three metres a second, which is what the disagreement turned out to be.
fn climb(items: &[Value], ascent: f64, multirotor: bool) -> f64 {
    if !multirotor || !ascent.is_finite() || ascent <= 0.0 {
        return 0.0;
    }
    let launch = items.first().and_then(|item| item.get("altitudeAmsl")).and_then(Value::as_f64);
    let first = flown(items).nth(1).and_then(|item| item.get("altitudeAmsl")).and_then(Value::as_f64);
    match (launch, first) {
        (Some(from), Some(to)) => (to - from).abs() / ascent,
        _ => 0.0,
    }
}

pub fn altitude_band(items: &[Value]) -> Option<(f64, f64)> {
    items
        .iter()
        .filter(|item| item.get("coordinate").and_then(point_of).is_some())
        .filter_map(|item| {
            let lowest = item.get("altitudeAmslLowest").and_then(Value::as_f64);
            let highest = item.get("altitudeAmslHighest").and_then(Value::as_f64);
            match (lowest, highest) {
                (Some(low), Some(high)) if high >= low => Some((low, high)),
                _ => item.get("altitudeAmsl").and_then(Value::as_f64).map(|at| (at, at)),
            }
        })
        .filter(|(low, high)| low.is_finite() && high.is_finite())
        .reduce(|(low, high), (next_low, next_high)| (low.min(next_low), high.max(next_high)))
}

pub fn summary_view(backend: &dyn Backend, args: &[String]) -> Value {
    let verify = args.iter().any(|arg| arg == "verify");
    let walked = verify
        .then(|| crate::missionitems::items_view(backend, &["geometry".to_string()]))
        .and_then(|view| view.get("items").and_then(Value::as_array).cloned());
    let imperial = imperial(backend);
    let mission = object(&backend.get_fields(
        "plan.missionController",
        "containsItems,missionTotalDistance,missionPlannedDistance,missionTime,missionHoverDistance,missionCruiseDistance,missionHoverTime,missionCruiseTime,missionMaxTelemetry,minAMSLAltitude,maxAMSLAltitude",
    ));
    let has_items = mission.get("containsItems").and_then(Value::as_bool).unwrap_or(false);
    let metres = |key: &str| mission.get(key).and_then(Value::as_f64).filter(|value| value.is_finite() && *value >= 0.0);
    let seconds = |key: &str| mission.get(key).and_then(Value::as_f64).filter(|value| value.is_finite() && *value >= 0.0);

    let airframe = object(&backend.get_fields("plan.controllerVehicle", "multiRotor,vtol"));
    let vtol = crate::read::flag(&airframe, "vtol");
    let hovers = vtol || crate::read::flag(&airframe, "multiRotor");
    let cruises = vtol || !crate::read::flag(&airframe, "multiRotor");

    let total = metres("missionTotalDistance");
    let rows: Vec<Value> = vec![
        row("Distance", total.map(|value| distance_text(value, imperial))),
        row("Planned", metres("missionPlannedDistance").map(|value| distance_text(value, imperial))),
        row("Time", seconds("missionTime").map(duration_text)),
        row("Hover", hovers.then(|| metres("missionHoverDistance")).flatten().map(|value| distance_text(value, imperial))),
        row("Cruise", cruises.then(|| metres("missionCruiseDistance")).flatten().map(|value| distance_text(value, imperial))),
        row("Furthest from launch", metres("missionMaxTelemetry").map(|value| distance_text(value, imperial))),
    ];

    json!({
        "kind": "object",
        "class": "MissionSummary",
        "available": has_items,
        "imperial": imperial,
        "rows": rows.into_iter().filter(|row| row["value"] != Value::Null).collect::<Vec<_>>(),
        "distanceMetres": total,
        // The same figure worked out by the core rather than read from the controller. Served
        // beside it so the two can be compared on every run against every plan a test builds,
        // which is the only way a port of this arithmetic can be trusted before it replaces it.
        // Opt-in, because working it out costs a whole view.missionItems and every head reading
        // the summary would pay for a figure only a test compares. view.missionSummary(verify).
        "durationComputedSeconds": walked
            .as_ref()
            .and_then(|items| {
                let vehicle = object(&backend.get_fields("plan.controllerVehicle", "multiRotor,vtol"));
                let speed = |name: &str| value_number(&backend.get(&format!("settings.appSettings.{name}.rawValue"))).unwrap_or(0.0);
                flown_seconds(items, speed("offlineEditingHoverSpeed"), speed("offlineEditingCruiseSpeed"), speed("offlineEditingAscentSpeed"), crate::read::flag(&vehicle, "multiRotor"), crate::read::flag(&vehicle, "vtol"))
            }),
        "durationSeconds": verify.then(|| metres("missionTime")).flatten(),
        "durationInputs": walked.as_ref().map(|items| json!({
            "hover": value_number(&backend.get("settings.appSettings.offlineEditingHoverSpeed.rawValue")),
            "cruise": value_number(&backend.get("settings.appSettings.offlineEditingCruiseSpeed.rawValue")),
            "ascent": value_number(&backend.get("settings.appSettings.offlineEditingAscentSpeed.rawValue")),
            "multiRotor": crate::read::flag(&object(&backend.get_fields("plan.controllerVehicle", "multiRotor")), "multiRotor"),
            "distance": flown_distance(items),
        })),
        "altitudeBandComputed": walked.as_ref().and_then(|items| altitude_band(items)).map(|(low, high)| json!([low, high])),
        "altitudeBandMetres": verify.then(|| json!([metres("minAMSLAltitude"), metres("maxAMSLAltitude")])),
        "maxTelemetryMetres": verify.then(|| metres("missionMaxTelemetry")).flatten(),
        "maxTelemetryComputedMetres": walked.as_ref().map(|items| max_telemetry_distance(items)),
        "maxTelemetryInputs": walked.as_ref().map(|items| json!({
            "terms": telemetry_terms(items).iter().map(|(sequence, from_home, span)| json!({ "sequence": sequence, "fromHome": from_home, "spanFromExit": span })).collect::<Vec<_>>(),
        })),
        "distanceComputedMetres": walked.as_ref().map(|items| flown_distance(items)),
        "distanceInputs": walked.as_ref().map(|items| json!({
            "legs": items.iter().filter(|item| crate::read::flag(item, "flownLeg")).count(),
            "endsRouteAt": items.iter().position(|item| crate::read::flag(item, "endsRoute")),
            "total": items.len(),
        })),
        "timeSeconds": seconds("missionTime"),
        "altitudeRange": altitude_range(&mission, &Unit::vertical(backend)),
        "reason": match has_items {
            true => "",
            false => "This plan has no items yet.",
        },
    })
}

fn row(label: &str, value: Option<String>) -> Value {
    json!({ "label": label, "value": value })
}

fn altitude_range(mission: &Value, vertical: &Unit) -> Value {
    let read = |key: &str| mission.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
    match (read("minAMSLAltitude"), read("maxAMSLAltitude")) {
        (Some(low), Some(high)) if high >= low => json!({
            "lowest": low,
            "highest": high,
            "text": format!("{} to {}", crate::read::altitude_text(low, vertical, false), crate::read::altitude_text(high, vertical, false)),
        }),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leg(lat: f64, lon: f64) -> Value {
        json!({ "flownLeg": true, "endsRoute": false, "coordinate": { "latitude": lat, "longitude": lon } })
    }

    #[test]
    fn the_walk_stops_where_the_route_ends_and_steps_over_what_is_not_flown() {
        let straight = vec![leg(47.3960, 8.5440), leg(47.3990, 8.5480)];
        let plain = flown_distance(&straight);
        assert!(plain > 400.0 && plain < 500.0, "two points about four hundred metres apart measured {plain}");

        // A return to launch carries no coordinate and ends the route. Anything after it is
        // uploaded and never reached, and the plan editor will not let one be built by appending -
        // it refuses to add after a landing - so the shape only arises from a return, and only a
        // list built by hand can put it in front of this function.
        let past_the_end = vec![
            leg(47.3960, 8.5440),
            leg(47.3990, 8.5480),
            json!({ "flownLeg": false, "endsRoute": true }),
            leg(47.4200, 8.5700),
        ];
        assert_eq!(flown_distance(&past_the_end), plain, "an item beyond the end of the route cannot lengthen the mission");

        // And a region of interest is a place with no leg to it, wherever it sits.
        let with_roi = vec![
            leg(47.3960, 8.5440),
            json!({ "flownLeg": false, "endsRoute": false, "coordinate": { "latitude": 47.4200, "longitude": 8.5700 } }),
            leg(47.3990, 8.5480),
        ];
        assert_eq!(flown_distance(&with_roi), plain, "an item that is not flown to adds no distance and no dogleg");
    }

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

    struct Plan(Value, f64, Value);

    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.unitsSettings.horizontalDistanceUnits.rawValue" => json!({ "kind": "value", "value": self.1 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "plan.controllerVehicle" => self.2.clone().to_string(),
                "plan.missionController" => self.0.to_string(),
                "units" => json!({ "kind": "object", "appSettingsHorizontalDistanceUnitsString": if self.1 == 0.0 { "ft" } else { "m" }, "appSettingsVerticalDistanceUnitsString": "m" }).to_string(),
                _ => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, path: &str, args: &str) -> String {
            let metres = || serde_json::from_str::<Vec<f64>>(args).unwrap()[0];
            match (path, self.1) {
                ("units.metersToAppSettingsHorizontalDistanceUnits", 0.0) => json!({ "ok": true, "result": metres() * FEET_PER_METRE }).to_string(),
                _ => String::new(),
            }
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn quad() -> Value {
        json!({ "kind": "object", "multiRotor": true, "vtol": false })
    }

    fn wing() -> Value {
        json!({ "kind": "object", "multiRotor": false, "vtol": false })
    }

    fn tiltrotor() -> Value {
        json!({ "kind": "object", "multiRotor": false, "vtol": true })
    }

    fn at(latitude: f64, longitude: f64) -> Value {
        json!({ "latitude": latitude, "longitude": longitude })
    }

    #[test]
    fn the_telemetry_terms_are_the_ones_the_answer_folds() {
        let plan = vec![
            json!({ "kind": "settings", "sequence": 0, "coordinate": at(50.0, 30.0), "flownLeg": true }),
            json!({ "kind": "waypoint", "sequence": 1, "coordinate": at(50.0, 30.0100), "flownLeg": true }),
            json!({ "kind": "roi", "sequence": 2, "coordinate": at(50.0, 30.0900), "flownLeg": false }),
            json!({ "kind": "survey", "sequence": 3, "coordinate": at(50.0, 30.0200), "flownLeg": true,
                    "exitCoordinate": at(50.0, 30.0200),
                    "geometry": { "transects": [at(50.0, 30.0250), at(50.0, 30.0700)] } }),
            json!({ "kind": "land", "sequence": 4, "coordinate": at(50.0, 30.0300), "flownLeg": true, "endsRoute": true }),
            json!({ "kind": "waypoint", "sequence": 5, "coordinate": at(50.0, 31.0000), "flownLeg": true }),
        ];

        let terms = telemetry_terms(&plan);
        let sequences: Vec<i64> = terms.iter().map(|(sequence, _, _)| *sequence).collect();
        assert_eq!(sequences, vec![1, 3, 4], "the ROI is not a flown leg, the settings entry is not somewhere the vehicle goes, and everything past the landing is not reached");

        let folded = terms.iter().map(|(_, from_home, span)| from_home.max(*span)).fold(0.0, f64::max);
        assert_eq!(max_telemetry_distance(&plan), folded, "the answer is the fold of these exact terms, so the inputs served beside it cannot describe a different walk");

        let survey = terms.iter().find(|(sequence, _, _)| *sequence == 3).unwrap();
        assert!(survey.2 > survey.1, "the survey's furthest transect is further from its own exit than its entry is from home, which is the term that makes this second and not a distance from launch at all");
        assert!(max_telemetry_distance(&plan) >= survey.2);
    }

    #[test]
    fn a_regime_the_airframe_does_not_have_is_absent_rather_than_zero() {
        let both = { let mut plan = flown(); plan["missionCruiseDistance"] = json!(0.0); plan };
        let rotor = summary_view(&Plan(both.clone(), 1.0, quad()), &[]);
        assert!(labelled(&rotor, "Cruise").is_none(), "a multirotor has no cruise regime, so a zero there is what the aircraft is and not something measured - and it lands in the cell the strip truncates first, spending width to say nothing");
        assert_eq!(labelled(&rotor, "Hover").unwrap(), "200 m");

        let fixed = { let mut plan = flown(); plan["missionHoverDistance"] = json!(0.0); plan };
        let plane = summary_view(&Plan(fixed, 1.0, wing()), &[]);
        assert!(labelled(&plane, "Hover").is_none(), "and symmetrically, a fixed wing does not hover");
        assert_eq!(labelled(&plane, "Cruise").unwrap(), "1.30 km");

        let vtol = summary_view(&Plan(both, 1.0, tiltrotor()), &[]);
        assert_eq!(labelled(&vtol, "Hover").unwrap(), "200 m", "a VTOL flies in both regimes, so both rows are real even when one reads zero");
        assert_eq!(labelled(&vtol, "Cruise").unwrap(), "0 m");
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
            "minAMSLAltitude": 480.0,
            "maxAMSLAltitude": 530.0,
        })
    }

    fn labelled(view: &Value, label: &str) -> Option<String> {
        view["rows"].as_array().unwrap().iter().find(|row| row["label"] == label).map(|row| row["value"].as_str().unwrap().to_string())
    }

    #[test]
    fn a_flown_plan_reports_what_it_will_cost_to_fly() {
        let view = summary_view(&Plan(flown(), 1.0, quad()), &[]);
        assert_eq!(view["available"], true);
        assert_eq!(labelled(&view, "Distance").unwrap(), "1.50 km");
        assert_eq!(labelled(&view, "Time").unwrap(), "3:05");
        assert_eq!(labelled(&view, "Furthest from launch").unwrap(), "640 m");
        assert_eq!(view["altitudeRange"]["text"], "480 m to 530 m");
        assert_eq!(view["distanceMetres"], 1500.0, "the raw metres travel too, so a head that wants to draw a bar is not parsing a string back");
    }

    #[test]
    fn the_same_plan_reads_in_feet_when_that_is_what_was_chosen() {
        let view = summary_view(&Plan(flown(), 0.0, quad()), &[]);
        assert_eq!(view["imperial"], true);
        assert_eq!(labelled(&view, "Distance").unwrap(), "4921 ft", "fifteen hundred metres is under a mile, so it reads in feet");
        let longer = summary_view(&Plan({ let mut plan = flown(); plan["missionTotalDistance"] = json!(3000.0); plan }, 0.0, quad()), &[]);
        assert_eq!(labelled(&longer, "Distance").unwrap(), "1.86 mi");
        assert_eq!(labelled(&view, "Time").unwrap(), "3:05", "time is not a unit the operator chooses");
        assert_eq!(view["distanceMetres"], 1500.0, "the raw number stays metric whichever way it is drawn");
        assert_eq!(view["altitudeRange"]["text"], "480 m to 530 m", "QGC keeps the vertical unit apart from the horizontal one, and an operator flying in feet over a map in miles still reads altitudes in the unit they set for altitudes");
    }

    #[test]
    fn a_row_the_controller_did_not_compute_is_left_out_rather_than_shown_as_zero() {
        let mut sparse = flown();
        sparse["missionHoverDistance"] = json!(-1.0);
        let view = summary_view(&Plan(sparse, 1.0, quad()), &[]);
        assert!(labelled(&view, "Hover").is_none(), "minus one is what this controller answers when it has not worked something out, and drawing it as a distance would be a lie");
        assert!(labelled(&view, "Distance").is_some());
    }

    #[test]
    fn an_empty_plan_says_so_rather_than_reporting_a_mission_of_nothing() {
        let empty = json!({ "kind": "object", "containsItems": false, "missionTotalDistance": 0.0, "missionTime": 0.0 });
        let view = summary_view(&Plan(empty, 1.0, quad()), &[]);
        assert_eq!(view["available"], false);
        assert_eq!(view["reason"], "This plan has no items yet.");
    }

    #[test]
    fn a_mission_below_sea_level_reads_as_a_depth_rather_than_as_nothing() {
        let mut low = flown();
        low["minAMSLAltitude"] = json!(-390.0);
        low["maxAMSLAltitude"] = json!(-340.0);
        let view = summary_view(&Plan(low, 1.0, quad()), &[]);
        assert_eq!(view["altitudeRange"]["text"], "-390 m to -340 m", "the Dead Sea is four hundred metres down and a plan flown over it still has altitudes");
        assert_eq!(view["altitudeRange"]["lowest"], -390.0);
    }

    #[test]
    fn an_altitude_range_the_wrong_way_round_is_no_range_at_all() {
        let mut wrong = flown();
        wrong["minAMSLAltitude"] = json!(530.0);
        wrong["maxAMSLAltitude"] = json!(480.0);
        assert_eq!(summary_view(&Plan(wrong, 1.0, quad()), &[])["altitudeRange"], Value::Null);
        let mut absent = flown();
        absent["maxAMSLAltitude"] = json!(f64::NAN.to_string());
        assert_eq!(summary_view(&Plan(absent, 1.0, quad()), &[])["altitudeRange"], Value::Null);
    }
}
