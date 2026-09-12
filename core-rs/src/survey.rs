use serde_json::{Value, json};

use crate::read::{Unit, flag, object, refused};
use crate::router::Backend;

pub const DEPS: &[&str] = &["plan.missionController.missionItemCount", "plan.dirty"];
const ABSENT: &str = "\u{2014}";

pub fn interval_text(seconds: f64) -> String {
    match seconds.is_finite() && seconds > 0.0 {
        true => format!("{seconds:.1} s"),
        false => ABSENT.to_string(),
    }
}

pub fn warning(minimum_interval: f64, seconds_between_shots: f64) -> String {
    match minimum_interval > 0.0 && seconds_between_shots > 0.0 && seconds_between_shots < minimum_interval {
        true => format!("The camera needs {minimum_interval:.2} s between shots but the survey asks for {seconds_between_shots:.2} s."),
        false => String::new(),
    }
}

pub fn survey_stats_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(index) = args.first().and_then(|a| a.parse::<usize>().ok()) else { return refused("view.surveyStats needs the index of the item in the plan, as view.surveyStats(3) - the position in the list, not the sequence number") };
    let item_path = format!("plan.missionController.visualItems.{index}");
    let survey = object(&backend.get_fields(&item_path, "isSurveyItem,cameraShots,timeBetweenShots,coveredArea,complexDistance"));
    let is_survey = flag(&survey, "isSurveyItem");
    let number = |key: &str| survey.get(key).and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(0.0);
    let calc = object(&backend.get(&format!("{item_path}.cameraCalc")));
    let fact = |property: &str| calc.get("facts").and_then(Value::as_array).and_then(|f| f.iter().find(|x| x.get("property").and_then(Value::as_str) == Some(property)));
    let fact_number = |property: &str| fact(property).and_then(|f| f.get("value")).and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(0.0);
    let distance_unit = Unit::horizontal(backend);
    let fact_metres = |property: &str| fact(property).and_then(|f| f.get("rawValue")).and_then(Value::as_f64).filter(|v| v.is_finite());
    let (shots, seconds, area_m2, distance_m) = (number("cameraShots") as i64, number("timeBetweenShots"), number("coveredArea"), number("complexDistance"));
    let (side, frontal) = (fact_metres("adjustedFootprintSide").unwrap_or(0.0), fact_metres("adjustedFootprintFrontal").unwrap_or(0.0));
    let footprint_units = distance_unit.name.clone();
    let minimum_interval = fact_number("minTriggerInterval");
    let surface = fact_metres("distanceToSurface").filter(|metres| *metres > 0.0);
    let area = Unit::area(backend);
    json!({
        "kind": "object",
        "class": "SurveyStats",
        "index": index,
        "isSurvey": is_survey,
        "available": shots > 0 || area_m2 > 0.0,
        "shots": shots,
        "shotsText": if shots > 0 { shots.to_string() } else { ABSENT.to_string() },
        "secondsBetweenShots": seconds,
        "intervalText": interval_text(seconds),
        "areaSquareMetres": area_m2,
        "areaText": if area_m2 > 0.0 { crate::read::format_measure(area.show(area_m2), &area.name) } else { ABSENT.to_string() },
        "distanceMetres": distance_m,
        "distanceText": if distance_m > 0.0 { crate::missionsummary::distance_text(distance_m, crate::missionsummary::imperial(backend)) } else { ABSENT.to_string() },
        "footprintSide": side,
        "footprintFrontal": frontal,
        "footprintUnits": footprint_units,
        "footprintText": if side > 0.0 && frontal > 0.0 { format!("{:.1} \u{d7} {:.1} {footprint_units}", distance_unit.show(side), distance_unit.show(frontal)) } else { ABSENT.to_string() },
        "surfaceDistanceMetres": surface,
        "surfaceDistanceText": surface.map(|metres| crate::read::format_measure(distance_unit.show(metres), &distance_unit.name)),
        "minimumInterval": minimum_interval,
        "tooFast": !warning(minimum_interval, seconds).is_empty(),
        "warning": warning(minimum_interval, seconds),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_survey_that_shoots_faster_than_the_camera_allows_is_warned() {
        assert_eq!(warning(1.5, 1.0), "The camera needs 1.50 s between shots but the survey asks for 1.00 s.");
        assert_eq!(warning(1.5, 2.0), "");
        assert_eq!(warning(0.0, 0.5), "");
        assert_eq!(interval_text(0.0), "\u{2014}");
        assert_eq!(interval_text(2.25), "2.2 s");
    }

    #[test]
    fn the_view_reads_the_item_and_its_camera_calc() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, path: &str) -> String {
                match path.ends_with("cameraCalc") {
                    true => json!({ "kind": "object", "facts": [
                        { "property": "adjustedFootprintSide", "value": 12.5, "rawValue": 12.5, "units": "m" },
                        { "property": "adjustedFootprintFrontal", "value": 8.0, "rawValue": 8.0, "units": "m" },
                        { "property": "minTriggerInterval", "value": 2.0, "rawValue": 2.0 },
                    ] }),
                    false => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "plan.missionController.visualItems.3" => json!({ "kind": "object", "isSurveyItem": true, "cameraShots": 40, "timeBetweenShots": 1.5, "coveredArea": 20000.0, "complexDistance": 900.0 }),
                    _ => json!({ "kind": "object", "appSettingsAreaUnitsString": "m²", "appSettingsHorizontalDistanceUnitsString": "m" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true, "result": 1.0 }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = survey_stats_view(&Fake, &["3".to_string()]);
        assert_eq!(view["available"], true);
        assert_eq!(view["shotsText"], "40");
        assert_eq!(view["areaText"], "20000 m²");
        assert_eq!(view["distanceText"], "900 m");
        assert_eq!(survey_stats_view(&Fake, &["3".to_string()])["distanceText"], crate::missionsummary::distance_text(900.0, false), "a survey's own length is a ground distance and is spelled the way every other ground distance is");
        assert_eq!(view["footprintText"], "12.5 \u{d7} 8.0 m");
        assert_eq!(view["tooFast"], true);
        let refusal = survey_stats_view(&Fake, &[]);
        assert_eq!(refusal["kind"], "null", "the kind stays null so a head that already treats this as absent is unaffected");
        assert!(refusal["reason"].as_str().unwrap().contains("index"), "and the reason says which argument is missing, because a bare null is indistinguishable from a survey that has nothing to report");
    }

    #[test]
    fn the_height_a_pattern_flies_above_the_ground_is_named_as_that() {
        struct Camera(Option<f64>);
        impl Backend for Camera {
            fn get(&self, path: &str) -> String {
                let mut facts = vec![json!({ "property": "adjustedFootprintSide", "value": 10.0, "rawValue": 10.0, "units": "m" })];
                if let Some(metres) = self.0 {
                    facts.push(json!({ "property": "distanceToSurface", "value": metres, "rawValue": metres }));
                }
                match path.ends_with("cameraCalc") {
                    true => json!({ "kind": "object", "facts": facts }),
                    false => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "plan.missionController.visualItems.3" => json!({ "kind": "object", "isSurveyItem": true, "cameraShots": 40, "coveredArea": 20000.0 }),
                    _ => json!({ "kind": "object", "appSettingsAreaUnitsString": "m\u{b2}", "appSettingsHorizontalDistanceUnitsString": "m" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true, "result": 1.0 }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        let flying = survey_stats_view(&Camera(Some(60.0)), &["3".to_string()]);
        assert_eq!(flying["surfaceDistanceMetres"], 60.0, "a TransectStyleComplexItem has no altitude fact at all - the camera's distanceToSurface is what a survey flies at, and it is a height above the ground rather than the item altitude a waypoint shows in the same column");
        assert_eq!(flying["surfaceDistanceText"], "60.0 m");

        struct Feet(f64);
        impl Backend for Feet {
            fn get(&self, path: &str) -> String {
                match path.ends_with("cameraCalc") {
                    true => json!({ "kind": "object", "facts": [
                        { "property": "adjustedFootprintSide", "value": 10.0 * 3.2808399, "rawValue": 10.0 },
                        { "property": "distanceToSurface", "value": self.0 * 3.2808399, "rawValue": self.0 },
                    ] }),
                    false => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, path: &str, f: &str) -> String {
                match path {
                    "units" => json!({ "kind": "object", "appSettingsHorizontalDistanceUnitsString": "ft", "appSettingsAreaUnitsString": "ft\u{b2}" }).to_string(),
                    _ => Camera(Some(self.0)).get_fields(path, f),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                match path {
                    "units.metersToAppSettingsHorizontalDistanceUnits" => json!({ "ok": true, "result": serde_json::from_str::<Vec<f64>>(args).unwrap()[0] * 3.2808399 }).to_string(),
                    _ => json!({ "ok": true, "result": 1.0 }).to_string(),
                }
            }
            fn watch(&self, _p: &[String]) {}
        }
        let feet = survey_stats_view(&Feet(60.0), &["3".to_string()]);
        assert_eq!(feet["surfaceDistanceText"], "197 ft", "the fact's cooked value is already in the operator's unit, so converting it again squares the factor - 60 m must read 197 ft and not 646");
        assert_eq!(feet["surfaceDistanceMetres"], 60.0, "and the raw metres travel unconverted");

        let bare = survey_stats_view(&Camera(None), &["3".to_string()]);
        assert_eq!(bare["surfaceDistanceMetres"], Value::Null, "a camera that carries no such fact says nothing");
        assert_eq!(bare["surfaceDistanceText"], Value::Null);

        let unresolved = survey_stats_view(&Camera(Some(0.0)), &["3".to_string()]);
        assert_eq!(unresolved["surfaceDistanceMetres"], Value::Null, "and a camera holding zero has not resolved a distance either - a survey flown at no height above the ground is not a survey, so the zero is the absence rather than a measurement");
        assert_eq!(unresolved["surfaceDistanceText"], Value::Null);
    }

    #[test]
    fn a_corridor_scan_reports_its_camera_work_too() {
        struct Corridor;
        impl Backend for Corridor {
            fn get(&self, path: &str) -> String {
                match path.ends_with("cameraCalc") {
                    true => json!({ "kind": "object", "facts": [
                        { "property": "adjustedFootprintSide", "value": 9.0, "rawValue": 9.0, "units": "m" },
                        { "property": "adjustedFootprintFrontal", "value": 6.0, "rawValue": 6.0, "units": "m" },
                    ] }),
                    false => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "plan.missionController.visualItems.3" => json!({ "kind": "object", "isSurveyItem": false, "cameraShots": 18, "timeBetweenShots": 2.0, "coveredArea": 4000.0, "complexDistance": 600.0 }),
                    _ => json!({ "kind": "object", "appSettingsAreaUnitsString": "m\u{b2}", "appSettingsHorizontalDistanceUnitsString": "m" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true, "result": 1.0 }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = survey_stats_view(&Corridor, &["3".to_string()]);
        assert_eq!(view["available"], true, "isSurveyItem is final true on SurveyComplexItem alone, but cameraShots and cameraCalc live on TransectStyleComplexItem, which a corridor scan also inherits - gating on the type hid camera work the item really did");
        assert_eq!(view["shotsText"], "18");
        assert_eq!(view["footprintText"], "9.0 \u{d7} 6.0 m");
        assert_eq!(view["isSurvey"], false, "and the type still travels, so a head that wants to word it differently can");
    }

    #[test]
    fn a_footprint_nothing_has_measured_yet_reads_as_absent_rather_than_as_zero() {
        struct Unmeasured;
        impl Backend for Unmeasured {
            fn get(&self, path: &str) -> String {
                match path.ends_with("cameraCalc") {
                    true => json!({ "kind": "object", "facts": [
                        { "property": "adjustedFootprintSide", "value": 0.0, "rawValue": 0.0, "units": "m" },
                        { "property": "adjustedFootprintFrontal", "value": 0.0, "rawValue": 0.0, "units": "m" },
                    ] }),
                    false => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "plan.missionController.visualItems.3" => json!({ "kind": "object", "isSurveyItem": true, "cameraShots": 0, "coveredArea": 0.0, "complexDistance": 0.0 }),
                    _ => json!({ "kind": "object", "appSettingsAreaUnitsString": "m\u{b2}", "appSettingsHorizontalDistanceUnitsString": "m" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true, "result": 1.0 }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }

        let view = survey_stats_view(&Unmeasured, &["3".to_string()]);
        assert_eq!(view["footprintText"], ABSENT);
        assert_eq!(view["footprintSide"], 0.0, "the raw numbers still travel, so a head that wants to know it is zero can");
        assert_eq!(view["shotsText"], ABSENT);
        assert_eq!(view["areaText"], ABSENT);
        assert_eq!(view["available"], false, "nothing has been computed, so there is nothing for a panel to show");
    }
}
