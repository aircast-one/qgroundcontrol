use serde_json::{Value, json};

use crate::read::{Unit, flag, format_measure, integer, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "plan.missionController.visualItems.count",
    "plan.missionController.currentPlanViewVIIndex",
    "plan.missionController.containsItems",
    "plan.missionController@visualItemsChanged",
    "plan.missionController@newItemsFromVehicle",
    "settings.unitsSettings.horizontalDistanceUnits",
    "settings.unitsSettings.verticalDistanceUnits",
];

const FIELDS: &str = "specifiedFlightSpeed,additionalTimeDelay,minAMSLAltitude,maxAMSLAltitude,sequenceNumber,abbreviation,commandName,commandDescription,isCurrentItem,specifiesCoordinate,isStandaloneCoordinate,specifiesAltitudeOnly,isSimpleItem,isTakeoffItem,isLandCommand,isSurveyItem,homePosition,coordinate,amslEntryAlt,altDifference,azimuth,distance,distanceFromStart,readyForSaveState,readyForSaveMessage,dirty,altitude,altitudeMode,isIncomplete,exitCoordinate,exitCoordinateSameAsEntry,commandName,command,category,specifiesAltitude,cameraShots,complexDistance,plannedHomePositionAltitude";

const READY_TO_SAVE: i64 = 0;
const AWAITING_TERRAIN: i64 = 1;
const RETURN_TO_LAUNCH: i64 = 20;

pub fn items_view(backend: &dyn Backend, args: &[String]) -> Value {
    let everything = args.iter().any(|arg| arg == "fields");
    let vertical = Unit::vertical(backend);
    let imperial = crate::missionsummary::imperial(backend);
    let shapes = args.iter().any(|arg| arg == "geometry");
    let count = integer(&object(&backend.get("plan.missionController.visualItems.count")), "value").unwrap_or(0);
    let has_items = flag(&object(&backend.get_fields("plan.missionController", "containsItems")), "containsItems");
    if count <= 0 {
        return json!({ "kind": "object", "class": "MissionItems", "available": false, "items": [], "selected": -1, "reason": "This plan has no items yet." });
    }
    let current = integer(&object(&backend.get("plan.missionController.currentPlanViewVIIndex")), "value").unwrap_or(-1);
    let listed = object(&backend.get_fields("plan.missionController.visualItems", FIELDS));
    let items: Vec<Value> = match listed.get("elements").and_then(Value::as_array) {
        Some(elements) => elements.iter().enumerate().map(|(index, element)| item(element, index as i64, &vertical, imperial)).collect(),
        None => (0..count)
            .map(|index| item(&object(&backend.get_fields(&format!("plan.missionController.visualItems.{index}"), FIELDS)), index, &vertical, imperial))
            .collect(),
    };
    let items: Vec<Value> = match walked(&items) {
        true => items,
        false => items.into_iter().map(unwalked).collect(),
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
        "selected": current,
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

fn item(read: &Value, index: i64, vertical: &Unit, imperial: bool) -> Value {
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
        "selected": flag(read, "isCurrentItem"),
        "coordinate": coordinate,
        "exitCoordinate": exit,
        "altitude": height(read),
        "altitudeText": height(read).map(|metres| format_measure(vertical.show(metres), &vertical.name)),
        "altitudeUnits": height(read).map(|_| vertical.name.clone()),
        "altitudeFactUnits": fact_units(read, "altitude").or_else(|| fact_units(read, "plannedHomePositionAltitude")),
        "specifiesAltitude": flag(read, "specifiesAltitude"),
        "altitudeOnly": flag(read, "specifiesAltitudeOnly"),
        "category": Some(text(read, "category")).filter(|category| !category.is_empty()),
        "cameraShots": number(read, "cameraShots").map(|shots| shots as i64).filter(|shots| *shots > 0),
        "patternDistance": number(read, "complexDistance").filter(|metres| *metres > 0.0),
        "simple": read.get("isSimpleItem").and_then(Value::as_bool),
        "altitudeMode": number(read, "altitudeMode").map(|mode| mode as i64),
        "altitudeAmsl": number(read, "amslEntryAlt"),
        "extraSeconds": number(read, "additionalTimeDelay"),
        "speedChange": number(read, "specifiedFlightSpeed"),
        "altitudeAmslLowest": number(read, "minAMSLAltitude"),
        "altitudeAmslHighest": number(read, "maxAMSLAltitude"),
        "specifiesCoordinate": flag(read, "specifiesCoordinate"),
        "altitudeChange": number(read, "altDifference"),
        "altitudeChangeText": number(read, "altDifference").map(|change| crate::read::altitude_text(change, vertical, true)),
        "azimuth": number(read, "azimuth"),
        "azimuthText": number(read, "azimuth").map(|bearing| format!("{}\u{b0}", (bearing.round() as i64).rem_euclid(360))),
        "distance": number(read, "distance"),
        "distanceText": number(read, "distance").map(|metres| crate::missionsummary::distance_text(metres, imperial)),
        "distanceFromStart": number(read, "distanceFromStart"),
        "edited": flag(read, "dirty"),
        "incomplete": flag(read, "isIncomplete"),
        "endsRoute": flag(read, "isLandCommand") || integer(read, "command") == Some(RETURN_TO_LAUNCH),
        "command": integer(read, "command"),
        "flownLeg": flag(read, "specifiesCoordinate") && !flag(read, "isStandaloneCoordinate") && !flag(read, "isIncomplete"),
        "movable": coordinate.is_some() && kind(read) != "settings",
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

fn fact_units(read: &Value, name: &str) -> Option<String> {
    read.get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| facts.iter().find(|fact| fact.get("property").and_then(Value::as_str) == Some(name)))
        .and_then(|fact| fact.get("units"))
        .and_then(Value::as_str)
        .filter(|units| !units.is_empty())
        .map(str::to_string)
}

const LEG_FIGURES: [&str; 6] = ["distance", "distanceText", "distanceFromStart", "azimuth", "azimuthText", "altitudeChangeText"];

fn walked(items: &[Value]) -> bool {
    let legs = items.iter().filter(|item| flag(item, "flownLeg")).count();
    let reached = |item: &Value| item.get("distanceFromStart").and_then(Value::as_f64).is_some_and(|metres| metres > 0.0);
    legs < 2 || items.iter().any(reached)
}

fn unwalked(item: Value) -> Value {
    match item {
        Value::Object(mut fields) => {
            LEG_FIGURES.iter().for_each(|key| {
                fields.insert((*key).to_string(), Value::Null);
            });
            Value::Object(fields)
        }
        other => other,
    }
}

fn height(read: &Value) -> Option<f64> {
    if flag(read, "isSimpleItem") && !flag(read, "specifiesAltitude") {
        return None;
    }
    match flag(read, "homePosition") {
        true => fact_number(read, "altitude").or_else(|| fact_number(read, "plannedHomePositionAltitude")),
        false => fact_number(read, "altitude"),
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
    let transects: Vec<Value> = object(&backend.get(&format!("plan.missionController.visualItems.{index}.visualTransectPoints")))
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
        false => json!({ "shape": shape.0, "property": shape.1, "vertices": listed, "transects": transects }),
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
        return crate::missionkinds::by_class(&text(read, "class")).map(|kind| kind.id).unwrap_or("complex");
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
        assert_eq!(view["selected"], 1);
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
        assert_eq!(none["selected"], -1);
    }

    const ALWAYS_PRESENT: [&str; 4] = ["kind", "class", "facts", "children"];

    #[test]
    fn every_field_this_view_reads_is_one_it_asked_for() {
        let body = include_str!("missionitems.rs").split("#[cfg(test)]").next().unwrap().to_string();
        let requested: Vec<&str> = FIELDS.split(',').map(str::trim).collect();
        let read: Vec<String> = ["flag(read, \"", "number(read, \"", "integer(read, \"", "text(read, \"", "read.get(\"", "at_key(read, \""]
            .iter()
            .flat_map(|opener| {
                body.match_indices(opener).filter_map(|(at, _)| {
                    let tail = &body[at + opener.len()..];
                    tail.find('"').map(|end| tail[..end].to_string())
                })
            })
            .collect();
        assert!(read.len() > 20, "the scan found {} reads, too few to be this view", read.len());

        let unasked: Vec<&String> = read
            .iter()
            .filter(|key| !requested.contains(&key.as_str()) && !ALWAYS_PRESENT.contains(&key.as_str()))
            .collect();
        assert!(unasked.is_empty(), "these are read from an item and never requested, so they always come back missing: {unasked:?}");
    }

    #[test]
    fn an_item_with_no_position_is_not_one_a_head_can_drag() {
        let unplaced = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "coordinate": at(0.0, 0.0) })], 1), &[]);
        assert_eq!(unplaced["items"][1]["coordinate"], Value::Null, "nought by nought is the unset coordinate, not a place off Africa");
        assert_eq!(unplaced["items"][1]["movable"], false);

        let placed = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "coordinate": at(47.397, 8.546) })], 1), &[]);
        assert_eq!(placed["items"][1]["movable"], true);

        assert_eq!(unplaced["items"][0]["kind"], "settings");
        assert_eq!(unplaced["items"][0]["movable"], false, "the plan's own entry is not on the map, whatever coordinate it reports");
    }

    #[test]
    fn a_complex_item_the_catalogue_does_not_name_is_still_listed_as_complex() {
        let pattern = json!({ "kind": "object", "sequenceNumber": 5, "abbreviation": "FWL", "commandName": "Fixed Wing Landing", "isSimpleItem": false });
        let view = items_view(&Plan(vec![settings(), pattern], 1), &[]);
        assert_eq!(view["items"][1]["kind"], "complex", "an item type the core has no entry for still has to draw as something rather than as a waypoint");
        let corridor = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "class": "CorridorScanComplexItem", "commandName": "\u{56de}\u{5eca}\u{30b9}\u{30ad}\u{30e3}\u{30f3}" })], 1), &[]);
        assert_eq!(corridor["items"][1]["kind"], "corridor", "a complex item is identified by what it is, not by what the interface happens to call it here");

        let structure = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "class": "StructureScanComplexItem", "commandName": "\u{69cb}\u{9020}\u{30b9}\u{30ad}\u{30e3}\u{30f3}" })], 1), &[]);
        assert_eq!(structure["items"][1]["kind"], "structure");
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
    fn an_altitude_is_drawn_in_the_unit_the_operator_chose_by_the_core_and_not_by_a_head() {
        struct Imperial(Value);
        impl Backend for Imperial {
            fn get(&self, path: &str) -> String { One(self.0.clone()).get(path) }
            fn get_fields(&self, path: &str, fields: &str) -> String {
                match path {
                    "units" => json!({ "kind": "object", "appSettingsVerticalDistanceUnitsString": "ft" }).to_string(),
                    _ => One(self.0.clone()).get_fields(path, fields),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                match path {
                    "units.metersToAppSettingsVerticalDistanceUnits" => json!({ "ok": true, "result": serde_json::from_str::<Vec<f64>>(args).unwrap()[0] * 3.2808399 }).to_string(),
                    _ => String::new(),
                }
            }
            fn watch(&self, _p: &[String]) {}
        }
        let item = json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true, "category": "Basic", "facts": [ { "name": "Altitude", "property": "altitude", "value": 75.0 } ] });
        let metric = listed(item.clone());
        assert_eq!(metric["altitudeText"], "75.0 m");
        assert_eq!(metric["altitudeUnits"], "m", "an editor needs the number and the unit apart, and parsing them back out of the text is the shape this replaces");
        assert_eq!(metric["altitude"], 75.0, "the raw metres travel too, so a head drawing a bar is not parsing its own string back");
        assert_eq!(metric["specifiesAltitude"], true, "an item that specifies an altitude of zero would be indistinguishable from one that specifies none, if this were inferred from the value");
        assert_eq!(metric["category"], "Basic");
        assert_eq!(metric["simple"], true, "whether a command can be changed follows from this, and inferring it from the presence of a command number is inferring a fact from the absence of another");

        let imperial = items_view(&Imperial(item), &[])["items"][1].clone();
        assert_eq!(imperial["altitudeText"], "246 ft", "the feet come from the app's own conversion, not a factor the core keeps its own copy of");
        assert_eq!(imperial["altitudeUnits"], "ft");
        assert_eq!(imperial["altitude"], 75.0);
    }

    #[test]
    fn only_a_command_that_states_an_altitude_shows_one() {
        let altitude = |value: f64| json!({ "property": "altitude", "value": value });
        let speed = listed(json!({
            "kind": "object", "sequenceNumber": 2, "isSimpleItem": true, "specifiesAltitude": false,
            "specifiesCoordinate": false, "facts": [ altitude(0.0) ],
        }));
        assert_eq!(speed["altitudeText"], Value::Null, "a plan loaded from a file sets the altitude fact from param7 without asking whether the command specifies one, so a DO_ item carries a real zero rather than a NaN and no fallback is involved");
        assert_eq!(speed["altitude"], Value::Null);

        let waypoint = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true,
            "specifiesCoordinate": true, "facts": [ altitude(0.0) ],
        }));
        assert_eq!(waypoint["altitudeText"], "0.0 m", "a waypoint at zero states an altitude of zero, which is a different thing from stating none");

        let home = listed(json!({
            "kind": "object", "sequenceNumber": 0, "homePosition": true, "isSimpleItem": false,
            "facts": [ json!({ "property": "plannedHomePositionAltitude", "value": 12.0 }) ],
        }));
        assert_eq!(home["altitudeText"], "12.0 m", "the launch row is not a SimpleMissionItem, so it carries no specifiesAltitude property at all - gating on the bare flag would have blanked it");

        let survey = listed(json!({
            "kind": "object", "sequenceNumber": 3, "isSimpleItem": false, "specifiesCoordinate": true,
            "facts": [ altitude(80.0) ],
        }));
        assert_eq!(survey["altitudeText"], "80.0 m", "and a complex item carries none either, for the same reason");
    }

    #[test]
    fn leg_figures_are_absent_until_the_controller_has_walked_the_plan() {
        struct Plan(Vec<Value>);
        impl Backend for Plan {
            fn get(&self, path: &str) -> String {
                match path {
                    "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": self.0.len() }).to_string(),
                    "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": 1 }).to_string(),
                    _ => String::new(),
                }
            }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "plan.missionController" => json!({ "kind": "object", "containsItems": true }).to_string(),
                    "plan.missionController.visualItems" => json!({ "kind": "object", "elements": self.0 }).to_string(),
                    _ => String::new(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let place = |sequence: i64| json!({ "kind": "object", "sequenceNumber": sequence, "isSimpleItem": true, "specifiesCoordinate": true, "distance": 0.0, "distanceFromStart": 0.0, "azimuth": 0.0, "altDifference": 0.0 });
        let fresh = items_view(&Plan(vec![place(0), place(1), place(2)]), &[]);
        let row = &fresh["items"][1];
        assert_eq!(row["distance"], Value::Null, "the controller fills these in a walk that runs after an insert returns, and until it does every one of them reads zero - which is a real possible distance, so a head cannot tell the difference and caches it");
        assert_eq!(row["distanceText"], Value::Null);
        assert_eq!(row["distanceFromStart"], Value::Null);
        assert_eq!(row["azimuthText"], Value::Null, "a bearing of zero is due north, which is the same trap one field over");
        assert_eq!(row["altitudeChangeText"], Value::Null);
        assert_eq!(row["kind"], "waypoint", "everything the walk does not produce still travels; this withholds four figures, not the item");

        let walked_plan = vec![place(0), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "distance": 120.0, "distanceFromStart": 120.0 }), place(2)];
        let done = items_view(&Plan(walked_plan), &[]);
        assert_eq!(done["items"][1]["distanceText"], "120 m", "one item reporting a distance from the start is the tell that the walk has run, and then every figure it produced is trusted - including the zeros, which are real once something moved");
        assert_eq!(done["items"][2]["distance"], 0.0);
    }

    #[test]
    fn a_command_with_no_altitude_does_not_inherit_the_launch_altitude() {
        let home_altitude = json!({ "property": "plannedHomePositionAltitude", "value": 0.0 });
        let speed_change = listed(json!({
            "kind": "object", "sequenceNumber": 2, "isSimpleItem": true, "specifiesCoordinate": false,
            "facts": [ { "property": "altitude", "value": null }, home_altitude.clone() ],
        }));
        assert_eq!(speed_change["altitudeText"], Value::Null, "QGC sets the altitude fact to NaN when an item specifies no altitude, so falling back to the launch altitude gave every DO_ command the home height as its own");
        assert_eq!(speed_change["altitude"], Value::Null);

        let start = items_view(&One(json!({ "kind": "object", "sequenceNumber": 0, "homePosition": true, "facts": [ home_altitude ] })), &[])["items"][1].clone();
        assert_eq!(start["altitudeText"], "0.0 m", "the fallback is right for the row whose altitude is the launch altitude, which is the row it was written for");
    }

    #[test]
    fn a_leg_is_spelled_by_the_same_code_that_spells_the_strip_above_it() {
        struct Units(Value, f64);
        impl Backend for Units {
            fn get(&self, path: &str) -> String {
                match path {
                    "settings.unitsSettings.horizontalDistanceUnits.rawValue" => json!({ "kind": "value", "value": self.1 }).to_string(),
                    _ => One(self.0.clone()).get(path),
                }
            }
            fn get_fields(&self, path: &str, fields: &str) -> String {
                match (path, self.1) {
                    ("units", 0.0) => json!({ "kind": "object", "appSettingsVerticalDistanceUnitsString": "ft" }).to_string(),
                    _ => One(self.0.clone()).get_fields(path, fields),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                match (path, self.1) {
                    ("units.metersToAppSettingsVerticalDistanceUnits", 0.0) => json!({ "ok": true, "result": serde_json::from_str::<Vec<f64>>(args).unwrap()[0] * 3.2808399 }).to_string(),
                    _ => String::new(),
                }
            }
            fn watch(&self, _p: &[String]) {}
        }
        let leg = |metres: f64| json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "distance": metres, "distanceFromStart": metres, "azimuth": 47.4, "altDifference": -12.0 });
        let row = |item: Value, raw: f64| items_view(&Units(item, raw), &[])["items"][1].clone();

        let short = row(leg(449.36), 1.0);
        assert_eq!(short["distanceText"], "449 m");
        assert_eq!(short["distanceText"], crate::missionsummary::distance_text(449.36, false), "the strip and the row are one function, so they cannot disagree about a number they both draw");
        assert_eq!(short["azimuthText"], "47\u{b0}");
        assert_eq!(row(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "azimuth": 359.7 }), 1.0)["azimuthText"], "0\u{b0}", "QGC rounds before it wraps, so a bearing a third of a degree short of north reads as north and never as 360");
        assert_eq!(short["altitudeChangeText"], "-12.0 m", "a descent reads as a descent; the bare magnitude leaves the operator to work out the direction from the two altitudes either side");
        assert_eq!(short["altitudeChange"], -12.0, "the signed number still travels, so a head drawing an arrow is not parsing its own string back");

        let far = row(leg(1500.0), 1.0);
        assert_eq!(far["distanceText"], "1.50 km", "the kilometre threshold is the part a head reimplementing this would get wrong");

        let feet = row(leg(449.36), 0.0);
        assert_eq!(feet["distanceText"], "1474 ft");
        assert_eq!(feet["altitudeChangeText"], "-39.4 ft", "an altitude is drawn in the vertical unit, which QGC keeps apart from the horizontal one, and by the app's own conversion rather than a factor the core copies");

        let climb = row(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "altDifference": 12.0 }), 1.0);
        assert_eq!(climb["altitudeChangeText"], "+12.0 m");
        let unknown = row(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true }), 1.0);
        assert_eq!(unknown["azimuthText"], Value::Null, "a bearing the controller has not worked out is absent, not a plausible one");
        assert_eq!(unknown["altitudeChangeText"], Value::Null);

        let command = row(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": false }), 1.0);
        assert_eq!(command["specifiesCoordinate"], false, "a change-speed or camera-trigger command has no place by design, and a list that says \"no position\" about it reports the nature of the command as a defect in the plan");
        assert_eq!(command["altitudeOnly"], false, "which is not the same as a takeoff, whose place is withheld by the display and not by the command");
        assert_eq!(short["specifiesCoordinate"], true);
        assert_eq!(climb["distanceText"], Value::Null, "a leg with no distance has no text, rather than a plausible zero");
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
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "specifiesAltitude": true,
            "amslEntryAlt": 660.0,
            "altitudeMode": 1,
            "facts": [ { "name": "Altitude", "property": "altitude", "value": 75.0 } ],
        }));
        assert_eq!(item["altitude"], 75.0, "a fact is found by its property, because name is the label an operator reads and has a capital in it");
        assert_eq!(item["altitudeAmsl"], 660.0, "and the one it flies at, which differ by the launch elevation");
        assert_eq!(item["altitudeMode"], 1, "altitudeMode is a plain property on the item and not a fact, so searching the facts for it finds nothing");
    }

    #[test]
    fn the_plans_own_entry_reports_the_height_it_keeps_under_its_own_name() {
        let settings = items_view(&One(json!({ "kind": "object", "sequenceNumber": 1 })), &[])["items"][0].clone();
        assert_eq!(settings["kind"], "settings");

        let launch = listed(json!({
            "kind": "object", "sequenceNumber": 0, "homePosition": true, "isSimpleItem": false,
            "facts": [ { "name": "Altitude", "property": "plannedHomePositionAltitude", "value": 585.0 } ],
        }));
        assert_eq!(launch["altitude"], 585.0, "the launch elevation is a height and the row that shows it goes blank if only the waypoint name is looked for");
        assert_eq!(launch["altitudeText"], "585 m", "every measure the core serves rounds the same way, and a row that kept a tenth here read 585.0 m beside a summary saying 585 m to 660 m");

        let labelled = listed(json!({
            "kind": "object", "sequenceNumber": 0, "homePosition": true, "isSimpleItem": false,
            "facts": [ { "name": "Altitude", "property": "plannedHomePositionAltitude", "value": 585.0, "units": "m" } ],
        }));
        assert_eq!(labelled["altitudeFactUnits"], "m");

        let waypoint = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true,
            "facts": [ { "name": "Altitude", "property": "altitude", "value": 50.0, "units": "ft" } ],
        }));
        assert_eq!(waypoint["altitudeFactUnits"], "ft", "a waypoint's own fact still wins, so the fallback is a fallback rather than an override");
    }

    #[test]
    fn a_route_stops_at_a_command_number_rather_than_at_a_word() {
        let rtl = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "command": RETURN_TO_LAUNCH, "commandName": "Zur\u{00fc}ck zum Start" }));
        assert_eq!(rtl["endsRoute"], true, "a translated command name is a different string in every locale, and a route drawn past a return to launch is a line across the map");
        assert_eq!(rtl["command"], RETURN_TO_LAUNCH);

        let land = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "isLandCommand": true, "command": 21 }));
        assert_eq!(land["endsRoute"], true);

        let waypoint = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "command": 16, "commandName": "Waypoint" }));
        assert_eq!(waypoint["endsRoute"], false);
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
