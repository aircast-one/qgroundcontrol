use serde_json::{Value, json};

use crate::read::{flag, integer, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["plan.missionController.visualItems", "plan.missionController.currentPlanViewVIIndex", "plan.missionController.containsItems"];

const FIELDS: &str = "sequenceNumber,abbreviation,commandName,commandDescription,isCurrentItem,specifiesCoordinate,isStandaloneCoordinate,specifiesAltitudeOnly,isSimpleItem,isTakeoffItem,isLandCommand,isSurveyItem,homePosition,coordinate,amslEntryAlt,altDifference,azimuth,distance,distanceFromStart,readyForSaveState,readyForSaveMessage,dirty";

const READY_TO_SAVE: i64 = 0;

pub fn items_view(backend: &dyn Backend, _args: &[String]) -> Value {
    // A plan always holds its settings entry, so a count of one is an empty plan rather than a
    // plan with something in it. The controller answers that question itself.
    let count = integer(&object(&backend.get("plan.missionController.visualItems.count")), "value").unwrap_or(0);
    let has_items = flag(&object(&backend.get_fields("plan.missionController", "containsItems")), "containsItems");
    if count <= 0 {
        return json!({ "kind": "object", "class": "MissionItems", "available": false, "items": [], "current": -1, "reason": "This plan has no items yet." });
    }
    let current = integer(&object(&backend.get("plan.missionController.currentPlanViewVIIndex")), "value").unwrap_or(-1);
    let items: Vec<Value> = (0..count)
        .map(|index| item(&object(&backend.get_fields(&format!("plan.missionController.visualItems.{index}"), FIELDS)), index))
        .collect();
    json!({
        "kind": "object",
        "class": "MissionItems",
        "available": has_items,
        "current": current,
        "items": items,
        "reason": match has_items {
            true => "",
            false => "This plan has no items yet.",
        },
    })
}

fn item(read: &Value, index: i64) -> Value {
    let coordinate = read.get("coordinate").cloned().filter(|at| at.get("valid").and_then(Value::as_bool) == Some(true));
    let ready = integer(read, "readyForSaveState");
    json!({
        "index": index,
        "sequence": integer(read, "sequenceNumber"),
        "abbreviation": text(read, "abbreviation"),
        "name": text(read, "commandName"),
        "description": text(read, "commandDescription"),
        "kind": kind(read),
        "current": flag(read, "isCurrentItem"),
        "coordinate": coordinate,
        "altitudeAmsl": number(read, "amslEntryAlt"),
        "altitudeChange": number(read, "altDifference"),
        "azimuth": number(read, "azimuth"),
        "distance": number(read, "distance"),
        "distanceFromStart": number(read, "distanceFromStart"),
        "edited": flag(read, "dirty"),
        "blocked": ready.is_some_and(|state| state != READY_TO_SAVE),
        "blockedReason": match ready {
            Some(state) if state != READY_TO_SAVE => Some(text(read, "readyForSaveMessage")).filter(|message| !message.is_empty()),
            _ => None,
        },
    })
}

fn number(read: &Value, key: &str) -> Option<f64> {
    read.get(key).and_then(Value::as_f64).filter(|value| value.is_finite())
}

fn kind(read: &Value) -> &'static str {
    if flag(read, "homePosition") {
        return "settings";
    }
    if flag(read, "isSurveyItem") {
        return "survey";
    }
    if flag(read, "isTakeoffItem") {
        return "takeoff";
    }
    if flag(read, "isLandCommand") {
        return "land";
    }
    if !flag(read, "isSimpleItem") {
        return "complex";
    }
    if flag(read, "specifiesAltitudeOnly") {
        return "altitude";
    }
    match flag(read, "specifiesCoordinate") && !flag(read, "isStandaloneCoordinate") {
        true => "waypoint",
        false => "command",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Plan(Vec<Value>, i64);

    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": self.0.len() }).to_string(),
                "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": self.1 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            if path == "plan.missionController" {
                return json!({ "kind": "object", "containsItems": self.0.len() > 1 }).to_string();
            }
            match path.rsplit_once('.').and_then(|(_, index)| index.parse::<usize>().ok()).and_then(|index| self.0.get(index)) {
                Some(item) => item.to_string(),
                None => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn at(latitude: f64, longitude: f64) -> Value {
        json!({ "kind": "coordinate", "valid": true, "latitude": latitude, "longitude": longitude, "altitude": 50.0 })
    }

    fn settings() -> Value {
        json!({ "kind": "object", "homePosition": true, "sequenceNumber": 0, "abbreviation": "Launch", "commandName": "Mission Settings", "isSimpleItem": false })
    }

    fn takeoff() -> Value {
        json!({
            "kind": "object", "sequenceNumber": 1, "abbreviation": "Takeoff", "commandName": "Takeoff",
            "isSimpleItem": true, "isTakeoffItem": true, "specifiesCoordinate": true, "coordinate": at(47.0, 8.0),
            "amslEntryAlt": 520.0, "distance": 0.0, "distanceFromStart": 0.0, "readyForSaveState": 0,
        })
    }

    fn waypoint(sequence: i64, distance: f64) -> Value {
        json!({
            "kind": "object", "sequenceNumber": sequence, "abbreviation": sequence.to_string(), "commandName": "Waypoint",
            "isSimpleItem": true, "specifiesCoordinate": true, "coordinate": at(47.1, 8.1),
            "amslEntryAlt": 540.0, "altDifference": 20.0, "azimuth": 45.0, "distance": distance,
            "distanceFromStart": distance, "readyForSaveState": 0, "dirty": true,
        })
    }

    fn survey() -> Value {
        json!({
            "kind": "object", "sequenceNumber": 3, "abbreviation": "S", "commandName": "Survey",
            "isSimpleItem": false, "isSurveyItem": true, "specifiesCoordinate": true, "coordinate": at(47.2, 8.2),
            "readyForSaveState": 2, "readyForSaveMessage": "The survey needs an area before it can be saved.",
        })
    }

    #[test]
    fn a_plan_lists_its_items_once_rather_than_a_path_per_property() {
        let view = items_view(&Plan(vec![settings(), takeoff(), waypoint(2, 400.0)], 1), &[]);
        assert_eq!(view["available"], true);
        let items = view["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["kind"], "settings");
        assert_eq!(items[1]["kind"], "takeoff");
        assert_eq!(items[2]["kind"], "waypoint");
        assert_eq!(items[2]["distance"], 400.0);
        assert_eq!(items[2]["azimuth"], 45.0);
        assert_eq!(items[1]["coordinate"]["latitude"], 47.0);
        assert_eq!(view["current"], 1);
    }

    #[test]
    fn an_item_that_cannot_be_saved_says_so_and_says_why() {
        let view = items_view(&Plan(vec![settings(), survey()], 1), &[]);
        let survey = &view["items"].as_array().unwrap()[1];
        assert_eq!(survey["kind"], "survey");
        assert_eq!(survey["blocked"], true);
        assert_eq!(survey["blockedReason"], "The survey needs an area before it can be saved.");
        let fine = items_view(&Plan(vec![settings(), takeoff()], 1), &[]);
        assert_eq!(fine["items"][1]["blocked"], false);
        assert_eq!(fine["items"][1]["blockedReason"], Value::Null, "an item with nothing wrong carries no reason, so a head drawing one is drawing a real problem");
    }

    #[test]
    fn an_item_with_no_place_on_the_map_carries_no_coordinate_rather_than_a_zero_one() {
        let command = json!({ "kind": "object", "sequenceNumber": 4, "abbreviation": "ROI", "commandName": "Region of interest", "isSimpleItem": true, "specifiesCoordinate": false, "coordinate": { "kind": "coordinate", "valid": false, "latitude": 0.0, "longitude": 0.0 } });
        let view = items_view(&Plan(vec![settings(), command], 0), &[]);
        let listed = &view["items"].as_array().unwrap()[1];
        assert_eq!(listed["coordinate"], Value::Null, "an invalid coordinate drawn on a map is a pin in the Atlantic");
        assert_eq!(listed["kind"], "command");
    }

    #[test]
    fn a_plan_holding_only_its_settings_entry_is_an_empty_plan() {
        let bare = items_view(&Plan(vec![settings()], 0), &[]);
        assert_eq!(bare["available"], false, "removing everything leaves the settings entry, so a count of one is not a plan with something in it");
        assert_eq!(bare["items"].as_array().unwrap().len(), 1, "the entry is still listed, because a head may want to draw the planned home position");
        assert!(!bare["reason"].as_str().unwrap().is_empty());

        let none = items_view(&Plan(Vec::new(), -1), &[]);
        assert_eq!(none["available"], false);
        assert_eq!(none["items"].as_array().unwrap().len(), 0);
        assert_eq!(none["current"], -1);
    }

    #[test]
    fn a_complex_item_the_catalogue_does_not_name_is_still_listed_as_complex() {
        let pattern = json!({ "kind": "object", "sequenceNumber": 5, "abbreviation": "FWL", "commandName": "Fixed Wing Landing", "isSimpleItem": false });
        let view = items_view(&Plan(vec![settings(), pattern], 1), &[]);
        assert_eq!(view["items"][1]["kind"], "complex", "an item type the core has no entry for still has to draw as something rather than as a waypoint");
        assert_eq!(view["items"][1]["name"], "Fixed Wing Landing");
    }
}
