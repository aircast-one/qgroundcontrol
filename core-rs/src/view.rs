use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::altitude;
use crate::battery;
use crate::calibration;
use crate::contract;
use crate::control;
use crate::fences;
use crate::flightmodes;
use crate::guided;
use crate::inspector;
use crate::instruments;
use crate::kml;
use crate::label;
use crate::links;
use crate::logs;
use crate::mapscale;
use crate::messages;
use crate::missionkinds;
use crate::plan;
use crate::planfile;
use crate::preflight;
use crate::radio;
use crate::read::value_string;
use crate::sensors;
use crate::settings;
use crate::setup;
use crate::speed;
use crate::survey;
use crate::takeoff;
use crate::terrain;
use crate::tlog;
use crate::vibration;
use crate::video;
use crate::warnings;
use crate::waypoints;
use crate::router::Backend;

pub struct View {
    pub path: &'static str,
    pub deps: &'static [&'static str],
    compute: fn(&dyn Backend, &[String]) -> Value,
}

pub const VIEWS: &[View] = &[
    View { path: "view.messages", deps: &["vehicle.formattedMessages"], compute: messages_view },
    View { path: "view.plan", deps: plan::DEPS, compute: plan::plan_view },
    View { path: "view.guidedActions", deps: guided::DEPS, compute: guided::guided_view },
    View { path: "view.guidedAltitude", deps: altitude::DEPS, compute: altitude::altitude_view },
    View { path: "view.guidedTakeoff", deps: takeoff::DEPS, compute: takeoff::takeoff_view },
    View { path: "view.guidedSpeed", deps: speed::DEPS, compute: speed::speed_view },
    View { path: "view.battery", deps: battery::DEPS, compute: battery::battery_view },
    View { path: "view.preflight", deps: preflight::DEPS, compute: preflight::preflight_view },
    View { path: "view.warnings", deps: warnings::DEPS, compute: warnings::warnings_view },
    View { path: "view.label", deps: label::DEPS, compute: label::label_view },
    View { path: "view.instruments", deps: instruments::DEPS, compute: instruments::instruments_view },
    View { path: "view.vibration", deps: vibration::DEPS, compute: vibration::vibration_view },
    View { path: "view.sensors", deps: sensors::DEPS, compute: sensors::sensors_view },
    View { path: "view.control", deps: control::DEPS, compute: control::control_view },
    View { path: "view.links", deps: links::DEPS, compute: links::links_view },
    View { path: "view.linkForm", deps: &[], compute: links::link_form_view },
    View { path: "view.mapScale", deps: mapscale::DEPS, compute: mapscale::map_scale_view },
    View { path: "view.terrainProfile", deps: terrain::DEPS, compute: terrain::terrain_view },
    View { path: "view.missionKinds", deps: missionkinds::DEPS, compute: missionkinds::kinds_view },
    View { path: "view.missionSeed", deps: missionkinds::DEPS, compute: missionkinds::seed_view },
    View { path: "view.calibration", deps: calibration::DEPS, compute: calibration::calibration_view },
    View { path: "view.radio", deps: radio::DEPS, compute: radio::radio_view },
    View { path: "view.logs", deps: logs::DEPS, compute: logs::logs_view },
    View { path: "view.inspector", deps: inspector::DEPS, compute: inspector::inspector_view },
    View { path: "view.flightModes", deps: flightmodes::DEPS, compute: flightmodes::flight_modes_view },
    View { path: "view.settings", deps: settings::DEPS, compute: settings::settings_view },
    View { path: "view.surveyStats", deps: survey::DEPS, compute: survey::survey_stats_view },
    View { path: "view.fences", deps: fences::DEPS, compute: fences::fences_view },
    View { path: "view.polygon", deps: &[], compute: fences::polygon_view },
    View { path: "view.setup", deps: setup::DEPS, compute: setup::setup_view },
    View { path: "view.video", deps: video::VIDEO_DEPS, compute: video::video_view },
    View { path: "view.camera", deps: video::CAMERA_DEPS, compute: video::camera_view },
    View { path: "view.tlog", deps: tlog::DEPS, compute: tlog::tlog_view },
    View { path: "view.contract", deps: contract::DEPS, compute: contract::contract_view },
    View { path: "view.planFile", deps: planfile::DEPS, compute: planfile::plan_file_view },
    View { path: "view.waypointsFile", deps: waypoints::DEPS, compute: waypoints::waypoints_view },
    View { path: "view.kmlFile", deps: kml::DEPS, compute: kml::kml_view },
];

pub fn owns(path: &str) -> bool {
    path.split(['.', '[']).next() == Some("view")
}

pub fn lookup(path: &str) -> Option<&'static View> {
    let (base, _) = split(path);
    VIEWS.iter().find(|view| view.path == base)
}

pub fn split_paths(csv: &str) -> Vec<String> {
    let (paths, last, _) = csv.chars().fold((Vec::new(), String::new(), 0usize), |(mut paths, mut current, depth), c| match (c, depth) {
        (',', 0) => {
            paths.push(std::mem::take(&mut current));
            (paths, current, depth)
        }
        ('(', _) => {
            current.push(c);
            (paths, current, depth + 1)
        }
        (')', _) => {
            current.push(c);
            (paths, current, depth.saturating_sub(1))
        }
        _ => {
            current.push(c);
            (paths, current, depth)
        }
    });
    paths.into_iter().chain(std::iter::once(last)).map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
}

pub fn split(path: &str) -> (&str, Vec<String>) {
    let Some((base, rest)) = path.split_once('(') else { return (path, Vec::new()) };
    let inner = rest.strip_suffix(')').unwrap_or(rest);
    if inner.trim().is_empty() {
        return (base, Vec::new());
    }
    let (args, last, _) = inner.chars().fold((Vec::new(), String::new(), 0usize), |(mut args, mut current, depth), c| match (c, depth) {
        (',', 0) => {
            args.push(std::mem::take(&mut current));
            (args, current, depth)
        }
        ('(', _) => {
            current.push(c);
            (args, current, depth + 1)
        }
        (')', _) => {
            current.push(c);
            (args, current, depth.saturating_sub(1))
        }
        _ => {
            current.push(c);
            (args, current, depth)
        }
    });
    (base, args.into_iter().chain(std::iter::once(last)).map(|a| a.trim().to_string()).collect())
}

impl View {
    pub fn render(&self, backend: &dyn Backend, path: &str) -> String {
        (self.compute)(backend, &split(path).1).to_string()
    }

    pub fn render_fields(&self, backend: &dyn Backend, path: &str, fields: &str) -> String {
        let value = (self.compute)(backend, &split(path).1);
        let Value::Object(map) = value else { return value.to_string() };
        if fields.trim() == "*" {
            return Value::Object(map).to_string();
        }
        let wanted: BTreeSet<&str> = fields.split(',').map(str::trim).filter(|f| !f.is_empty()).collect();
        let unknown: Vec<&str> = wanted.iter().copied().filter(|f| !map.contains_key(*f)).collect();
        let kept = map
            .into_iter()
            .filter(|(key, _)| key == "kind" || key == "class" || wanted.contains(key.as_str()))
            .chain((!unknown.is_empty()).then(|| ("unknownFields".to_string(), json!(unknown))))
            .collect();
        Value::Object(kept).to_string()
    }
}

fn messages_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let items = messages::parse(&value_string(&backend.get("vehicle.formattedMessages")));
    json!({ "kind": "object", "class": "VehicleMessages", "count": items.len(), "items": items })
}

