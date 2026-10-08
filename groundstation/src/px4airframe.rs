use std::sync::LazyLock;

use serde_json::{Value, json};

use crate::read::flag;
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicle.parameterManager.parametersReady",
    "vehicle.parameterManager.getParameter(-1,SYS_AUTOSTART).rawValue",
    "vehicles.vehicles.count",
];

pub const APPLY: &str = "px4Airframe.apply";
pub const RESET: &str = "px4Airframe.reset";
const AIRFRAMES_XML: &str = include_str!("../../src/AutoPilotPlugins/PX4/AirframeFactMetaData.xml");
const PRIORITY_TYPES: [&str; 3] = ["Quadrotor x", "Standard Plane", "Standard VTOL"];
const AUTOSTART: &str = "vehicle.parameterManager.getParameter(-1,SYS_AUTOSTART)";
const AUTOCONFIG: &str = "vehicle.parameterManager.getParameter(-1,SYS_AUTOCONFIG)";

#[derive(Debug, Clone, PartialEq)]
pub struct Airframe {
    pub name: String,
    pub autostart_id: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirframeType {
    pub name: String,
    pub image: String,
    pub airframes: Vec<Airframe>,
}

pub fn parse(xml: &str) -> Vec<AirframeType> {
    let Ok(document) = roxmltree::Document::parse(xml) else { return Vec::new() };
    let groups = document.descendants().filter(|node| node.has_tag_name("airframe_group")).filter_map(|group| {
        let airframes = group
            .children()
            .filter(|node| node.has_tag_name("airframe"))
            .filter_map(|airframe| Some(Airframe { name: airframe.attribute("name")?.to_string(), autostart_id: airframe.attribute("id")?.parse().ok()? }))
            .collect::<Vec<_>>();
        Some((group.attribute("name")?.to_string(), group.attribute("image")?.to_string(), airframes))
    });
    let merged = groups.fold(Vec::<AirframeType>::new(), |types, (name, image, airframes)| match types.iter().position(|t| t.name == name) {
        Some(at) => types.into_iter().enumerate().map(|(i, t)| if i == at { AirframeType { airframes: t.airframes.into_iter().chain(airframes.clone()).collect(), ..t } } else { t }).collect(),
        None => types.into_iter().chain(std::iter::once(AirframeType { name, image, airframes })).collect(),
    });
    let alphabetical: Vec<AirframeType> = {
        let mut sorted = merged;
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        sorted
    };
    PRIORITY_TYPES
        .iter()
        .filter_map(|name| alphabetical.iter().find(|t| t.name == *name).cloned())
        .chain(alphabetical.iter().filter(|t| !PRIORITY_TYPES.contains(&t.name.as_str())).cloned())
        .collect()
}

static TYPES: LazyLock<Vec<AirframeType>> = LazyLock::new(|| parse(AIRFRAMES_XML));

pub fn current(types: &[AirframeType], autostart: i64) -> Option<(usize, usize)> {
    types
        .iter()
        .enumerate()
        .flat_map(|(t, kind)| kind.airframes.iter().enumerate().filter(move |(_, a)| a.autostart_id == autostart).map(move |(i, _)| (t, i)))
        .last()
}

pub fn current_names(autostart: i64) -> Option<(String, String)> {
    current(&TYPES, autostart).map(|(t, i)| (TYPES[t].name.clone(), TYPES[t].airframes[i].name.clone()))
}

fn fact_value(backend: &dyn Backend, path: &str) -> Option<f64> {
    let fact = backend.value(path);
    (fact.get("kind").and_then(Value::as_str) == Some("fact")).then(|| fact.get("rawValue").or_else(|| fact.get("value")).and_then(Value::as_f64)).flatten()
}

pub fn heading(vehicle_name: Option<&str>) -> String {
    let lead = vehicle_name.map_or_else(|| "Airframe is not set.".to_string(), |name| format!("You've connected a {name}."));
    format!("{lead} To change this configuration, select the desired airframe below then click 'Apply and Restart'.")
}

pub fn airframe_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let (Some(autostart), Some(_)) = (fact_value(backend, AUTOSTART), fact_value(backend, AUTOCONFIG)) else {
        return json!({ "kind": "object", "class": "Px4Airframe", "available": false, "types": [] });
    };
    let autostart = autostart as i64;
    let found = current(&TYPES, autostart);
    let vehicle_name = match found {
        Some((t, i)) => Some(TYPES[t].airframes[i].name.clone()),
        None => Some(autostart.to_string()),
    };
    json!({
        "kind": "object",
        "class": "Px4Airframe",
        "available": true,
        "autostartId": autostart,
        "custom": autostart != 0 && found.is_none(),
        "customText": "Your vehicle is using a custom airframe configuration. This configuration can only be modified through the Parameter Editor.\n\nIf you want to reset your airframe configuration and select a standard configuration, click 'Reset' below.",
        "heading": heading(vehicle_name.as_deref().filter(|name| !name.is_empty() && autostart != 0)),
        "currentType": found.map(|(t, _)| TYPES[t].name.clone()),
        "currentIndex": found.map_or(0, |(_, i)| i),
        "applyTitle": "Apply and Restart",
        "applyText": "Clicking 'Apply' will save the changes you have made to your airframe configuration.<br><br>All vehicle parameters other than Radio Calibration will be reset.<br><br>Your vehicle will also be restarted in order to complete the process.",
        "types": TYPES.iter().map(|kind| json!({
            "name": kind.name,
            "image": kind.image,
            "airframes": kind.airframes.iter().map(|a| json!({ "name": a.name, "autostartId": a.autostart_id })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn vehicle_count(backend: &dyn Backend) -> i64 {
    backend.value("vehicles.vehicles.count").get("value").and_then(Value::as_i64).unwrap_or(0)
}

pub fn apply(backend: &dyn Backend, args: &str) -> Value {
    let Some(autostart) = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_i64()) else {
        return json!({ "ok": false, "reason": "Apply takes the autostart id of the airframe chosen." });
    };
    if vehicle_count(backend) > 1 {
        return json!({ "ok": false, "reason": "You cannot change airframe configuration while connected to multiple vehicles." });
    }
    let vehicle = crate::guided::active_id(backend);
    let on_core = vehicle.and_then(|id| backend.core_guided(&json!({ "action": "changeAutostart", "autostartId": autostart, "vehicle": id })));
    match on_core {
        Some(Ok(())) => json!({ "ok": true }),
        Some(Err(reason)) => json!({ "ok": false, "reason": reason }),
        None => {
            let written = [(AUTOSTART, autostart), (AUTOCONFIG, 1)].iter().all(|(path, value)| flag(&crate::factwrite::write(backend, path, &json!({ "value": value }).to_string()), "ok"));
            match written {
                true => crate::guided::dispatch(backend, None, vehicle, "vehicle.rebootVehicle", "[]"),
                false => json!({ "ok": false, "reason": "The vehicle did not take the airframe parameters." }),
            }
        }
    }
}

pub fn reset(backend: &dyn Backend) -> Value {
    crate::factwrite::write(backend, AUTOSTART, &json!({ "value": 0 }).to_string())
}

pub fn owns(path: &str) -> bool {
    [APPLY, RESET].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_frames_come_first_then_the_rest_alphabetically() {
        let names: Vec<&str> = TYPES.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(&names[..3], PRIORITY_TYPES);
        let rest: Vec<&str> = names[3..].to_vec();
        let mut sorted = rest.clone();
        sorted.sort();
        assert_eq!(rest, sorted);
        assert_eq!(TYPES.iter().map(|t| t.airframes.len()).sum::<usize>(), AIRFRAMES_XML.matches("<airframe ").count(), "every airframe in the bundled file is offered");
    }

    #[test]
    fn the_current_airframe_is_found_by_its_autostart_id() {
        let quad_x = TYPES.iter().position(|t| t.name == "Quadrotor x").unwrap();
        let first = &TYPES[quad_x].airframes[0];
        assert_eq!(current(&TYPES, first.autostart_id), Some((quad_x, 0)));
        assert_eq!(current(&TYPES, 999_999), None);
        assert_eq!(heading(None), "Airframe is not set. To change this configuration, select the desired airframe below then click 'Apply and Restart'.");
        assert!(heading(Some(&first.name)).starts_with(&format!("You've connected a {}.", first.name)));
    }

    #[test]
    fn groups_with_the_same_name_merge_in_file_order() {
        let xml = r#"<airframes><version>1</version><airframe_group name="B" image="b"><airframe name="b1" id="2"/></airframe_group><airframe_group name="A" image="a"><airframe name="a1" id="1"/></airframe_group><airframe_group name="B" image="b"><airframe name="b2" id="3"/></airframe_group></airframes>"#;
        let types = parse(xml);
        assert_eq!(types.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["A", "B"]);
        assert_eq!(types[1].airframes.iter().map(|a| a.autostart_id).collect::<Vec<_>>(), [2, 3]);
    }
}
