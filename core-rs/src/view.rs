use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::altitude;
use crate::altitudemodes;
use crate::battery;
use crate::calibration;
use crate::contract;
use crate::control;
use crate::fences;
use crate::flightmodes;
use crate::missionitems;
use crate::vehicles;
use crate::missionsummary;
use crate::modeslots;
use crate::flystate;
use crate::operatorcontrol;
use crate::orbit;
use crate::vehiclelinks;
use crate::geo;
use crate::adsb;
use crate::cameradef;
use crate::debugapi;
use crate::packetradio;
use crate::geotag;
use crate::gpsrtk;
use crate::videostate;
use crate::cameraproto;
use crate::joystick;
use crate::detections;
use crate::followme;
use crate::gcsposition;
use crate::gimbal;
use crate::guided;
use crate::inspector;
use crate::instruments;
use crate::landing;
use crate::obstacle;
use crate::kml;
use crate::label;
use crate::links;
use crate::logs;
use crate::mapscale;
use crate::messages;
use crate::missionkinds;
use crate::plan;
use crate::hub;
use crate::linkhost;
use crate::mission;
use crate::planfile;
use crate::preflight;
use crate::radio;
use crate::read::value_string;
use crate::sensors;
use crate::settings;
use crate::shp;
use crate::setup;
use crate::speed;
use crate::survey;
use crate::takeoff;
use crate::terrain;
use crate::terraintile;
use crate::track;
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

pub const ARGUMENT_MODES: &[(&str, &str)] = &[
    ("view.altitudeModes", "item,<index>"),
    ("view.cameraDefinition", "<file path>[,<locale>]"),
    ("view.debugApi", "<method>,<path>[,<query>]"),
    ("view.geoTag", "<file path>[,<tolerance seconds>]"),
    ("view.packetRadio", "<status>[,<adapter>[,<stats>]]"),
    ("view.gpsRtkBase", "<gps type>"),
    ("view.videoSource", "<source>[,<url>[,<rtsp timeout seconds>]]"),
    ("view.control", "<fact path>"),
    ("view.guidedAltitude", "<metres>"),
    ("view.guidedSpeed", "<metres per second>"),
    ("view.guidedTakeoff", "<metres>"),
    ("view.instruments", "<group/fact>,..."),
    ("view.landingPattern", "<index>"),
    ("view.mapScale", "<pixels>"),
    ("view.missionItems", "geometry | fields"),
    ("view.missionKinds", "<kind id>"),
    ("view.missionSummary", "verify"),
    ("view.polygon", "<path>[,line]"),
    ("view.settings", "<page>"),
    ("view.setup", "<page>"),
    ("view.surveyStats", "<index>"),
    ("view.label", "<fact name>"),
    ("view.linkForm", "<type>,<host>,<port>"),
    ("view.missionSeed", "<kind>,<latitude>,<longitude>"),
    ("view.tlog", "<file path>"),
    ("view.planFile", "<file path>[,<firmware>]"),
    ("view.waypointsFile", "<file path>"),
    ("view.planFromWaypoints", "<file path>"),
    ("view.missionFile", "<file path>"),
    ("view.kmlFile", "<file path>"),
    ("view.shapeFile", "<file path>"),
    ("view.terrainTile", "<file path>,<latitude>,<longitude>"),
    ("view.geoToNed", "<lat>,<lon>,<alt>,<originLat>,<originLon>,<originAlt>"),
    ("view.nedToGeo", "<north>,<east>,<down>,<originLat>,<originLon>,<originAlt>"),
    ("view.geoToUtm", "<latitude>,<longitude>"),
    ("view.utmToGeo", "<easting>,<northing>,<zone>[,<southern>]"),
    ("view.coreVehicle", "<vehicle id>"),
    ("view.coreGuided", "<vehicle id>"),
    ("view.coreParameter", "<vehicle id>,<name>"),
    ("view.coreParameters", "<vehicle id>"),
    ("view.coreMission", "<vehicle id>"),
    ("view.coreRemoteId", "<vehicle id>"),
    ("view.coreCalibration", "<vehicle id>"),
];

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
    View { path: "view.obstacle", deps: obstacle::DEPS, compute: obstacle::obstacle_view },
    View { path: "view.landingPattern", deps: landing::DEPS, compute: landing::landing_view },
    View { path: "view.vibration", deps: vibration::DEPS, compute: vibration::vibration_view },
    View { path: "view.sensors", deps: sensors::DEPS, compute: sensors::sensors_view },
    View { path: "view.control", deps: control::DEPS, compute: control::control_view },
    View { path: "view.links", deps: links::DEPS, compute: links::links_view },
    View { path: "view.linkForm", deps: &[], compute: links::link_form_view },
    View { path: "view.mapScale", deps: mapscale::DEPS, compute: mapscale::map_scale_view },
    View { path: "view.terrainProfile", deps: terrain::DEPS, compute: terrain::terrain_view },
    View { path: "view.vehicles", deps: vehicles::DEPS, compute: vehicles::vehicles_view },
    View { path: "view.missionItems", deps: missionitems::DEPS, compute: missionitems::items_view },
    View { path: "view.missionSummary", deps: missionsummary::DEPS, compute: missionsummary::summary_view },
    View { path: "view.modeSlots", deps: modeslots::DEPS, compute: modeslots::slots_view },
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
    View { path: "view.planFromWaypoints", deps: planfile::DEPS, compute: planfile::plan_from_waypoints_view },
    View { path: "view.missionFile", deps: mission::DEPS, compute: mission::mission_file_view },
    View { path: "view.transports", deps: &[], compute: linkhost::transports_view },
    View { path: "view.track", deps: track::DEPS, compute: track::track_view },
    View { path: "view.altitudeModes", deps: altitudemodes::DEPS, compute: altitudemodes::altitude_modes_view },
    View { path: "view.flyState", deps: flystate::DEPS, compute: flystate::fly_state_view },
    View { path: "view.operatorControl", deps: operatorcontrol::DEPS, compute: operatorcontrol::operator_control_view },
    View { path: "view.orbit", deps: orbit::DEPS, compute: orbit::orbit_view },
    View { path: "view.vehicleLinks", deps: vehiclelinks::DEPS, compute: vehiclelinks::vehicle_links_view },
    View { path: "view.coreVehicle", deps: &[], compute: hub::core_vehicle_view },
    View { path: "view.coreGuided", deps: &[], compute: hub::core_guided_view },
    View { path: "view.coreParameter", deps: &[], compute: hub::core_parameter_view },
    View { path: "view.coreParameters", deps: &[], compute: hub::core_parameters_view },
    View { path: "view.coreMission", deps: &[], compute: hub::core_mission_view },
    View { path: "view.coreRemoteId", deps: &[], compute: hub::core_remote_id_view },
    View { path: "view.coreCalibration", deps: &[], compute: hub::core_calibration_view },
    View { path: "view.detections", deps: detections::DEPS, compute: detections::detections_view },
    View { path: "view.adsbTraffic", deps: adsb::DEPS, compute: adsb::adsb_traffic_view },
    View { path: "view.cameraDefinition", deps: cameradef::DEPS, compute: cameradef::camera_definition_view },
    View { path: "view.cameraProtocol", deps: &[], compute: cameraproto::protocol_view },
    View { path: "view.joystickMapping", deps: &[], compute: joystick::joystick_view },
    View { path: "view.followMe", deps: followme::DEPS, compute: followme::follow_me_view },
    View { path: "view.gcsPosition", deps: &[], compute: gcsposition::gcs_position_view },
    View { path: "view.gimbal", deps: &[], compute: gimbal::gimbal_view },
    View { path: "view.debugApi", deps: debugapi::DEPS, compute: debugapi::debug_api_view },
    View { path: "view.geoTag", deps: geotag::DEPS, compute: geotag::geotag_view },
    View { path: "view.packetRadio", deps: &[], compute: packetradio::packet_radio_view },
    View { path: "view.gpsRtkBase", deps: &[], compute: gpsrtk::base_view },
    View { path: "view.videoSource", deps: videostate::DEPS, compute: videostate::video_source_view },
    View { path: "view.kmlFile", deps: kml::DEPS, compute: kml::kml_view },
    View { path: "view.shapeFile", deps: shp::DEPS, compute: shp::shp_view },
    View { path: "view.geoToNed", deps: geo::DEPS, compute: geo::geo_to_ned_view },
    View { path: "view.nedToGeo", deps: geo::DEPS, compute: geo::ned_to_geo_view },
    View { path: "view.geoToUtm", deps: geo::DEPS, compute: geo::geo_to_utm_view },
    View { path: "view.utmToGeo", deps: geo::DEPS, compute: geo::utm_to_geo_view },
    View { path: "view.terrainTile", deps: terraintile::DEPS, compute: terraintile::terrain_tile_view },
    View { path: "view.dependencies", deps: &[], compute: dependencies_view },
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
    pub fn deps_for(&self, args: &[String]) -> Vec<String> {
        match self.path {
            "view.instruments" => instruments::deps_for(args),
            "view.battery" => battery::deps(),
            "view.coreRemoteId" => crate::remoteidview::deps(),
            _ => self.deps.iter().map(|d| d.to_string()).collect(),
        }
    }

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

pub const ORDER: &str = "oldestFirst";

fn dependencies_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    json!({
        "kind": "object",
        "class": "ViewDependencies",
        "count": VIEWS.len(),
        "views": VIEWS.iter().map(|view| json!({ "path": view.path, "deps": view.deps })).collect::<Vec<_>>(),
        "argumentModes": ARGUMENT_MODES.iter().map(|(path, shape)| json!({ "path": path, "arguments": shape })).collect::<Vec<_>>(),
    })
}

fn messages_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let items = messages::parse(&value_string(&backend.get("vehicle.formattedMessages")));
    json!({ "kind": "object", "class": "VehicleMessages", "order": ORDER, "count": items.len(), "items": items })
}

#[cfg(test)]
mod watch_paths {
    use super::split_paths;

    #[test]
    fn a_view_path_carrying_commas_inside_its_arguments_survives_the_list() {
        let asked = "view.instruments(altitudeRelative,groundSpeed,distanceToHome,heading)";
        assert_eq!(split_paths(asked), vec![asked.to_string()]);
        assert_eq!(
            split_paths(&format!("view.plan,{asked},view.messages")),
            vec!["view.plan".to_string(), asked.to_string(), "view.messages".to_string()],
            "and it survives having neighbours, which is the case that actually happens"
        );
        assert_eq!(split_paths("view.a(x),view.b(y,z)"), vec!["view.a(x)".to_string(), "view.b(y,z)".to_string()]);
    }
}

#[cfg(test)]
mod deps_cover_reads {
    use std::collections::BTreeSet;

    const UNWATCHED_BECAUSE_CONSTANT: &[&str] = &[
        "vehicle.flightModeSetAvailable",
        "vehicle.rtlFlightMode",
        "vehicle.landFlightMode",
        "links.linkTypeStrings",
    "vehicle.roiModeSupported",
    "links.linkTypeIds",
        "links.serialBaudRates",
        "radioCal.channelCount",
        "vehicle.guidedModeSupported",
        "vehicle.takeoffVehicleSupported",
        "vehicle.pauseVehicleSupported",
        "vehicle.hasGripper",
        "vehicle.smartRTLFlightMode",
        "vehicle.missionFlightMode",
        "vehicle.pauseFlightMode",
    ];

    fn deps_of(module: &str) -> Option<BTreeSet<String>> {
        let start = module.find("pub const DEPS: &[&str] = &[")?;
        let rest = &module[start..];
        let end = rest.find("];")?;
        Some(literals(&rest[..end]).into_iter().collect())
    }

    fn literals(text: &str) -> Vec<String> {
        text.split('"').skip(1).step_by(2).map(str::to_string).collect()
    }

    fn literal_reads(body: &str) -> Vec<(String, String)> {
        body.match_indices("get_fields(")
            .filter_map(|(at, _)| {
                let tail = &body[at..];
                let close = tail.find(')')?;
                let args = literals(&tail[..close]);
                match args.len() {
                    2 => Some((args[0].clone(), args[1].clone())),
                    _ => None,
                }
            })
            .collect()
    }

    fn covered(path: &str, field: &str, deps: &BTreeSet<String>) -> bool {
        let full = format!("{path}.{field}");
        UNWATCHED_BECAUSE_CONSTANT.contains(&full.as_str())
            || deps.iter().any(|dep| *dep == full || dep == path || full.starts_with(&format!("{dep}.")) || dep.starts_with(&format!("{full}.")))
    }

    #[test]
    fn every_field_a_view_reads_is_one_it_watches() {
        let modules: &[(&str, &str)] = &[
            ("altitudemodes", include_str!("altitudemodes.rs")),
            ("calibration", include_str!("calibration.rs")),
            ("control", include_str!("control.rs")),
            ("fences", include_str!("fences.rs")),
            ("flightmodes", include_str!("flightmodes.rs")),
            ("flystate", include_str!("flystate.rs")),
            ("guided", include_str!("guided.rs")),
            ("links", include_str!("links.rs")),
            ("logs", include_str!("logs.rs")),
            ("missionitems", include_str!("missionitems.rs")),
            ("missionsummary", include_str!("missionsummary.rs")),
            ("modeslots", include_str!("modeslots.rs")),
            ("plan", include_str!("plan.rs")),
            ("preflight", include_str!("preflight.rs")),
            ("setup", include_str!("setup.rs")),
            ("survey", include_str!("survey.rs")),
            ("terrain", include_str!("terrain.rs")),
            ("vehicles", include_str!("vehicles.rs")),
        ];
        let unwatched: Vec<String> = modules
            .iter()
            .filter_map(|(name, source)| {
                let body = source.split("#[cfg(test)]").next()?;
                let deps = deps_of(body)?;
                let missing: Vec<String> = literal_reads(body)
                    .into_iter()
                    .flat_map(|(path, fields)| {
                        fields
                            .split(',')
                            .map(str::trim)
                            .filter(|field| !field.is_empty() && !covered(&path, field, &deps))
                            .map(|field| format!("{name}: {path}.{field}"))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                (!missing.is_empty()).then_some(missing)
            })
            .flatten()
            .collect();
        assert!(
            unwatched.is_empty(),
            "these are read by a view and named in no dep, so the view is never recomputed when they change. Watch them, or add them to \
             UNWATCHED_BECAUSE_CONSTANT once you have checked the Q_PROPERTY really is CONSTANT: {unwatched:?}"
        );
    }

    #[test]
    fn the_check_can_fail() {
        let deps: BTreeSet<String> = ["vehicle.armed".to_string()].into_iter().collect();
        assert!(covered("vehicle", "armed", &deps));
        assert!(!covered("vehicle", "flying", &deps), "a field named in no dep and on no constant list has to come back uncovered, or the sweep above passes by construction");
        assert!(covered("vehicle", "rtlFlightMode", &deps), "the constant list is the escape hatch, and it has to work");
        assert_eq!(literal_reads(r#"let x = backend.get_fields("vehicle", "armed,flying");"#), vec![("vehicle".to_string(), "armed,flying".to_string())]);
    }
}

#[cfg(test)]
mod argument_modes {
    use super::*;

    fn source(module: &str) -> String {
        std::fs::read_to_string(format!("{}/src/{module}.rs", env!("CARGO_MANIFEST_DIR"))).unwrap_or_default()
    }

    #[test]
    fn a_view_that_refuses_its_arguments_says_what_it_wanted() {
        let modules: std::collections::BTreeSet<String> = source("view")
            .lines()
            .filter_map(|line| Some(line.split("compute: ").nth(1)?.split("::").next()?.to_string()))
            .collect();
        assert!(modules.len() > 20, "the registry parsed, so an empty answer below would mean nothing");
        let silent: Vec<&String> = modules
            .iter()
            .filter(|module| source(module).lines().any(|line| line.contains("return json!({ \"kind\": \"null\" })") && !line.contains(".to_string()")))
            .collect();
        assert!(silent.is_empty(), "these refuse an argument they cannot use and say nothing about what they wanted, which is the case where a caller most needs telling: {silent:?}");
    }

    #[test]
    fn every_view_that_reads_its_arguments_declares_what_they_are() {
        let registry = source("view");
        let wired: Vec<(String, String)> = registry
            .lines()
            .filter_map(|line| {
                let path = line.split("View { path: \"").nth(1)?.split('"').next()?;
                let compute = line.split("compute: ").nth(1)?.split(&[',', ' ', '}'][..]).next()?;
                Some((path.to_string(), compute.to_string()))
            })
            .collect();
        assert!(wired.len() > 40, "the registry parsed, so an empty answer below would mean something");

        let declared: Vec<&str> = ARGUMENT_MODES.iter().map(|(path, _)| *path).collect();
        let reads_arguments = |compute: &str| {
            let (module, function) = compute.split_once("::").unwrap_or(("view", compute));
            source(module).lines().any(|line| line.contains(&format!("fn {function}(")) && line.contains(", args: &[String]"))
        };

        let undeclared: Vec<&String> = wired.iter().filter(|(path, m)| reads_arguments(m) && !declared.contains(&path.as_str())).map(|(p, _)| p).collect();
        assert!(undeclared.is_empty(), "these views read their arguments and ARGUMENT_MODES does not say so, which is the list both heads' sweeps enumerate from: {undeclared:?}");

        let unread: Vec<&&str> = declared.iter().filter(|path| wired.iter().any(|(p, m)| p == *path && !reads_arguments(m))).collect();
        assert!(unread.is_empty(), "and these are declared as taking arguments by a compute function that ignores them: {unread:?}");
    }
}
