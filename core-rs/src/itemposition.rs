use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

// A mission item is placed by writing a QGeoCoordinate property: coordinate, a takeoff's
// launchCoordinate, a landing pattern's landingCoordinate or finalApproachCoordinate. The heads send
// latitude and longitude with altitude 0, and the bridge turns a missing altitude into 0 as well.
// SimpleMissionItem::setCoordinate takes only the latitude and longitude, but MissionSettingsItem
// stores the whole coordinate - and TakeoffMissionItem hands it its coordinate whenever the launch
// is at the takeoff, and its launchCoordinate always - so placing a takeoff zeroed the planned home
// altitude. The written coordinate keeps the altitude the property already holds.
const POSITION_ITEMS: &str = "plan.missionController.visualItems.";
const MEMBERS: [&str; 4] = ["coordinate", "launchCoordinate", "landingCoordinate", "finalApproachCoordinate"];
const WIZARD: &str = "wizardMode";

pub fn target(path: &str) -> Option<(usize, &str)> {
    let (index, member) = path.strip_prefix(POSITION_ITEMS)?.split_once('.')?;
    (MEMBERS.contains(&member) || member == WIZARD).then_some(())?;
    Some((index.parse().ok()?, member))
}

pub fn owns(path: &str) -> bool {
    target(path).is_some()
}

fn refused(token: &str, reason: String) -> Value {
    json!({ "ok": false, "result": false, "refusal": token, "reason": reason })
}

pub fn write(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let Some((index, member)) = target(path) else {
        return refused("malformed", "That is not a mission item position the core writes.".to_string());
    };
    if flag(&object(&backend.get_fields("plan", "syncInProgress")), "syncInProgress") {
        return refused("busy", "Wait for the sync to finish before changing the plan.".to_string());
    }
    let item = object(&backend.get_fields(&format!("{POSITION_ITEMS}{index}"), member));
    if item.get("kind").and_then(Value::as_str) != Some("object") || item.get(member).is_none() {
        return refused("noSuchItem", format!("Item {index} has no {member} to set."));
    }
    let asked = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned()).unwrap_or(Value::Null);
    let sent = match member {
        WIZARD => match asked.as_bool() {
            Some(on) => json!(on),
            None => return refused("malformed", "wizardMode is true or false.".to_string()),
        },
        _ => {
            let Some((latitude, longitude)) = crate::fenceedit::point(Some(&asked)) else {
                return refused("badCoordinate", "A position needs a latitude from -90 to 90 and a longitude from -180 to 180.".to_string());
            };
            let mut at = json!({ "latitude": latitude, "longitude": longitude });
            if let Some(altitude) = item[member].get("altitude").and_then(Value::as_f64).filter(|a| a.is_finite()) {
                at["altitude"] = json!(altitude);
            }
            at
        }
    };
    let answered = flag(&object(&backend.set(path, &json!({ "value": sent }).to_string())), "ok");
    json!({ "ok": answered, "result": answered, "refusal": Value::Null, "reason": match answered { true => Value::Null, false => json!("The plan did not take that position.") } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Plan {
        syncing: bool,
        written: RefCell<Vec<(String, Value)>>,
    }

    impl Backend for Plan {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "plan" => json!({ "kind": "object", "syncInProgress": self.syncing }),
                "plan.missionController.visualItems.1" => json!({ "kind": "object",
                    "coordinate": { "latitude": 47.39, "longitude": 8.54, "altitude": 30.0, "valid": true },
                    "launchCoordinate": { "latitude": 47.39, "longitude": 8.54, "altitude": 488.0, "valid": true },
                    "wizardMode": true }),
                "plan.missionController.visualItems.2" => json!({ "kind": "object", "coordinate": { "latitude": 47.4, "longitude": 8.55, "valid": true } }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, p: &str, v: &str) -> String {
            self.written.borrow_mut().push((p.to_string(), object(v)["value"].clone()));
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn placing_a_takeoff_keeps_the_planned_home_altitude() {
        let plan = Plan { syncing: false, written: RefCell::new(Vec::new()) };
        let launch = "plan.missionController.visualItems.1.launchCoordinate";
        assert!(crate::actions::owns_write(launch));
        assert_eq!(write(&plan, launch, r#"{"value":{"latitude":47.3985,"longitude":8.5461,"altitude":0}}"#)["ok"], true);
        assert_eq!(
            plan.written.borrow()[0].1,
            json!({ "latitude": 47.3985, "longitude": 8.5461, "altitude": 488.0 }),
            "MissionSettingsItem stores the whole coordinate, so the head's altitude 0 became the planned home altitude"
        );
        let _ = write(&plan, "plan.missionController.visualItems.2.coordinate", r#"{"value":{"latitude":47.41,"longitude":8.56,"altitude":0}}"#);
        assert_eq!(plan.written.borrow()[1].1, json!({ "latitude": 47.41, "longitude": 8.56 }), "with no altitude held, none is invented");
        assert_eq!(write(&plan, "plan.missionController.visualItems.1.coordinate", r#"{"value":{"latitude":147,"longitude":8}}"#)["refusal"], "badCoordinate");
        assert_eq!(write(&plan, "plan.missionController.visualItems.5.coordinate", r#"{"value":{"latitude":47,"longitude":8}}"#)["refusal"], "noSuchItem");
        assert_eq!(write(&plan, "plan.missionController.visualItems.2.launchCoordinate", r#"{"value":{"latitude":47,"longitude":8}}"#)["refusal"], "noSuchItem", "only a takeoff has a launch coordinate");
        assert_eq!(write(&plan, "plan.missionController.visualItems.1.wizardMode", r#"{"value":"false"}"#)["refusal"], "malformed");
        assert_eq!(write(&plan, "plan.missionController.visualItems.1.wizardMode", r#"{"value":false}"#)["ok"], true);
        let busy = Plan { syncing: true, written: RefCell::new(Vec::new()) };
        assert_eq!(write(&busy, launch, r#"{"value":{"latitude":47,"longitude":8}}"#)["refusal"], "busy");
        assert!(!owns("plan.missionController.visualItems.1.altitude"), "a fact write stays with the fact check");
    }
}
