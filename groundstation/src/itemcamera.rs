use serde_json::{Value, json};

use crate::read::{enum_choice, enum_labels, flag, object, shown_text};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "plan.missionController.currentPlanViewVIIndex",
    "plan.controllerVehicle.apmFirmware",
    "plan.missionController@visualItemsReset",
];

const NO_CAMERA_ACTION: i64 = 0;
const TAKE_PHOTOS_TIME: i64 = 1;
pub const MISSION_START_NOTE: &str = "Camera commands above take effect immediately at mission start.";

pub fn mission_start_camera_shown(index: usize, apm_firmware: bool, advanced: bool) -> bool {
    index != 0 || (!apm_firmware && advanced)
}
const TAKE_PHOTOS_DISTANCE: i64 = 2;

fn fact<'a>(section: &'a Value, name: &str) -> Option<&'a Value> {
    section
        .get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| facts.iter().find(|f| f.get("property").and_then(Value::as_str) == Some(name)))
}

fn measure(section: &Value, name: &str) -> Value {
    match fact(section, name) {
        Some(found) => {
            let labels = enum_labels(found);
            json!({
                "value": found.get("value").cloned().unwrap_or(Value::Null),
                "text": shown_text(found),
                "units": found.get("units").cloned().unwrap_or(Value::Null),
                "choices": (!labels.is_empty()).then_some(labels),
                "choice": enum_choice(found),
                "slider": crate::read::user_slider(found),
            })
        }
        None => Value::Null,
    }
}

pub fn item_camera_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(index) = args.first().and_then(|a| a.trim().parse::<usize>().ok()) else {
        return json!({ "kind": "object", "class": "ItemCamera", "available": false, "reason": "A mission item index is required." });
    };
    let section = crate::coreplan::camera_section(index).unwrap_or_else(|| object(&backend.get(&format!("plan.missionController.visualItems.{index}.cameraSection"))));
    let apm = flag(&object(&backend.get_fields("plan.controllerVehicle", "apmFirmware")), "apmFirmware");
    let present = section.get("kind").and_then(Value::as_str) == Some("object") && mission_start_camera_shown(index, apm, crate::advancedui::shown());
    let specified = flag(&section, "specifyGimbal");
    let action = fact(&section, "cameraAction").and_then(|f| f.get("value")).and_then(Value::as_i64);
    json!({
        "kind": "object",
        "class": "ItemCamera",
        "index": index,
        "available": present,
        "reason": match present {
            true => Value::Null,
            false => json!("This mission item has no camera section."),
        },
        "commandsGimbal": present && specified,
        "gimbalPitch": if specified { measure(&section, "gimbalPitch") } else { Value::Null },
        "gimbalYaw": if specified { measure(&section, "gimbalYaw") } else { Value::Null },
        "cameraAction": measure(&section, "cameraAction"),
        "intervalTime": match action { Some(TAKE_PHOTOS_TIME) => measure(&section, "cameraPhotoIntervalTime"), _ => Value::Null },
        "intervalDistance": match action { Some(TAKE_PHOTOS_DISTANCE) => measure(&section, "cameraPhotoIntervalDistance"), _ => Value::Null },
        "cameraModeSupported": present && flag(&section, "cameraModeSupported"),
        "commandsMode": present && flag(&section, "specifyCameraMode"),
        "cameraMode": if flag(&section, "cameraModeSupported") { measure(&section, "cameraMode") } else { Value::Null },
        "path": format!("plan.missionController.visualItems.{index}.cameraSection"),
        "note": (present && index == 0 && (specified || flag(&section, "specifyCameraMode") || action.is_some_and(|a| a != NO_CAMERA_ACTION))).then_some(MISSION_START_NOTE),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mission_start_camera_section_is_not_offered_for_ardupilot_like_mission_settings_editor() {
        assert!(!mission_start_camera_shown(0, true, true));
        assert!(mission_start_camera_shown(0, false, true));
        assert!(!mission_start_camera_shown(0, false, false), "Advanced Mode off hides it unless missionWaypointsOnly, which is false");
        assert!(mission_start_camera_shown(3, true, false), "a waypoint's own camera section stays");
    }

    struct Item(Option<bool>);
    impl Backend for Item {
        fn get(&self, path: &str) -> String {
            match (path.ends_with(".cameraSection"), self.0) {
                (true, Some(specify)) => json!({ "kind": "object", "specifyGimbal": specify, "facts": [
                    { "property": "gimbalPitch", "value": -90.0, "valueString": "-90", "enumOrValueString": "-90", "units": "deg", "userMin": -90.0, "userMax": 0.0, "decimalPlaces": 0 },
                    { "property": "gimbalYaw", "value": 45.0, "valueString": "45", "enumOrValueString": "45", "units": "deg" },
                    { "property": "cameraAction", "value": 6, "valueString": "6", "enumOrValueString": "Take photo",
                      "enumStrings": ["No change", "Take photo", "Take photos (time)", "Take photos (distance)", "Stop taking photos", "Start recording video", "Stop recording video"],
                      "enumValues": [0, 6, 1, 2, 3, 4, 5], "enumIndex": 1, "units": "" },
                ] })
                .to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            self.get(p)
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    struct Timed;
    impl Backend for Timed {
        fn get(&self, _p: &str) -> String {
            let section = crate::plandoc::camera_section(&[crate::plandoc::Simple { command: 2000, frame: 2, params: [Some(0.0), Some(4.0), Some(0.0), None, None, None, None], auto_continue: true, altitude: None, sections: Vec::new() }]);
            match section {
                Value::Object(map) => Value::Object(map.into_iter().chain([("cameraModeSupported".to_string(), json!(true))]).collect()),
                other => other,
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_core_section_angles_carry_camera_section_fact_metadata_user_ranges() {
        let section = crate::plandoc::camera_section(&[]);
        assert_eq!(crate::read::user_slider(fact(&section, "gimbalPitch").unwrap()), json!({ "from": -90.0, "to": 0.0, "decimals": 0 }));
        assert_eq!(crate::read::user_slider(fact(&section, "gimbalYaw").unwrap()), json!({ "from": -180.0, "to": 180.0, "decimals": 0 }));
    }

    #[test]
    fn a_timed_photo_action_carries_its_interval_and_the_mode_choice() {
        let view = item_camera_view(&Timed, &["2".into()]);
        assert_eq!(view["intervalTime"]["value"], 4.0, "CameraSection shows Time only for Take photos (time)");
        assert_eq!(view["intervalDistance"], Value::Null);
        assert_eq!(view["cameraModeSupported"], true);
        assert_eq!(view["commandsMode"], false);
        assert_eq!(view["cameraMode"]["choices"], json!(["Photo", "Video", "Survey"]));
        assert_eq!(view["path"], "plan.missionController.visualItems.2.cameraSection");
    }

    #[test]
    fn an_angle_only_travels_when_the_item_actually_commands_it() {
        let commanding = item_camera_view(&Item(Some(true)), &["3".into()]);
        assert_eq!(commanding["commandsGimbal"], true);
        assert_eq!(commanding["gimbalPitch"]["value"], -90.0);
        assert_eq!(commanding["gimbalPitch"]["text"], "-90");
        assert_eq!(commanding["gimbalPitch"]["slider"], json!({ "from": -90.0, "to": 0.0, "decimals": 0 }), "CameraSection draws Pitch as a FactTextFieldSlider over the fact's userMin..userMax");
        assert_eq!(commanding["gimbalYaw"]["slider"], Value::Null, "a fact without a user range gets no slider");

        let untouched = item_camera_view(&Item(Some(false)), &["3".into()]);
        assert_eq!(untouched["commandsGimbal"], false);
        assert_eq!(untouched["gimbalPitch"], Value::Null, "the fact carries -90 whether or not the item commands the gimbal, so serving it unguarded tells a head this waypoint points the camera down when it leaves it alone");
        assert_eq!(untouched["available"], true, "the section is there; it simply does not specify a gimbal, which is not the same as having no section");
        assert_eq!(untouched["cameraAction"]["text"], "Take photo", "the camera action is not gated by the gimbal flag and still travels");

        let absent = item_camera_view(&Item(None), &["3".into()]);
        assert_eq!(absent["available"], false);
        assert_eq!(absent["commandsGimbal"], false);

        assert_eq!(commanding["cameraAction"]["text"], "Take photo", "CameraAction declares enumValues 0,6,1,2,3,4,5 so the raw value for Take photo is 6, and valueString is that 6 - a head showing it names no action and a head indexing the label list by it names Stop recording video");
        assert_eq!(commanding["cameraAction"]["choice"], 1);
        assert_eq!(commanding["cameraAction"]["choices"][1], "Take photo");
        assert_eq!(commanding["gimbalPitch"]["choices"], Value::Null, "an angle has no choices, and an empty list reads to a picker as a choice with nothing in it rather than as not a choice at all");
        assert_eq!(commanding["gimbalPitch"]["choice"], Value::Null);

        let unasked = item_camera_view(&Item(Some(true)), &[]);
        assert_eq!(unasked["available"], false, "no index names no item, which is not an item without a camera");
        assert!(unasked["reason"].as_str().unwrap().contains("index"));
    }
}
