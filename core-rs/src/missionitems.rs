use serde_json::{Value, json};

use crate::read::{flag, integer, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["plan.missionController.visualItems.count", "plan.missionController.currentPlanViewVIIndex", "plan.missionController.containsItems"];

const FIELDS: &str = "sequenceNumber,abbreviation,commandName,commandDescription,isCurrentItem,specifiesCoordinate,isStandaloneCoordinate,specifiesAltitudeOnly,isSimpleItem,isTakeoffItem,isLandCommand,isSurveyItem,homePosition,coordinate,amslEntryAlt,altDifference,azimuth,distance,distanceFromStart,readyForSaveState,readyForSaveMessage,dirty,altitude,altitudeMode,isIncomplete,exitCoordinate,exitCoordinateSameAsEntry,commandName";

const READY_TO_SAVE: i64 = 0;
const AWAITING_TERRAIN: i64 = 1;

pub fn items_view(backend: &dyn Backend, args: &[String]) -> Value {
    let everything = args.iter().any(|arg| arg == "fields");
    let shapes = args.iter().any(|arg| arg == "geometry");
    let count = integer(&object(&backend.get("plan.missionController.visualItems.count")), "value").unwrap_or(0);
    let has_items = flag(&object(&backend.get_fields("plan.missionController", "containsItems")), "containsItems");
    if count <= 0 {
        return json!({ "kind": "object", "class": "MissionItems", "available": false, "items": [], "current": -1, "reason": "This plan has no items yet." });
    }
    let current = integer(&object(&backend.get("plan.missionController.currentPlanViewVIIndex")), "value").unwrap_or(-1);
    let listed = object(&backend.get_fields("plan.missionController.visualItems", FIELDS));
    let items: Vec<Value> = match listed.get("elements").and_then(Value::as_array) {
        Some(elements) => elements.iter().enumerate().map(|(index, element)| item(element, index as i64)).collect(),
        None => (0..count)
            .map(|index| item(&object(&backend.get_fields(&format!("plan.missionController.visualItems.{index}"), FIELDS)), index))
            .collect(),
    };
    let items: Vec<Value> = match shapes {
        false => items,
        true => items
            .into_iter()
            .enumerate()
            .map(|(index, mut listed)| {
                listed["geometry"] = geometry_of(backend, index as i64, listed["kind"].as_str().unwrap_or_default());
                listed
            })
            .collect(),
    };
    let items: Vec<Value> = match everything {
        false => items,
        true => items
            .into_iter()
            .enumerate()
            .map(|(index, mut listed)| {
                listed["fields"] = fields_of(backend, index as i64);
                listed
            })
            .collect(),
    };
    json!({
        "kind": "object",
        "class": "MissionItems",
        "available": has_items,
        "editing": editable(backend, current),
        "current": current,
        "items": items,
        "reason": match has_items {
            true => "",
            false => "This plan has no items yet.",
        },
    })
}

fn placed(read: &Value) -> Option<Value> {
    at_key(read, "coordinate")
}

fn at_key(read: &Value, key: &str) -> Option<Value> {
    if !flag(read, "specifiesCoordinate") {
        return None;
    }
    let at = read.get(key)?;
    let number = |key: &str| at.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
    let (Some(latitude), Some(longitude)) = (number("latitude"), number("longitude")) else {
        return None;
    };
    let unset = latitude == 0.0 && longitude == 0.0;
    (at.get("valid").and_then(Value::as_bool) == Some(true) && !unset).then(|| at.clone())
}

fn item(read: &Value, index: i64) -> Value {
    let coordinate = placed(read);
    let exit = match flag(read, "exitCoordinateSameAsEntry") {
        true => None,
        false => at_key(read, "exitCoordinate").filter(|exit| Some(exit) != coordinate.as_ref()),
    };
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
        "exitCoordinate": exit,
        "altitude": fact_number(read, "altitude"),
        "altitudeMode": number(read, "altitudeMode").map(|mode| mode as i64),
        "altitudeAmsl": number(read, "amslEntryAlt"),
        "altitudeChange": number(read, "altDifference"),
        "azimuth": number(read, "azimuth"),
        "distance": number(read, "distance"),
        "distanceFromStart": number(read, "distanceFromStart"),
        "edited": flag(read, "dirty"),
        "incomplete": flag(read, "isIncomplete"),
        "endsRoute": flag(read, "isLandCommand") || text(read, "commandName").contains("Return"),
        "flownLeg": flag(read, "specifiesCoordinate") && !flag(read, "isStandaloneCoordinate") && !flag(read, "isIncomplete"),
        "blocked": ready.is_some_and(|state| state != READY_TO_SAVE && state != AWAITING_TERRAIN),
        "awaitingTerrain": ready == Some(AWAITING_TERRAIN),
        "blockedReason": match ready {
            Some(state) if state != READY_TO_SAVE => Some(text(read, "readyForSaveMessage")).filter(|message| !message.is_empty()),
            _ => None,
        },
    })
}

fn editable(backend: &dyn Backend, current: i64) -> Value {
    if current <= 0 {
        return Value::Null;
    }
    match fields_of(backend, current) {
        Value::Null => Value::Null,
        fields => json!({ "index": current, "fields": fields }),
    }
}

fn geometry_of(backend: &dyn Backend, index: i64, kind: &str) -> Value {
    let Some(shape) = crate::missionkinds::lookup(kind).and_then(|kind| kind.geometry) else {
        return Value::Null;
    };
    let vertices = object(&backend.get(&format!("plan.missionController.visualItems.{index}.{}.path", shape.1)));
    let listed: Vec<Value> = vertices
        .get("value")
        .and_then(Value::as_array)
        .map(|points| {
            points
                .iter()
                .filter_map(|at| Some(json!({ "latitude": at.get("latitude")?.as_f64()?, "longitude": at.get("longitude")?.as_f64()? })))
                .collect()
        })
        .unwrap_or_default();
    match listed.is_empty() {
        true => Value::Null,
        false => json!({ "shape": shape.0, "property": shape.1, "vertices": listed }),
    }
}

fn fields_of(backend: &dyn Backend, index: i64) -> Value {
    if index <= 0 {
        return Value::Null;
    }
    let path = format!("plan.missionController.visualItems.{index}");
    let item = object(&backend.get(&path));
    let Some(facts) = item.get("facts").and_then(Value::as_array) else {
        return Value::Null;
    };
    let fields: Vec<Value> = facts
        .iter()
        .filter_map(|fact| {
            let property = fact.get("property").and_then(Value::as_str).filter(|property| !property.is_empty())?;
            let name = fact.get("name").and_then(Value::as_str).filter(|name| !name.is_empty()).unwrap_or(property);
            Some(json!({
                "name": property,
                "label": name,
                "units": fact.get("units").and_then(Value::as_str).filter(|units| !units.is_empty()),
                "value": fact.get("value"),
                "text": fact.get("enumOrValueString").and_then(Value::as_str),
                "choices": fact.get("enumStrings"),
                "path": format!("{path}.{property}"),
            }))
        })
        .collect();
    match fields.is_empty() {
        true => Value::Null,
        false => Value::Array(fields),
    }
}

fn fact_number(read: &Value, name: &str) -> Option<f64> {
    read.get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| facts.iter().find(|fact| fact.get("property").and_then(Value::as_str) == Some(name)))
        .and_then(|fact| fact.get("value"))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
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
    if read.get("isSimpleItem").and_then(Value::as_bool) == Some(false) {
        return crate::missionkinds::lookup(&text(read, "commandName")).map(|kind| kind.id).unwrap_or("complex");
    }
    if !flag(read, "isSimpleItem") {
        return "unreadable";
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
        let corridor = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "commandName": "Corridor Scan" })], 1), &[]);
        assert_eq!(corridor["items"][1]["kind"], "corridor", "a complex item the catalogue does know is named, because its geometry and its shape are looked up by that name");
        assert_eq!(view["items"][1]["name"], "Fixed Wing Landing");
    }
}

#[cfg(test)]
mod reported {
    use super::*;

    struct One(Value);

    impl Backend for One {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": 2 }).to_string(),
                "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": 1 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "plan.missionController" => json!({ "kind": "object", "containsItems": true }).to_string(),
                "plan.missionController.visualItems.0" => json!({ "kind": "object", "homePosition": true, "sequenceNumber": 0, "isSimpleItem": false }).to_string(),
                "plan.missionController.visualItems.1" => self.0.to_string(),
                _ => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn listed(item: Value) -> Value {
        items_view(&One(item), &[])["items"][1].clone()
    }

    #[test]
    fn waiting_for_terrain_is_not_the_operators_task() {
        let waiting = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true,
            "readyForSaveState": AWAITING_TERRAIN, "readyForSaveMessage": "Waiting for terrain data",
        }));
        assert_eq!(waiting["blocked"], false, "there is nothing for an operator to fix while a terrain server is being waited on, and a banner naming this item invites a click that cannot help");
        assert_eq!(waiting["awaitingTerrain"], true);
        assert_eq!(waiting["blockedReason"], "Waiting for terrain data", "it still says what it is waiting for");

        let missing = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true,
            "readyForSaveState": 2, "readyForSaveMessage": "The survey needs an area",
        }));
        assert_eq!(missing["blocked"], true, "an item missing something the operator has to supply is a task");
        assert_eq!(missing["awaitingTerrain"], false);
    }

    #[test]
    fn a_takeoff_that_has_not_been_placed_carries_no_place() {
        let unplaced = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "isTakeoffItem": true, "specifiesCoordinate": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 0.0, "longitude": 0.0, "altitude": 0.0 },
        }));
        assert_eq!(unplaced["coordinate"], Value::Null, "zero, zero is a valid coordinate by Qt's rules and is where an unplaced takeoff sits, so a head trusting valid draws it in the Atlantic");

        let placed = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0, "altitude": 50.0 },
        }));
        assert_eq!(placed["coordinate"]["latitude"], 47.0);

        let never = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": false, "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 } }));
        assert_eq!(never["coordinate"], Value::Null, "an item that does not place itself on the map has no place, whatever its coordinate happens to hold");
    }

    #[test]
    fn the_height_the_operator_set_travels_beside_the_height_above_the_sea() {
        let item = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true,
            "amslEntryAlt": 660.0,
            "altitudeMode": 1,
            "facts": [ { "name": "Altitude", "property": "altitude", "value": 75.0 } ],
        }));
        assert_eq!(item["altitude"], 75.0, "a fact is found by its property, because name is the label an operator reads and has a capital in it");
        assert_eq!(item["altitudeAmsl"], 660.0, "and the one it flies at, which differ by the launch elevation");
        assert_eq!(item["altitudeMode"], 1, "altitudeMode is a plain property on the item and not a fact, so searching the facts for it finds nothing");
    }



    #[test]
    fn a_pattern_the_vehicle_leaves_by_another_corner_says_where_it_leaves() {
        let survey = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "isSurveyItem": true,
            "specifiesCoordinate": true, "isIncomplete": false, "exitCoordinateSameAsEntry": false,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
            "exitCoordinate": { "kind": "coordinate", "valid": true, "latitude": 47.1, "longitude": 8.1 },
        }));
        assert_eq!(survey["exitCoordinate"]["latitude"], 47.1, "a route drawn from where the vehicle went in doubles back across the pattern");
        assert_eq!(survey["coordinate"]["latitude"], 47.0);

        let waypoint = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "exitCoordinateSameAsEntry": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
            "exitCoordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
        }));
        assert_eq!(waypoint["exitCoordinate"], Value::Null, "a waypoint is left where it was entered, and drawing a second point there is a point on top of a point");
    }

    #[test]
    fn a_complex_item_with_no_shape_yet_is_not_a_leg_the_vehicle_flies() {
        let drawn = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "isSurveyItem": true,
            "specifiesCoordinate": true, "isIncomplete": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
        }));
        assert_eq!(drawn["incomplete"], true);
        assert_eq!(drawn["flownLeg"], false, "a survey still being drawn has a centre but no route through it, and drawing a leg to it puts a line across the map");

        let finished = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "isSurveyItem": true,
            "specifiesCoordinate": true, "isIncomplete": false,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
        }));
        assert_eq!(finished["flownLeg"], true);

        let standalone = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "isStandaloneCoordinate": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
        }));
        assert_eq!(standalone["flownLeg"], false, "a region of interest has a place and the vehicle does not fly to it");
    }

    #[test]
    fn an_item_that_did_not_read_is_not_quietly_a_complex_one() {
        let unread = listed(json!({ "kind": "null" }));
        assert_eq!(unread["kind"], "unreadable", "a read that found nothing used to classify as complex, because complex was the only negative test");
    }
}
