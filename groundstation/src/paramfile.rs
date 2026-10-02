use serde_json::{Value, json};

use crate::read::{flag, integer, object, text};
use crate::router::Backend;

pub const FILE_SAVE: &str = "parameterFile.save";
pub const FILE_REVIEW: &str = "parameterFile.review";
pub const FILE_APPLY: &str = "parameterFile.apply";

const MAV_PARAM_TYPE_REAL32: u8 = 9;
const MAV_PARAM_TYPE_REAL64: u8 = 10;
const AUTOPILOT_ARDUPILOT: i64 = 3;
const AUTOPILOT_PX4: i64 = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct FileParam {
    pub vehicle: Option<i64>,
    pub component: i64,
    pub name: String,
    pub value: String,
    pub mav_type: Option<u8>,
    pub mission_planner: bool,
}

pub fn parse(text: &str) -> Vec<FileParam> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(|c: char| c == '\t' || c == ' ' || c == ',').filter(|f| !f.is_empty()).collect();
            match fields.as_slice() {
                [vehicle, component, name, value, mav_type] => Some(FileParam {
                    vehicle: vehicle.parse().ok(),
                    component: component.parse().unwrap_or(0),
                    name: (*name).to_string(),
                    value: (*value).to_string(),
                    mav_type: mav_type.parse().ok(),
                    mission_planner: false,
                }),
                [name, value] => Some(FileParam { vehicle: None, component: -1, name: (*name).to_string(), value: (*value).to_string(), mav_type: None, mission_planner: true }),
                _ => None,
            }
        })
        .collect()
}

pub fn raw_text(value: f64, mav_type: u8) -> String {
    let text = match mav_type {
        MAV_PARAM_TYPE_REAL32 => format!("{}", value as f32),
        MAV_PARAM_TYPE_REAL64 => format!("{value}"),
        _ => format!("{}", value as i64),
    };
    if text == "-0" { "0".to_string() } else { text }
}

fn stack(firmware: i64) -> &'static str {
    match firmware {
        AUTOPILOT_PX4 => "PX4 Pro",
        AUTOPILOT_ARDUPILOT => "ArduPilot",
        _ => "Generic",
    }
}

fn vehicle_class(mav_type: i64) -> &'static str {
    match mav_type {
        10 | 11 => "Rover-Boat",
        12 => "Sub",
        31 => "Spacecraft",
        2 | 3 | 4 | 13 | 14 | 15 => "Multi-Rotor",
        19..=25 => "VTOL",
        1 => "Fixed Wing",
        7 => "Airship",
        _ => "Generic",
    }
}

fn parameter_path(component: i64, name: &str) -> String {
    format!("vehicle.parameterManager.getParameter({component},{name})")
}

fn fact(backend: &dyn Backend, component: i64, name: &str) -> Option<Value> {
    let read = object(&backend.get(&parameter_path(component, name)));
    let present = read.get("kind").and_then(Value::as_str) == Some("fact") && read.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
    present.then_some(read)
}

fn components(backend: &dyn Backend) -> Vec<i64> {
    let listed = object(&backend.invoke("vehicle.parameterManager.componentIds", "[]"));
    listed.get("result").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_i64).collect::<Vec<_>>()).filter(|c| !c.is_empty()).unwrap_or_else(|| vec![-1])
}

fn names(backend: &dyn Backend, component: i64) -> Vec<String> {
    let listed = object(&backend.invoke("vehicle.parameterManager.parameterNames", &json!([component]).to_string()));
    listed
        .get("result")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect::<std::collections::BTreeSet<_>>().into_iter().collect())
        .unwrap_or_default()
}

pub fn save(backend: &dyn Backend) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "id,firmwareType,vehicleType,firmwareMajorVersion,firmwareMinorVersion,firmwarePatchVersion,firmwareVersionTypeString,gitHash"));
    let Some(id) = integer(&vehicle, "id") else { return json!({ "ok": false, "reason": "No vehicle is connected." }) };
    let whole = |key: &str| integer(&vehicle, key).unwrap_or(0);
    let header = [
        format!("# Onboard parameters for Vehicle {id}"),
        "#".to_string(),
        format!("# Stack: {}", stack(whole("firmwareType"))),
        format!("# Vehicle: {}", vehicle_class(whole("vehicleType"))),
        format!("# Version: {}.{}.{} {}", whole("firmwareMajorVersion"), whole("firmwareMinorVersion"), whole("firmwarePatchVersion"), text(&vehicle, "firmwareVersionTypeString")),
        format!("# Git Revision: {}", text(&vehicle, "gitHash")),
        "#".to_string(),
        "# Vehicle-Id Component-Id Name Value Type".to_string(),
    ];
    let rows: Vec<String> = components(backend)
        .into_iter()
        .flat_map(|asked| names(backend, asked).into_iter().map(move |name| (asked, name)))
        .filter_map(|(asked, name)| {
            let read = fact(backend, asked, &name)?;
            let mav_type = read.get("mavType").and_then(Value::as_u64).and_then(|t| u8::try_from(t).ok())?;
            let component = integer(&read, "componentId").unwrap_or(1);
            let raw = read.get("rawValue").and_then(Value::as_f64)?;
            Some(format!("{id}\t{component}\t{name}\t{}\t{mav_type}", raw_text(raw, mav_type)))
        })
        .collect();
    let body: Vec<String> = header.into_iter().chain(rows).collect();
    json!({ "ok": true, "result": body.join("\n") + "\n" })
}

fn same(vehicle_raw: f64, file: f64, mav_type: u8) -> bool {
    match mav_type {
        MAV_PARAM_TYPE_REAL32 => (vehicle_raw as f32) == (file as f32),
        MAV_PARAM_TYPE_REAL64 => vehicle_raw == file,
        _ => vehicle_raw.round() == file.trunc(),
    }
}

fn shown(read: &Value, raw: f64) -> String {
    let labels = read.get("enumStrings").and_then(Value::as_array).cloned().unwrap_or_default();
    let values = read.get("enumValues").and_then(Value::as_array).cloned().unwrap_or_default();
    labels
        .iter()
        .zip(values.iter())
        .find(|(_, v)| v.as_f64() == Some(raw) || v.as_str().and_then(|s| s.parse::<f64>().ok()) == Some(raw))
        .and_then(|(label, _)| label.as_str().map(str::to_string))
        .unwrap_or_else(|| raw_text(raw, read.get("mavType").and_then(Value::as_u64).map_or(MAV_PARAM_TYPE_REAL32, |t| t as u8)))
}

pub fn review(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_str().map(str::to_string)).unwrap_or_default();
    let Some(vehicle_id) = integer(&object(&backend.get_fields("vehicle", "id")), "id") else { return json!({ "ok": false, "reason": "No vehicle is connected." }) };
    let parsed = parse(&given);
    if parsed.is_empty() {
        return json!({ "ok": false, "reason": "No valid parameters found in file. Check that the file is in QGC or Mission Planner format." });
    }
    let first_component = parsed.iter().find(|p| !p.mission_planner).map(|p| p.component);
    let other_vehicle = parsed.iter().any(|p| p.vehicle.is_some_and(|v| v != vehicle_id));
    let multiple_components = parsed.iter().any(|p| !p.mission_planner && Some(p.component) != first_component);
    let rows: Vec<Value> = parsed
        .iter()
        .filter_map(|p| {
            let number = p.value.parse::<f64>().ok();
            match fact(backend, p.component, &p.name) {
                Some(read) => {
                    let mav_type = p.mav_type.or_else(|| read.get("mavType").and_then(Value::as_u64).and_then(|t| u8::try_from(t).ok())).unwrap_or(MAV_PARAM_TYPE_REAL32);
                    let vehicle_raw = read.get("rawValue").and_then(Value::as_f64)?;
                    let file = number?;
                    if same(vehicle_raw, file, mav_type) {
                        return Some(json!({ "unchanged": true }));
                    }
                    if flag(&read, "readOnly") {
                        return Some(json!({ "readOnly": true }));
                    }
                    Some(json!({
                        "componentId": p.component,
                        "name": p.name,
                        "fileValue": shown(&read, file),
                        "fileRaw": file,
                        "vehicleValue": read.get("enumOrValueString").and_then(Value::as_str).map_or_else(|| shown(&read, vehicle_raw), str::to_string),
                        "units": text(&read, "units"),
                        "noVehicleValue": false,
                        "cannotSend": false,
                    }))
                }
                None if p.mission_planner => Some(json!({ "componentId": p.component, "name": p.name, "fileValue": p.value, "fileRaw": number, "vehicleValue": "", "units": "", "noVehicleValue": true, "cannotSend": true })),
                None => number.map(|file| json!({ "componentId": p.component, "name": p.name, "fileValue": p.value, "fileRaw": file, "vehicleValue": "", "units": "", "noVehicleValue": true, "cannotSend": p.mav_type.is_none(), "mavType": p.mav_type })),
            }
        })
        .collect();
    let count = |key: &str| rows.iter().filter(|r| flag(r, key)).count();
    let listed: Vec<&Value> = rows.iter().filter(|r| r.get("name").is_some()).collect();
    json!({
        "ok": true,
        "result": {
            "otherVehicle": other_vehicle,
            "multipleComponents": multiple_components,
            "parsed": parsed.len(),
            "unchanged": count("unchanged"),
            "readOnly": count("readOnly"),
            "rows": listed,
        },
    })
}

pub fn apply(backend: &dyn Backend, args: &str) -> Value {
    let chosen = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_array().cloned()).unwrap_or_default();
    let outcomes: Vec<(String, bool)> = chosen
        .iter()
        .filter_map(|row| {
            let name = row.get("name")?.as_str()?.to_string();
            let component = row.get("componentId").and_then(Value::as_i64).unwrap_or(-1);
            let value = row.get("fileRaw").and_then(Value::as_f64)?;
            let sendable = !flag(row, "cannotSend");
            let written = match (sendable, fact(backend, component, &name).is_some()) {
                (false, _) => false,
                (true, true) => flag(&crate::factwrite::write(backend, &parameter_path(component, &name), &json!({ "value": value }).to_string()), "ok"),
                (true, false) => {
                    let vehicle = crate::hub::lock().active_id();
                    let raw = json!({ "action": "paramSetRaw", "vehicle": vehicle, "component": component, "name": name, "value": value, "type": row.get("mavType") });
                    vehicle.and_then(|_| backend.core_guided(&raw)).is_some_and(|sent| sent.is_ok())
                }
            };
            Some((name, written))
        })
        .collect();
    let failed: Vec<&String> = outcomes.iter().filter(|(_, ok)| !ok).map(|(name, _)| name).collect();
    json!({
        "ok": failed.is_empty(),
        "sent": outcomes.len() - failed.len(),
        "reason": if failed.is_empty() { Value::Null } else { json!(format!("Not sent: {}", failed.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(", "))) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    struct Vehicle(RefCell<BTreeMap<String, (f64, u8)>>);

    impl Backend for Vehicle {
        fn get(&self, path: &str) -> String {
            let name = path.rsplit(',').next().unwrap_or_default().trim_end_matches(')');
            match self.0.borrow().get(name) {
                Some((value, mav_type)) => json!({ "kind": "fact", "name": name, "rawValue": value, "value": value, "mavType": mav_type, "componentId": 1, "enumOrValueString": value.to_string(), "units": "m" }),
                None => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            json!({ "kind": "object", "id": 1, "firmwareType": 3, "vehicleType": 2, "firmwareMajorVersion": 4, "firmwareMinorVersion": 6, "firmwarePatchVersion": 1, "firmwareVersionTypeString": "Official", "gitHash": "abc" }).to_string()
        }
        fn set(&self, path: &str, value: &str) -> String {
            let name = path.rsplit(',').next().unwrap_or_default().trim_end_matches(')').to_string();
            let v = serde_json::from_str::<Value>(value).unwrap()["value"].as_f64().unwrap();
            let kind = self.0.borrow()[&name].1;
            self.0.borrow_mut().insert(name, (v, kind));
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true, "result": self.0.borrow().keys().cloned().collect::<Vec<_>>() }).to_string() }
        fn watch(&self, _p: &[String]) {}
    }

    fn vehicle() -> Vehicle {
        Vehicle(RefCell::new(BTreeMap::from([("RTL_ALT".to_string(), (1500.0, 6)), ("WPNAV_SPEED".to_string(), (0.1, 9))])))
    }

    #[test]
    fn saves_in_qgc_layout() {
        let saved = save(&vehicle())["result"].as_str().unwrap().to_string();
        let lines: Vec<&str> = saved.lines().collect();
        assert_eq!(lines[0], "# Onboard parameters for Vehicle 1");
        assert_eq!(lines[2], "# Stack: ArduPilot");
        assert_eq!(lines[3], "# Vehicle: Multi-Rotor");
        assert_eq!(lines[4], "# Version: 4.6.1 Official");
        assert_eq!(lines[7], "# Vehicle-Id Component-Id Name Value Type");
        assert_eq!(lines[8], "1\t1\tRTL_ALT\t1500\t6");
        assert_eq!(lines[9], "1\t1\tWPNAV_SPEED\t0.1\t9", "a float is written in the shortest form that reads back to the same bits");
        assert_eq!(parse(&saved).len(), 2, "and it reads back");
    }

    #[test]
    fn every_component_with_parameters_is_saved() {
        struct Two;
        impl Backend for Two {
            fn get(&self, path: &str) -> String {
                let (component, name) = path.trim_end_matches(')').rsplit_once('(').unwrap().1.split_once(',').unwrap();
                json!({ "kind": "fact", "name": name, "rawValue": 1.0, "mavType": 6, "componentId": component.parse::<i64>().unwrap() }).to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "id": 1, "firmwareType": 3, "vehicleType": 2 }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                match (path, args) {
                    ("vehicle.parameterManager.componentIds", _) => json!({ "ok": true, "result": [1, 154] }),
                    (_, "[1]") => json!({ "ok": true, "result": ["RTL_ALT"] }),
                    (_, "[154]") => json!({ "ok": true, "result": ["MNT1_TYPE"] }),
                    _ => json!({ "ok": false }),
                }
                .to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let saved = save(&Two)["result"].as_str().unwrap().to_string();
        let rows: Vec<&str> = saved.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(rows, ["1\t1\tRTL_ALT\t1\t6", "1\t154\tMNT1_TYPE\t1\t6"], "ParameterManager::writeParametersToStream walks every component's map");
    }

    #[test]
    fn parses_both_formats() {
        let read = parse("# comment\n1\t1\tRTL_ALT\t1500\t6\nRTL_ALT,1200\n\ngarbage line here\n");
        assert_eq!(read.len(), 2);
        assert_eq!(read[1], FileParam { vehicle: None, component: -1, name: "RTL_ALT".into(), value: "1200".into(), mav_type: None, mission_planner: true });
    }

    #[test]
    fn review_lists_only_what_would_change() {
        let file = "2\t1\tRTL_ALT\t2000\t6\n2\t1\tWPNAV_SPEED\t0.1\t9\n2\t1\tNEW_PARAM\t3\t6\nMP_ONLY 4\n";
        let reviewed = review(&vehicle(), &json!([file]).to_string());
        let result = &reviewed["result"];
        assert_eq!(result["otherVehicle"], true, "the file's vehicle id is 2");
        assert_eq!(result["unchanged"], 1, "0.1 in the file is the same float the vehicle holds");
        let rows = result["rows"].as_array().unwrap();
        assert_eq!(rows.iter().map(|r| r["name"].as_str().unwrap()).collect::<Vec<_>>(), ["RTL_ALT", "NEW_PARAM", "MP_ONLY"]);
        assert_eq!(rows[0]["fileValue"], "2000");
        assert_eq!(rows[2]["cannotSend"], true);
        assert_eq!((rows[1]["noVehicleValue"].clone(), rows[1]["cannotSend"].clone(), rows[1]["mavType"].clone()), (json!(true), json!(false), json!(6)), "ParameterEditorController keeps a QGC-format row the vehicle lacks sendable, typed from its own column");
        assert_eq!(review(&vehicle(), &json!(["# nothing"]).to_string())["ok"], false);
    }

    #[test]
    fn apply_writes_the_chosen_rows() {
        let vehicle = vehicle();
        let applied = apply(&vehicle, &json!([[{ "componentId": 1, "name": "RTL_ALT", "fileRaw": 2000.0 }, { "componentId": 1, "name": "NEW_PARAM", "fileRaw": 3.0, "noVehicleValue": true, "cannotSend": true }]]).to_string());
        assert_eq!(applied["sent"], 1);
        assert_eq!(vehicle.0.borrow()["RTL_ALT"].0, 2000.0);
        assert_eq!(applied["ok"], false, "a parameter the vehicle does not have cannot be written through a fact");
    }
}
