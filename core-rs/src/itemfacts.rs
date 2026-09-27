use serde_json::{Value, json};

use crate::read::{flag, object, refused};
use crate::router::Backend;

// The macOS mission item editor read four raw groups per selected item - the item itself, its
// textFieldFacts and comboboxFacts lists, and cameraCalc - and chose among them in the head: the
// lists when the item has any, else a complex item's own facts, plus the camera block for a
// complex item. Each fact is served here in view.control's shape, with the path the head writes.
pub const DEPS: &[&str] = &[
    "plan.missionController.visualItems.count",
    "plan.missionController@visualItemsReset",
    "plan.dirty",
    "settings.unitsSettings.horizontalDistanceUnits",
    "settings.unitsSettings.verticalDistanceUnits",
];

const ITEM_ROOT: &str = "plan.missionController.visualItems";
const LISTS: [&str; 2] = ["textFieldFacts", "comboboxFacts"];
// Mission Settings edits the launch altitude through its own control, so it is not a field here.
const LAUNCH_ALTITUDE: &str = "plannedHomePositionAltitude";
const SURVEY_PROPERTIES: [&str; 4] = ["distanceToSurface", "imageDensity", "frontalOverlap", "sideOverlap"];
const OPTICS_PROPERTIES: [&str; 7] = ["sensorWidth", "sensorHeight", "imageWidth", "imageHeight", "focalLength", "landscape", "minTriggerInterval"];

fn field(fact: &Value, item: &str, suffix: &str, group: &str) -> Value {
    let mut control = crate::control::decode(fact, &format!("{item}.{suffix}"));
    if let Value::Object(map) = &mut control {
        map.insert("pathSuffix".to_string(), json!(suffix));
        map.insert("group".to_string(), json!(group));
    }
    control
}

fn named(fact: &Value) -> bool {
    fact.get("name").and_then(Value::as_str).is_some_and(|name| !name.is_empty())
}

fn listed(backend: &dyn Backend, item: &str) -> Vec<Value> {
    LISTS
        .iter()
        .flat_map(|list| {
            let read = object(&backend.get(&format!("{item}.{list}")));
            let elements = read.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
            elements.into_iter().enumerate().filter(|(_, fact)| named(fact)).map(move |(at, fact)| field(&fact, item, &format!("{list}.{at}"), "Settings")).collect::<Vec<_>>()
        })
        .collect()
}

fn owned(read: &Value, item: &str) -> Vec<Value> {
    read.get("facts")
        .and_then(Value::as_array)
        .map(|facts| {
            facts
                .iter()
                .filter(|fact| named(fact))
                .filter_map(|fact| {
                    let property = fact.get("property").and_then(Value::as_str).filter(|p| !p.is_empty() && *p != LAUNCH_ALTITUDE)?;
                    Some(field(fact, item, property, "Settings"))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn strings(read: &Value, key: &str) -> Vec<String> {
    read.get(key).and_then(Value::as_array).map(|list| list.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

fn camera(backend: &dyn Backend, item: &str) -> Value {
    let read = object(&backend.get(&format!("{item}.cameraCalc")));
    if read.get("kind").and_then(Value::as_str) != Some("object") {
        return Value::Null;
    }
    let text = |key: &str| read.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let custom = flag(&read, "isCustomCamera");
    let wanted: Vec<&str> = match custom {
        true => OPTICS_PROPERTIES.iter().chain(SURVEY_PROPERTIES.iter()).copied().collect(),
        false => SURVEY_PROPERTIES.to_vec(),
    };
    let facts = read.get("facts").and_then(Value::as_array).cloned().unwrap_or_default();
    let shown: Vec<Value> = wanted
        .iter()
        .filter_map(|property| facts.iter().find(|fact| fact.get("property").and_then(Value::as_str) == Some(property)))
        .filter(|fact| named(fact))
        .map(|fact| {
            let property = fact.get("property").and_then(Value::as_str).unwrap_or_default();
            field(fact, item, &format!("cameraCalc.{property}"), "Camera")
        })
        .collect();
    json!({
        "brand": text("cameraBrand"),
        "model": text("cameraModel"),
        "brands": strings(&read, "cameraBrandList"),
        "models": strings(&read, "cameraModelList"),
        "manualName": text("xlatManualCameraName"),
        "customName": text("xlatCustomCameraName"),
        "custom": custom,
        "distanceMode": read.get("distanceMode").cloned().unwrap_or(Value::Null),
        "brandPath": format!("{item}.cameraCalc.cameraBrand"),
        "modelPath": format!("{item}.cameraCalc.cameraModel"),
        "facts": shown,
    })
}

pub fn item_facts_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(index) = args.first().and_then(|a| a.parse::<usize>().ok()) else {
        return refused("view.itemFacts needs the index of the item in the plan, as view.itemFacts(3)");
    };
    let item = format!("{ITEM_ROOT}.{index}");
    let read = object(&backend.get(&item));
    let available = read.get("kind").and_then(Value::as_str) == Some("object");
    let simple = flag(&read, "isSimpleItem");
    let lists = match available {
        true => listed(backend, &item),
        false => Vec::new(),
    };
    let fields = match (lists.is_empty(), available && !simple) {
        (true, true) => owned(&read, &item),
        _ => lists,
    };
    json!({
        "kind": "object",
        "class": "ItemFacts",
        "available": available,
        "index": index,
        "simple": simple,
        "fields": fields,
        "camera": match available && !simple {
            true => camera(backend, &item),
            false => Value::Null,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(name: &str, property: &str, value: f64) -> Value {
        json!({ "kind": "fact", "name": name, "property": property, "value": value, "valueString": format!("{value}"), "units": "m", "readOnly": false, "shortDescription": "" })
    }

    struct Plan {
        simple: bool,
        lists: bool,
        custom: bool,
    }

    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.2" => json!({ "kind": "object", "isSimpleItem": self.simple, "facts": [fact("Altitude", "altitude", 50.0), fact("Launch", LAUNCH_ALTITUDE, 0.0)] }),
                "plan.missionController.visualItems.2.textFieldFacts" if self.lists => json!({ "kind": "list", "elements": [fact("Hold", "", 5.0), json!({ "kind": "fact", "name": "" })] }),
                "plan.missionController.visualItems.2.comboboxFacts" if self.lists => json!({ "kind": "list", "elements": [fact("Mode", "", 1.0)] }),
                "plan.missionController.visualItems.2.cameraCalc" => json!({
                    "kind": "object", "cameraBrand": "Sony", "cameraModel": "RX100", "cameraBrandList": ["Manual", "Sony"], "cameraModelList": ["RX100"],
                    "xlatManualCameraName": "Manual (no camera specs)", "xlatCustomCameraName": "Custom Camera", "isCustomCamera": self.custom, "distanceMode": 1,
                    "facts": [fact("SensorWidth", "sensorWidth", 13.2), fact("FrontalOverlap", "frontalOverlap", 70.0), fact("DistanceToSurface", "distanceToSurface", 50.0)],
                }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, path: &str, _f: &str) -> String { self.get(path) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn suffixes(view: &Value, key: &str) -> Vec<String> {
        view[key].as_array().unwrap().iter().map(|f| f["pathSuffix"].as_str().unwrap().to_string()).collect()
    }

    #[test]
    fn a_simple_item_edits_its_lists_and_carries_no_camera() {
        let view = item_facts_view(&Plan { simple: true, lists: true, custom: false }, &["2".to_string()]);
        assert_eq!(suffixes(&view, "fields"), ["textFieldFacts.0", "comboboxFacts.0"], "an unnamed element is not a field anyone can edit, and the position stays the list's own");
        assert_eq!(view["fields"][0]["path"], "plan.missionController.visualItems.2.textFieldFacts.0", "the path is the one the head writes");
        assert!(crate::factwrite::owns(view["fields"][0]["path"].as_str().unwrap()), "and that write is one the core validates");
        assert_eq!(view["fields"][0]["class"], "Control", "each field is view.control's shape, which the head already decodes");
        assert_eq!(view["camera"], Value::Null);
    }

    #[test]
    fn a_complex_item_without_lists_edits_its_own_facts_and_its_camera() {
        let view = item_facts_view(&Plan { simple: false, lists: false, custom: false }, &["2".to_string()]);
        assert_eq!(suffixes(&view, "fields"), ["altitude"], "Mission Settings edits the launch altitude through its own control");
        let camera = &view["camera"];
        assert_eq!((&camera["brand"], &camera["model"], &camera["custom"], &camera["distanceMode"]), (&json!("Sony"), &json!("RX100"), &json!(false), &json!(1)));
        assert_eq!(camera["brands"], json!(["Manual", "Sony"]));
        assert_eq!(camera["brandPath"], "plan.missionController.visualItems.2.cameraCalc.cameraBrand");
        let shown: Vec<&str> = camera["facts"].as_array().unwrap().iter().map(|f| f["pathSuffix"].as_str().unwrap()).collect();
        assert_eq!(shown, ["cameraCalc.distanceToSurface", "cameraCalc.frontalOverlap"], "a catalogue camera shows only the survey figures, in the head's order");
        assert_eq!(camera["facts"][0]["group"], "Camera");

        let custom = item_facts_view(&Plan { simple: false, lists: false, custom: true }, &["2".to_string()]);
        assert_eq!(custom["camera"]["facts"][0]["pathSuffix"], "cameraCalc.sensorWidth", "a custom camera's optics come first, then the survey figures");

        let listed = item_facts_view(&Plan { simple: false, lists: true, custom: false }, &["2".to_string()]);
        assert_eq!(suffixes(&listed, "fields"), ["textFieldFacts.0", "comboboxFacts.0"], "a complex item with lists edits the lists, as the head chose");
    }

    #[test]
    fn an_item_that_is_not_there_says_so() {
        let view = item_facts_view(&Plan { simple: true, lists: false, custom: false }, &["9".to_string()]);
        assert_eq!((&view["available"], &view["fields"], &view["camera"]), (&json!(false), &json!([]), &Value::Null));
        assert_eq!(item_facts_view(&Plan { simple: true, lists: false, custom: false }, &[])["kind"], "null");
    }
}
