use std::fs::File;
use std::io::Write;
use std::sync::{Mutex, PoisonError};

use serde_json::Value;

use crate::read::object;
use crate::router::Backend;

const LINE_INTERVAL_MS: u64 = 1000;

struct Csv {
    file: File,
    columns: Vec<String>,
}

struct State {
    open: std::collections::BTreeMap<u8, Csv>,
    last_ms: u64,
}

static STATE: Mutex<State> = Mutex::new(State { open: std::collections::BTreeMap::new(), last_ms: 0 });

pub struct Logged {
    pub id: u8,
    pub listed: usize,
    pub armed: bool,
}

fn setting(path: &str) -> bool {
    crate::settingsstore::raw_setting(path).and_then(|v| v.as_bool()).unwrap_or(false)
}

pub fn columns() -> Vec<String> {
    let (groups, vehicle) = crate::vehiclefact::instrument_catalogue(false);
    let names = |listing: &Value| -> Vec<String> {
        listing["facts"].as_array().cloned().unwrap_or_default().iter().filter_map(|fact| fact["property"].as_str().map(str::to_string)).collect()
    };
    names(&vehicle)
        .into_iter()
        .chain(groups.iter().flat_map(|(group, listing)| names(listing).into_iter().map(move |name| format!("{group}.{name}"))))
        .collect()
}

pub fn header(columns: &[String]) -> String {
    format!("Timestamp,{}\n", columns.join(","))
}

fn value(backend: &dyn Backend, listed: usize, column: &str) -> String {
    object(&backend.get(&format!("vehicles.vehicles.{listed}.{column}"))).get("valueString").and_then(Value::as_str).unwrap_or_default().to_string()
}

fn open(vehicle: u8) -> Option<Csv> {
    let folder = std::path::PathBuf::from(crate::settingsstore::telemetry_save_path()?);
    std::fs::create_dir_all(&folder).ok()?;
    let name = format!("{} vehicle{vehicle}.csv", chrono::Local::now().format("%Y-%m-%d %H-%M-%S"));
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(folder.join(name)).ok()?;
    let columns = columns();
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
    state.open.retain(|id, _| wanted && vehicles.iter().any(|v| v.id == *id));
    vehicles.iter().filter(|_| wanted).for_each(|vehicle| {
        if !state.open.contains_key(&vehicle.id) && (vehicle.armed || not_armed) {
            if let Some(csv) = open(vehicle.id) {
                state.open.insert(vehicle.id, csv);
            }
        }
        let Some(csv) = state.open.get_mut(&vehicle.id) else { return };
        let line = std::iter::once(chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()).chain(csv.columns.iter().map(|column| value(backend, vehicle.listed, column))).collect::<Vec<_>>().join(",");
        if csv.file.write_all(format!("{line}\n").as_bytes()).is_err() {
            state.open.remove(&vehicle.id);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_names_vehicle_facts_then_group_facts_by_group() {
        let listed = columns();
        assert!(listed.contains(&"roll".to_string()), "{:?}", &listed[..5]);
        assert!(listed.contains(&"gps.lat".to_string()));
        assert!(listed.iter().position(|c| c == "roll") < listed.iter().position(|c| c == "gps.lat"), "the vehicle's own facts come before its groups");
        assert_eq!(header(&["a".to_string(), "g.b".to_string()]), "Timestamp,a,g.b\n");
    }
}
