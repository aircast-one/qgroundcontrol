use serde_json::{Value, json};

use crate::read::{object, refused};
use crate::router::Backend;

pub const DEPS: &[&str] = &["plan.missionController.currentPlanViewSeqNum", "plan.missionController.onlyInsertTakeoffValid", "plan.missionController.isInsertTakeoffValid", "plan.missionController.isInsertLandValid", "plan.missionController.flyThroughCommandsAllowed"];
const DEFAULT_AREA_METRES: f64 = 150.0;
const METRES_PER_DEGREE: f64 = 111_320.0;

pub struct Kind {
    pub id: &'static str,
    pub title: &'static str,
    pub invokable: &'static str,
    pub complex_name: Option<&'static str>,
    pub class_name: Option<&'static str>,
    pub geometry: Option<(&'static str, &'static str)>,
    pub placement_hint: &'static str,
}

pub const KINDS: &[Kind] = &[
    Kind { id: "waypoint", title: "Waypoint", invokable: "insertSimpleMissionItem", complex_name: None, class_name: None, geometry: None, placement_hint: "Click the map to place a waypoint." },
    Kind { id: "takeoff", title: "Takeoff", invokable: "insertTakeoffItem", complex_name: None, class_name: None, geometry: None, placement_hint: "Click the map to place a takeoff." },
    Kind { id: "land", title: "Land", invokable: "insertLandItem", complex_name: None, class_name: None, geometry: None, placement_hint: "Click the map to place a land." },
    Kind { id: "roi", title: "Region of Interest", invokable: "insertROIMissionItem", complex_name: None, class_name: None, geometry: None, placement_hint: "Click the map to place a region of interest." },
    Kind { id: "survey", title: "Survey", invokable: "insertComplexMissionItem", complex_name: Some("Survey"), class_name: Some("SurveyComplexItem"), geometry: Some(("area", "surveyAreaPolygon")), placement_hint: "Click the map to place a survey area." },
    Kind { id: "corridor", title: "Corridor Scan", invokable: "insertComplexMissionItem", complex_name: Some("Corridor Scan"), class_name: Some("CorridorScanComplexItem"), geometry: Some(("line", "corridorPolyline")), placement_hint: "Click the map to place a corridor to scan along." },
    Kind { id: "structure", title: "Structure Scan", invokable: "insertComplexMissionItem", complex_name: Some("Structure Scan"), class_name: Some("StructureScanComplexItem"), geometry: Some(("area", "structurePolygon")), placement_hint: "Click the map to place a structure to scan around." },
];

impl Kind {
    pub fn shape_noun(&self) -> &str {
        match self.geometry {
            Some(("line", _)) => "path",
            Some(("area", _)) => "area",
            _ => "shape",
        }
    }
}

pub fn lookup(id_or_name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.id == id_or_name || k.complex_name == Some(id_or_name))
}

pub fn by_class(class: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.class_name == Some(class))
}

const NEEDS_TAKEOFF_FIRST: &str = "This mission starts from the ground, so a takeoff has to come before anything else.";
const ALREADY_TAKES_OFF: &str = "The mission already takes off before this point.";
const LAND_COMES_LAST: &str = "A landing goes after the takeoff and after every place the vehicle flies through.";
const NOT_AFTER_LANDING: &str = "The vehicle has already landed at this point in the mission.";

pub struct Insertable {
    pub at_sequence: Option<i64>,
    pub only_takeoff: bool,
    pub takeoff: bool,
    pub land: bool,
    pub fly_through: bool,
}

pub fn insertable(backend: &dyn Backend) -> Insertable {
    let mission = object(&backend.get_fields(
        "plan.missionController",
        "currentPlanViewSeqNum,onlyInsertTakeoffValid,isInsertTakeoffValid,isInsertLandValid,flyThroughCommandsAllowed",
    ));
    let answered = |key: &str, unset: bool| mission.get(key).and_then(Value::as_bool).unwrap_or(unset);
    Insertable {
        at_sequence: mission.get("currentPlanViewSeqNum").and_then(Value::as_i64).filter(|sequence| *sequence >= 0),
        only_takeoff: answered("onlyInsertTakeoffValid", true),
        takeoff: answered("isInsertTakeoffValid", true),
        land: answered("isInsertLandValid", false),
        fly_through: answered("flyThroughCommandsAllowed", true),
    }
}

pub fn refusal(kind: &Kind, insertable: &Insertable) -> Option<&'static str> {
    match kind.id {
        "takeoff" if !insertable.takeoff => Some(ALREADY_TAKES_OFF),
        "takeoff" => None,
        _ if insertable.only_takeoff => Some(NEEDS_TAKEOFF_FIRST),
        "land" if !insertable.land => Some(LAND_COMES_LAST),
        "land" => None,
        "roi" => None,
        _ if !insertable.fly_through => Some(NOT_AFTER_LANDING),
        _ => None,
    }
}

fn kind_json(kind: &Kind) -> Value {
    json!({
        "id": kind.id,
        "title": kind.title,
        "invokable": kind.invokable,
        "complexName": kind.complex_name,
        "className": kind.class_name,
        "geometry": kind.geometry.map(|(shape, _)| shape),
        "geometryProperty": kind.geometry.map(|(_, property)| property),
        "shapeNoun": match kind.geometry { Some(("line", _)) => "path", Some(("area", _)) => "area", _ => "shape" },
        "placementHint": kind.placement_hint,
        "simple": kind.complex_name.is_none(),
    })
}

fn offered(kind: &Kind, insertable: &Insertable) -> Value {
    let refused = refusal(kind, insertable);
    let mut json = kind_json(kind);
    let asked = insertable.at_sequence.is_some();
    json["enabled"] = if asked { json!(refused.is_none()) } else { Value::Null };
    json["disabledReason"] = match asked {
        true => refused.map(|reason| json!(reason)).unwrap_or(Value::Null),
        false => Value::Null,
    };
    json["atSequence"] = insertable.at_sequence.map(|sequence| json!(sequence)).unwrap_or(Value::Null);
    json
}

pub fn kinds_view(backend: &dyn Backend, args: &[String]) -> Value {
    let insertable = insertable(backend);
    match args.first() {
        Some(wanted) => lookup(wanted).map(|kind| offered(kind, &insertable)).unwrap_or_else(|| {
            crate::read::refused(&format!("no mission kind is called {wanted}; this takes a kind id or a complex item name, one of {}", KINDS.iter().map(|kind| kind.id).collect::<Vec<_>>().join(", ")))
        }),
        None => json!({ "kind": "object", "class": "MissionKinds", "kinds": KINDS.iter().map(|kind| offered(kind, &insertable)).collect::<Vec<_>>() }),
    }
}

pub fn default_area(latitude: f64, longitude: f64) -> Vec<(f64, f64)> {
    let latitude_span = DEFAULT_AREA_METRES / METRES_PER_DEGREE;
    let longitude_span = DEFAULT_AREA_METRES / (METRES_PER_DEGREE * latitude.to_radians().cos().max(0.01));
    [
        (latitude - latitude_span, longitude - longitude_span),
        (latitude - latitude_span, longitude + longitude_span),
        (latitude + latitude_span, longitude + longitude_span),
        (latitude + latitude_span, longitude - longitude_span),
    ]
    .into_iter()
    .map(|(lat, lon)| crate::geo::wrap(lat, lon))
    .collect()
}

pub fn default_line(latitude: f64, longitude: f64) -> Vec<(f64, f64)> {
    let span = DEFAULT_AREA_METRES / METRES_PER_DEGREE;
    [(latitude - span, longitude), (latitude + span, longitude)].into_iter().map(|(lat, lon)| crate::geo::wrap(lat, lon)).collect()
}

pub fn seed_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let number = |i: usize| args.get(i).and_then(|a| a.parse::<f64>().ok()).filter(|v| v.is_finite());
    let (Some(kind), Some(latitude), Some(longitude)) = (args.first().and_then(|k| lookup(k)), number(1), number(2)) else {
        return refused("view.seed needs a kind from view.missionKinds and a place, as view.seed(survey,47.4,8.5)");
    };
    let Some((shape, property)) = kind.geometry else { return refused("that kind draws no shape, so there is nothing to seed") };
    let points = match shape {
        "line" => default_line(latitude, longitude),
        _ => default_area(latitude, longitude),
    };
    json!({
        "kind": "object",
        "class": "MissionSeed",
        "id": kind.id,
        "property": property,
        "points": points.iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon, "altitude": 0 })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_argument_that_names_no_kind_says_what_it_wanted() {
        struct Nothing;
        impl Backend for Nothing {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let refused = kinds_view(&Nothing, &["1".to_string()]);
        assert_eq!(refused["kind"], "null");
        let reason = refused["reason"].as_str().expect("a refusal without a reason is the one case where the caller most needs to be told what the view wanted, and this one answered a bare null while landingPattern and control both answer a sentence");
        assert!(reason.contains("survey"), "naming the ids it accepts is what turns the refusal into an answer: {reason}");
        assert!(kinds_view(&Nothing, &["survey".to_string()])["id"] == "survey", "and a real id still resolves, so the refusal is not swallowing everything");
    }

    #[test]
    fn the_simple_kinds_come_first_and_only_the_complex_ones_carry_a_shape() {
        let (simple, complex): (Vec<&Kind>, Vec<&Kind>) = KINDS.iter().partition(|kind| kind.geometry.is_none());
        assert_eq!(simple.iter().map(|k| k.id).collect::<Vec<_>>(), vec!["waypoint", "takeoff", "land", "roi"]);
        assert_eq!(complex.iter().map(|k| k.id).collect::<Vec<_>>(), vec!["survey", "corridor", "structure"]);
        assert_eq!(&KINDS[..simple.len()].iter().map(|k| k.id).collect::<Vec<_>>(), &simple.iter().map(|k| k.id).collect::<Vec<_>>(), "the simple kinds are the leading run, which is what makes items 0 to 3 of view.missionKinds carry no className, complexName or geometry");
        assert!(simple.iter().all(|k| k.class_name.is_none() && k.complex_name.is_none()), "a kind with no shape has no complex class behind it either, so those three fields are absent together or not at all");
        assert!(complex.iter().all(|k| k.class_name.is_some() && k.complex_name.is_some()));
    }

    struct Nothing;
    impl Backend for Nothing {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_catalogue_names_the_invokable_and_the_geometry() {
        let all = kinds_view(&Nothing, &[]);
        assert_eq!(all["kinds"].as_array().unwrap().len(), 7);
        let corridor = kinds_view(&Nothing, &["Corridor Scan".to_string()]);
        assert_eq!(corridor["id"], "corridor");
        assert_eq!(corridor["invokable"], "insertComplexMissionItem");
        assert_eq!(corridor["geometryProperty"], "corridorPolyline");
        assert_eq!(corridor["shapeNoun"], "path");
        let waypoint = kinds_view(&Nothing, &["waypoint".to_string()]);
        assert_eq!(waypoint["simple"], true);
        assert_eq!(waypoint["geometry"], Value::Null);
        assert_eq!(kinds_view(&Nothing, &["nope".to_string()])["kind"], "null");
    }

    #[test]
    fn a_seed_is_a_square_for_areas_and_a_north_south_line_for_corridors() {
        let survey = seed_view(&Nothing, &["survey".to_string(), "47.0".to_string(), "8.0".to_string()]);
        assert_eq!(survey["property"], "surveyAreaPolygon");
        assert_eq!(survey["points"].as_array().unwrap().len(), 4);
        let width = survey["points"][1]["longitude"].as_f64().unwrap() - survey["points"][0]["longitude"].as_f64().unwrap();
        assert!(width > 2.0 * DEFAULT_AREA_METRES / METRES_PER_DEGREE);
        let corridor = seed_view(&Nothing, &["corridor".to_string(), "0".to_string(), "0".to_string()]);
        assert_eq!(corridor["points"].as_array().unwrap().len(), 2);
        assert_eq!(corridor["points"][0]["longitude"], 0.0);
        assert_eq!(seed_view(&Nothing, &["waypoint".to_string(), "0".to_string(), "0".to_string()])["kind"], "null");
        assert_eq!(seed_view(&Nothing, &["survey".to_string()])["kind"], "null");
    }

    #[test]
    fn a_seed_beside_the_dateline_stays_on_the_map() {
        let area = default_area(-16.5, 179.999);
        assert!(area.iter().all(|(lat, lon)| (-90.0..=90.0).contains(lat) && (-180.0..=180.0).contains(lon)), "a survey seeded in Fiji must not carry a longitude no autopilot will accept: {area:?}");
        assert!(area.iter().any(|(_, lon)| *lon < 0.0) && area.iter().any(|(_, lon)| *lon > 0.0), "the area still straddles the dateline rather than collapsing to one side");
        let pole = default_area(89.999, 8.5);
        assert!(pole.iter().all(|(lat, _)| (-90.0..=90.0).contains(lat)));
        let line = default_line(89.999, 179.999);
        assert!(line.iter().all(|(lat, lon)| (-90.0..=90.0).contains(lat) && (-180.0..=180.0).contains(lon)));
        let ordinary = default_area(47.4, 8.5);
        assert!(ordinary.iter().all(|(lat, lon)| *lat > 47.0 && *lat < 48.0 && *lon > 8.0 && *lon < 9.0), "an ordinary seed is unchanged");
    }
}

#[cfg(test)]
mod offering {
    use super::*;

    struct Mission(Value);
    impl Backend for Mission {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "plan.missionController" => self.0.to_string(),
                _ => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn offering(mission: Value) -> Vec<(String, Option<bool>, String)> {
        kinds_view(&Mission(mission), &[])["kinds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|kind| {
                (
                    kind["id"].as_str().unwrap().to_string(),
                    kind["enabled"].as_bool(),
                    kind["disabledReason"].as_str().unwrap_or("").to_string(),
                )
            })
            .collect()
    }

    fn state(only_takeoff: bool, takeoff: bool, land: bool, fly_through: bool) -> Value {
        json!({
            "kind": "object",
            "currentPlanViewSeqNum": 1,
            "onlyInsertTakeoffValid": only_takeoff,
            "isInsertTakeoffValid": takeoff,
            "isInsertLandValid": land,
            "flyThroughCommandsAllowed": fly_through,
        })
    }

    fn named<'a>(offered: &'a [(String, Option<bool>, String)], id: &str) -> &'a (String, Option<bool>, String) {
        offered.iter().find(|(kind, _, _)| kind == id).unwrap()
    }

    #[test]
    fn an_empty_ground_mission_offers_only_a_takeoff() {
        let offered = offering(state(true, true, false, true));
        assert_eq!(named(&offered, "takeoff").1, Some(true), "the one thing that can be added is the one thing offered");
        assert_eq!(named(&offered, "waypoint").1, Some(false));
        assert_eq!(named(&offered, "waypoint").2, NEEDS_TAKEOFF_FIRST);
        assert_eq!(named(&offered, "survey").1, Some(false), "a complex item is no more insertable than a waypoint before a takeoff");
        assert_eq!(offered.iter().filter(|(_, enabled, _)| *enabled == Some(true)).count(), 1);
    }

    #[test]
    fn a_mission_that_already_takes_off_offers_everything_but_another_takeoff() {
        let offered = offering(state(false, false, true, true));
        assert_eq!(named(&offered, "takeoff").1, Some(false));
        assert_eq!(named(&offered, "takeoff").2, ALREADY_TAKES_OFF);
        assert_eq!(named(&offered, "waypoint").1, Some(true));
        assert_eq!(named(&offered, "land").1, Some(true));
        assert_eq!(named(&offered, "survey").1, Some(true));
        assert!(offered.iter().filter(|(kind, _, _)| kind != "takeoff").all(|(_, enabled, _)| *enabled == Some(true)));
    }

    #[test]
    fn a_landing_is_withheld_until_it_would_come_last() {
        let offered = offering(state(false, false, false, true));
        assert_eq!(named(&offered, "land").1, Some(false));
        assert_eq!(named(&offered, "land").2, LAND_COMES_LAST);
        assert_eq!(named(&offered, "waypoint").1, Some(true), "the rest of the mission is still editable at a point a landing cannot go");
    }

    #[test]
    fn nothing_the_vehicle_would_fly_through_is_offered_after_it_has_landed() {
        let offered = offering(state(false, false, false, false));
        assert_eq!(named(&offered, "waypoint").1, Some(false));
        assert_eq!(named(&offered, "waypoint").2, NOT_AFTER_LANDING);
        assert_eq!(named(&offered, "survey").1, Some(false));
        assert_eq!(named(&offered, "roi").1, Some(true), "a region of interest is a camera instruction rather than a place to fly, so it is still allowed");
    }

    #[test]
    fn a_plan_view_that_has_never_selected_anything_answers_that_it_does_not_know() {
        let offered = offering(json!({ "kind": "object" }));
        assert!(offered.iter().all(|(_, enabled, _)| enabled.is_none()), "these flags are only assigned when the plan view selects an item, so a head that never selects one would otherwise read the controller's constructor values as a verdict, and they say a landing can never be added");
        assert!(offered.iter().all(|(_, _, reason)| reason.is_empty()), "nothing is refused, so nothing has a reason to show");
    }

    #[test]
    fn a_verdict_says_which_point_in_the_mission_it_is_about() {
        let asked = kinds_view(&Mission(state(false, false, true, true)), &[]);
        assert_eq!(asked["kinds"][0]["atSequence"], 1);
        let unasked = kinds_view(&Mission(json!({ "kind": "object" })), &[]);
        assert_eq!(unasked["kinds"][0]["atSequence"], Value::Null, "what can be inserted depends on where, so an answer with no where is not an answer");
    }

    #[test]
    fn one_kind_asked_for_by_name_carries_the_same_verdict_as_the_list() {
        let mission = Mission(state(false, false, true, true));
        let alone = kinds_view(&mission, &["takeoff".to_string()]);
        assert_eq!(alone["enabled"], false);
        assert_eq!(alone["disabledReason"], ALREADY_TAKES_OFF);
        let land = kinds_view(&mission, &["Survey".to_string()]);
        assert_eq!(land["enabled"], true);
        assert_eq!(land["disabledReason"], Value::Null);
    }
}
