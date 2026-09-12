use serde_json::{Value, json};

use crate::read::{Unit, format_measure, object, refused};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "plan.missionController.missionItemCount",
    "plan.missionController.containsItems",
    "plan.dirty",
    "settings.unitsSettings.horizontalDistanceUnits",
];

const FIELDS: &str = "isSimpleItem,landingCoordinate,slopeStartCoordinate,finalApproachCoordinate,sequenceNumber";

fn place(item: &Value, key: &str) -> Option<Value> {
    let at = item.get(key)?;
    let number = |name: &str| at.get(name).and_then(Value::as_f64).filter(|value| value.is_finite());
    let (Some(latitude), Some(longitude)) = (number("latitude"), number("longitude")) else {
        return None;
    };
    match at.get("valid").and_then(Value::as_bool) == Some(true) && !(latitude == 0.0 && longitude == 0.0) {
        true => Some(json!({ "latitude": latitude, "longitude": longitude, "altitude": number("altitude") })),
        false => None,
    }
}

pub fn landing_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(index) = args.first().and_then(|a| a.parse::<usize>().ok()) else {
        return refused("view.landingPattern needs the index of the item in the plan, as view.landingPattern(4)");
    };
    let path = format!("plan.missionController.visualItems.{index}");
    let item = object(&backend.get_fields(&path, FIELDS));
    let (landing, slope_start, approach) = (place(&item, "landingCoordinate"), place(&item, "slopeStartCoordinate"), place(&item, "finalApproachCoordinate"));
    if landing.is_none() && slope_start.is_none() && approach.is_none() {
        return refused("that item draws no landing pattern; only a fixed wing or a VTOL gets one, and a multirotor land is a plain return");
    }
    let facts = object(&backend.get(&path));
    let fact = |property: &str| {
        facts
            .get("facts")
            .and_then(Value::as_array)
            .and_then(|list| list.iter().find(|f| f.get("property").and_then(Value::as_str) == Some(property)))
            .and_then(|f| f.get("value"))
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
    };
    let unit = Unit::horizontal(backend);
    let radius = fact("loiterRadius").filter(|metres| *metres > 0.0);
    json!({
        "kind": "object",
        "class": "LandingPattern",
        "index": index,
        "landing": landing,
        "slopeStart": slope_start,
        "finalApproach": approach,
        "loiterRadiusMetres": radius,
        "loiterRadiusText": radius.map(|metres| format_measure(unit.show(metres), &unit.name)),
        "loiterClockwise": fact("loiterClockwise").map(|turn| turn != 0.0),
        "loiterToAltitude": fact("useLoiterToAlt").map(|use_it| use_it != 0.0),
        "landingAltitudeMetres": fact("landingAltitude"),
        "landingHeadingDegrees": fact("landingHeading"),
        "landingDistanceMetres": fact("landingDistance"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Plan(Value);
    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.4" => self.0.to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "units" => json!({ "kind": "object", "appSettingsHorizontalDistanceUnitsString": "m" }).to_string(),
                _ => self.get(path),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn at(latitude: f64, longitude: f64) -> Value {
        json!({ "valid": true, "latitude": latitude, "longitude": longitude, "altitude": 50.0 })
    }

    fn pattern() -> Value {
        json!({ "kind": "object", "sequenceNumber": 5, "isSimpleItem": false,
                "landingCoordinate": at(47.400, 8.540),
                "slopeStartCoordinate": at(47.404, 8.546),
                "finalApproachCoordinate": at(47.410, 8.552),
                "facts": [
                    { "property": "loiterRadius", "value": 75.0 },
                    { "property": "loiterClockwise", "value": 1.0 },
                    { "property": "useLoiterToAlt", "value": 0.0 },
                    { "property": "landingAltitude", "value": 0.0 },
                    { "property": "landingHeading", "value": 130.0 },
                    { "property": "landingDistance", "value": 200.0 }
                ] })
    }

    #[test]
    fn the_three_places_a_landing_pattern_is_drawn_from_all_travel() {
        let view = landing_view(&Plan(pattern()), &["4".to_string()]);
        assert_eq!(view["landing"]["latitude"], 47.400);
        assert_eq!(view["slopeStart"]["longitude"], 8.546);
        assert_eq!(view["finalApproach"]["latitude"], 47.410);
        assert_eq!(view["loiterRadiusMetres"], 75.0);
        assert_eq!(view["loiterRadiusText"], "75.0 m");
        assert_eq!(view["loiterClockwise"], true, "the direction is a fact holding 0 or 1, and a circle drawn the wrong way round is an approach from the wrong side");
        assert_eq!(view["loiterToAltitude"], false);
        assert_eq!(view["landingHeadingDegrees"], 130.0);
        assert_eq!(view["landingAltitudeMetres"], 0.0, "a touchdown at field elevation is zero and that is a measurement, not an absence");
    }

    #[test]
    fn a_land_that_is_not_a_pattern_says_so_rather_than_answering_empty() {
        let plain = json!({ "kind": "object", "sequenceNumber": 5, "isSimpleItem": true, "facts": [] });
        let refusal = landing_view(&Plan(plain), &["4".to_string()]);
        assert_eq!(refusal["kind"], "null");
        assert!(refusal["reason"].as_str().unwrap().contains("multirotor"), "MissionController::insertLandItem builds a pattern only for a fixed wing or a VTOL, so a rotor's land is a plain return and a head asking for its pattern deserves that answer rather than silence");

        let missing = landing_view(&Plan(pattern()), &[]);
        assert!(missing["reason"].as_str().unwrap().contains("index"));
    }

    #[test]
    fn a_loiter_of_no_radius_is_not_a_circle() {
        let mut flat = pattern();
        flat["facts"][0] = json!({ "property": "loiterRadius", "value": 0.0 });
        let view = landing_view(&Plan(flat), &["4".to_string()]);
        assert_eq!(view["loiterRadiusMetres"], Value::Null, "a radius of zero draws a dot, and a head given the number would draw one rather than skip the circle");
        assert_eq!(view["loiterRadiusText"], Value::Null);
        assert_eq!(view["landing"]["latitude"], 47.400, "the rest of the pattern is unaffected");
    }

    #[test]
    fn an_unplaced_corner_is_absent_and_the_rest_still_draws() {
        let mut partial = pattern();
        partial["slopeStartCoordinate"] = json!({ "valid": false, "latitude": 0.0, "longitude": 0.0 });
        let view = landing_view(&Plan(partial), &["4".to_string()]);
        assert_eq!(view["slopeStart"], Value::Null, "an unset corner reads as 0,0 valid:false, which is the Gulf of Guinea if a head plots it");
        assert_eq!(view["landing"]["latitude"], 47.400, "and the corners that are placed still travel, so a partial pattern draws what it has");
    }
}
