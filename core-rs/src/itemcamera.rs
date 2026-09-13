use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

// currentPlanFile was invented and the bridge has no such property; two guards said so. These are
// the paths missionitems already watches. The item's own camera facts are behind an indexed path
// and cannot be watched at all, which is the standing limit on every view keyed by an argument -
// this recomputes when the selection moves or the items change, and rides the poll otherwise.
pub const DEPS: &[&str] = &[
    "plan.missionController.currentPlanViewVIIndex",
    "plan.missionController@visualItemsChanged",
];

fn fact<'a>(section: &'a Value, name: &str) -> Option<&'a Value> {
    section
        .get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| facts.iter().find(|f| f.get("property").and_then(Value::as_str) == Some(name)))
}

fn measure(section: &Value, name: &str) -> Value {
    match fact(section, name) {
        Some(found) => json!({
            "value": found.get("value").cloned().unwrap_or(Value::Null),
            "text": found.get("valueString").cloned().unwrap_or(Value::Null),
            "units": found.get("units").cloned().unwrap_or(Value::Null),
        }),
        None => Value::Null,
    }
}

pub fn item_camera_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(index) = args.first().and_then(|a| a.trim().parse::<usize>().ok()) else {
        return json!({ "kind": "object", "class": "ItemCamera", "available": false, "reason": "A mission item index is required." });
    };
    let section = object(&backend.get(&format!("plan.missionController.visualItems.{index}.cameraSection")));
    let present = section.get("kind").and_then(Value::as_str) == Some("object");
    // The angles are Facts and always carry a number, so a head reading them alone is told the
    // gimbal points somewhere for an item that never touches it. specifyGimbal is a plain bool
    // rather than a Fact, so it does not arrive with them through view.control - which is how a
    // head ends up with the value and not the thing that says whether it means anything.
    let specified = flag(&section, "specifyGimbal");
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item(Option<bool>);
    impl Backend for Item {
        fn get(&self, path: &str) -> String {
            match (path.ends_with(".cameraSection"), self.0) {
                (true, Some(specify)) => json!({ "kind": "object", "specifyGimbal": specify, "facts": [
                    { "property": "gimbalPitch", "value": -90.0, "valueString": "-90", "units": "deg" },
                    { "property": "gimbalYaw", "value": 45.0, "valueString": "45", "units": "deg" },
                    { "property": "cameraAction", "value": 1.0, "valueString": "Take photo", "units": "" },
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

    #[test]
    fn an_angle_only_travels_when_the_item_actually_commands_it() {
        let commanding = item_camera_view(&Item(Some(true)), &["3".into()]);
        assert_eq!(commanding["commandsGimbal"], true);
        assert_eq!(commanding["gimbalPitch"]["value"], -90.0);
        assert_eq!(commanding["gimbalPitch"]["text"], "-90");

        let untouched = item_camera_view(&Item(Some(false)), &["3".into()]);
        assert_eq!(untouched["commandsGimbal"], false);
        assert_eq!(untouched["gimbalPitch"], Value::Null, "the fact carries -90 whether or not the item commands the gimbal, so serving it unguarded tells a head this waypoint points the camera down when it leaves it alone");
        assert_eq!(untouched["available"], true, "the section is there; it simply does not specify a gimbal, which is not the same as having no section");
        assert_eq!(untouched["cameraAction"]["text"], "Take photo", "the camera action is not gated by the gimbal flag and still travels");

        let absent = item_camera_view(&Item(None), &["3".into()]);
        assert_eq!(absent["available"], false);
        assert_eq!(absent["commandsGimbal"], false);

        let unasked = item_camera_view(&Item(Some(true)), &[]);
        assert_eq!(unasked["available"], false, "no index names no item, which is not an item without a camera");
        assert!(unasked["reason"].as_str().unwrap().contains("index"));
    }
}
