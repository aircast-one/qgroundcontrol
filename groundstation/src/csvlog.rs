use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::sync::{Mutex, PoisonError};

use serde_json::Value;

use crate::read::object;
use crate::router::Backend;

const LINE_INTERVAL_MS: u64 = 1000;

#[derive(Debug, Clone, PartialEq)]
enum Source {
    Path(String),
    Member { list: &'static str, id: u64, name: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    header: String,
    source: Source,
}

struct Csv {
    file: File,
    columns: Vec<Column>,
}

struct State {
    open: BTreeMap<u8, Csv>,
    last_ms: u64,
}

static STATE: Mutex<State> = Mutex::new(State { open: BTreeMap::new(), last_ms: 0 });

pub struct Logged {
    pub id: u8,
    pub listed: usize,
    pub armed: bool,
    pub sub: bool,
}

const MEMBER_LISTS: [(&str, &str); 2] = [("batteries", "battery"), ("escs", "escStatus")];

fn setting(path: &str) -> bool {
    crate::settingsstore::raw_setting(path).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn member_path(listed: usize, tail: &str) -> String {
    format!("vehicles.vehicles.{listed}.{tail}")
}

fn listed_facts(listing: &Value) -> Vec<(String, String)> {
    listing["facts"].as_array().map_or_else(Vec::new, |facts| facts.iter().filter_map(|fact| Some((fact["property"].as_str()?.to_string(), fact["name"].as_str()?.to_string()))).collect())
}

fn fact_group_name(group: &str) -> &str {
    match group {
        "distanceSensors" => "distanceSensor",
        other => other,
    }
}

fn element_id(element: &Value) -> Option<u64> {
    element["facts"].as_array()?.iter().find(|fact| fact["name"] == "id").and_then(|fact| fact["rawValue"].as_f64().or_else(|| fact["value"].as_f64())).map(|id| id as u64)
}

fn list_names(list: &str) -> Vec<String> {
    match list {
        "batteries" => crate::vehiclefact::BATTERY_FACT_NAMES.iter().map(|name| name.to_string()).collect(),
        _ => crate::vehiclefact::ESC.added.iter().map(|name| name.to_string()).collect(),
    }
}

pub fn columns(backend: &dyn Backend, listed: usize, sub: bool, gimbals: &[String]) -> Vec<Column> {
    let (groups, vehicle) = crate::vehiclefact::instrument_catalogue(sub);
    let own = listed_facts(&vehicle).into_iter().map(|(property, name)| Column { header: name, source: Source::Path(property) });
    let fixed = groups
        .into_iter()
        .filter(|(group, _)| *group != "orbitMapCircle" && (sub || *group != crate::vehiclefact::SUB_INFO_GROUP))
        .map(|(group, listing)| (fact_group_name(group).to_string(), listed_facts(&listing).into_iter().map(|(property, name)| (format!("{group}.{property}"), name)).collect::<Vec<_>>()))
        .map(|(group, facts)| (group.clone(), facts.into_iter().map(|(path, name)| Column { header: format!("{group}.{name}"), source: Source::Path(path) }).collect::<Vec<_>>()));
    let members = MEMBER_LISTS.iter().flat_map(|(list, prefix)| {
        let elements = object(&backend.get(&member_path(listed, list)))["elements"].as_array().cloned().unwrap_or_default();
        elements.iter().filter_map(element_id).map(|id| (format!("{prefix}{id}"), list_names(list).into_iter().map(|name| Column { header: format!("{prefix}{id}.{name}"), source: Source::Member { list, id, name } }).collect::<Vec<_>>())).collect::<Vec<_>>()
    });
    let gimbal_groups = gimbals.iter().map(|group| (group.clone(), crate::vehiclefact::GIMBAL_FACT_NAMES.iter().map(|name| Column { header: format!("{group}.{name}"), source: Source::Path(format!("{group}.{name}")) }).collect::<Vec<_>>()));
    let by_group_name: BTreeMap<String, Vec<Column>> = fixed.chain(members).chain(gimbal_groups).collect();
    own.chain(by_group_name.into_values().flatten()).collect()
}

pub fn header(columns: &[Column]) -> String {
    format!("Timestamp,{}\n", columns.iter().map(|column| column.header.as_str()).collect::<Vec<_>>().join(","))
}

fn value_string(fact: &Value) -> String {
    fact.get("valueString").and_then(Value::as_str).unwrap_or_default().to_string()
}

fn values(backend: &dyn Backend, listed: usize, columns: &[Column]) -> Vec<String> {
    let lists: BTreeMap<&str, Value> = MEMBER_LISTS.iter().filter(|(list, _)| columns.iter().any(|column| matches!(&column.source, Source::Member { list: wanted, .. } if wanted == list))).map(|(list, _)| (*list, object(&backend.get(&member_path(listed, list))))).collect();
    columns
        .iter()
        .map(|column| match &column.source {
            Source::Path(path) => value_string(&object(&backend.get(&member_path(listed, path)))),
            Source::Member { list, id, name } => lists
                .get(list)
                .and_then(|members| members["elements"].as_array()?.iter().find(|element| element_id(element) == Some(*id)).cloned())
                .and_then(|element| element["facts"].as_array()?.iter().find(|fact| fact["name"] == name.as_str()).map(value_string))
                .unwrap_or_default(),
        })
        .collect()
}

fn open(backend: &dyn Backend, vehicle: &Logged) -> Option<Csv> {
    let folder = std::path::PathBuf::from(crate::settingsstore::telemetry_save_path()?);
    std::fs::create_dir_all(&folder).ok()?;
    let name = format!("{} vehicle{}.csv", chrono::Local::now().format("%Y-%m-%d %H-%M-%S"), vehicle.id);
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(folder.join(name)).ok()?;
    let gimbals: Vec<String> = crate::hub::lock().vehicle(vehicle.id).map(|v| v.gimbals.fact_groups()).unwrap_or_default().into_iter().map(|(pair, _)| crate::vehiclefact::gimbal_group_name(pair)).collect();
    let columns = columns(backend, vehicle.listed, vehicle.sub, &gimbals);
    file.write_all(header(&columns).as_bytes()).ok()?;
    Some(Csv { file, columns })
}

pub fn tick(backend: &dyn Backend, vehicles: &[Logged], now_ms: u64) {
    let mut state = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    if now_ms.saturating_sub(state.last_ms) < LINE_INTERVAL_MS {
        return;
    }
    state.last_ms = now_ms;
    let wanted = setting("settings.mavlinkSettings.saveCsvTelemetry");
    let not_armed = setting("settings.mavlinkSettings.telemetrySaveNotArmed");
    state.open.retain(|id, _| vehicles.iter().any(|v| v.id == *id));
    vehicles.iter().for_each(|vehicle| {
        if wanted
            && !state.open.contains_key(&vehicle.id)
            && (vehicle.armed || not_armed)
            && let Some(csv) = open(backend, vehicle)
        {
            state.open.insert(vehicle.id, csv);
        }
        let Some(csv) = state.open.get_mut(&vehicle.id) else { return };
        let line = std::iter::once(chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()).chain(values(backend, vehicle.listed, &csv.columns)).collect::<Vec<_>>().join(",");
        if csv.file.write_all(format!("{line}\n").as_bytes()).is_err() {
            state.open.remove(&vehicle.id);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Fleet;
    impl Backend for Fleet {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicles.vehicles.0.batteries" => json!({ "kind": "object", "elements": [
                    { "facts": [{ "name": "id", "rawValue": 0 }, { "name": "voltage", "valueString": "16.1" }] },
                    { "facts": [{ "name": "id", "rawValue": 12 }, { "name": "voltage", "valueString": "15.9" }] },
                ] })
                .to_string(),
                "vehicles.vehicles.0.escs" => json!({ "kind": "object", "elements": [{ "facts": [{ "name": "id", "rawValue": 0 }, { "name": "rpm", "valueString": "4200" }] }] }).to_string(),
                "vehicles.vehicles.0.roll" => json!({ "kind": "fact", "valueString": "1.5" }).to_string(),
                "vehicles.vehicles.0.hygrometer.hygroID" => json!({ "kind": "fact", "valueString": "7" }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            json!({ "kind": "null" }).to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn headers(sub: bool) -> Vec<String> {
        columns(&Fleet, 0, sub, &[]).into_iter().map(|column| column.header).collect()
    }

    #[test]
    fn columns_are_the_vehicle_facts_then_every_fact_group_in_fact_group_names_order() {
        let listed = headers(false);
        assert_eq!(listed[..3], ["roll", "pitch", "heading"], "Vehicle::factNames in _addFact order");
        let groups: Vec<&str> = listed.iter().filter_map(|header| header.split_once('.').map(|(group, _)| group)).fold(Vec::new(), |seen, group| if seen.last() == Some(&group) { seen } else { [seen, vec![group]].concat() });
        assert_eq!(
            groups,
            ["aircastLink", "battery0", "battery12", "clock", "distanceSensor", "efi", "escStatus0", "estimatorStatus", "generator", "gps", "gps2", "gpsAggregate", "hygrometer", "localPosition", "localPositionSetpoint", "radioStatus", "rpm", "setpoint", "temperature", "terrain", "vibration", "wind"],
            "factGroupNames is a QMap key list: battery<id>/escStatus<id> sort among the fixed groups, distanceSensor is the group's name, orbitMapCircle is no fact group and apmSubInfo only comes with ArduSub"
        );
        assert!(listed.iter().any(|header| header == "hygrometer.temperature"), "headers are fact names, not property names");
        assert_eq!(listed.iter().filter(|header| header.starts_with("battery0.")).take(4).collect::<Vec<_>>(), ["battery0.id", "battery0.batteryFunction", "battery0.batteryType", "battery0.voltage"]);
        assert!(headers(true).iter().any(|header| header == "apmSubInfo.cameraTilt"));
        assert_eq!(header(&[Column { header: "a".into(), source: Source::Path("a".into()) }, Column { header: "g.b".into(), source: Source::Path("g.b".into()) }]), "Timestamp,a,g.b\n");
    }

    #[test]
    fn a_complete_gimbal_adds_its_fact_group_columns_in_fact_group_names_order() {
        let listed: Vec<String> = columns(&Fleet, 0, false, &["gimbal1154".to_string()]).into_iter().map(|column| column.header).collect();
        let at = |header: &str| listed.iter().position(|h| h == header).unwrap();
        assert!(at("generator.status") < at("gimbal1154.gimbalRoll") && at("gimbal1154.managerCompid") < at("gps.lat"), "gimbal<manager><device> sorts between generator and gps in the QMap");
        assert_eq!(listed.iter().filter(|h| h.starts_with("gimbal1154.")).collect::<Vec<_>>(), ["gimbal1154.gimbalRoll", "gimbal1154.gimbalPitch", "gimbal1154.gimbalYaw", "gimbal1154.gimbalAzimuth", "gimbal1154.deviceId", "gimbal1154.managerCompid"]);
    }

    #[test]
    fn a_line_reads_paths_and_each_list_member_by_its_id() {
        let picked: Vec<Column> = columns(&Fleet, 0, false, &[]).into_iter().filter(|column| ["roll", "battery12.voltage", "escStatus0.rpm", "hygrometer.hygrometerid", "battery0.current"].contains(&column.header.as_str())).collect();
        let headers: Vec<&str> = picked.iter().map(|column| column.header.as_str()).collect();
        assert_eq!(headers, ["roll", "battery0.current", "battery12.voltage", "escStatus0.rpm", "hygrometer.hygrometerid"]);
        assert_eq!(values(&Fleet, 0, &picked), ["1.5", "", "15.9", "4200", "7"]);
    }
}
