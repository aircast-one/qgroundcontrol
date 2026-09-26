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
use crate::frame;
use crate::console;
use crate::itemcamera;
use crate::gcsposition;
use crate::gimbal;
use crate::guided;
use crate::inspector;
use crate::instrumentgroups;
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
    ("view.itemCamera", "<item index>"),
    ("view.cameraDefinition", "<file path>[,<locale>]"),
    ("view.debugApi", "<method>,<path>[,<query>]"),
    ("view.geoTag", "<file path>[,<tolerance seconds>]"),
    ("view.packetRadio", "<status>[,<adapter>[,<stats>]]"),
    ("view.gpsRtkBase", "<gps type>"),
    ("view.videoSource", "<source>[,<url>[,<rtsp timeout seconds>]]"),
    ("view.control", "<fact path>"),
    ("view.guidedAltitude", "<metres>[,pause]"),
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
    ("view.supportHost", "<host or host:port>"),
    ("view.missionSeed", "<kind>,<latitude>,<longitude>"),
    ("view.tlog", "<file path>"),
    ("view.planFile", "<file path>"),
    ("view.waypointsFile", "<file path>"),
    ("view.planFromWaypoints", "<file path>[,<firmware type>[,<vehicle type>]]"),
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
    View { path: "view.instrumentGroups", deps: instrumentgroups::DEPS, compute: instrumentgroups::instrument_groups_view },
    View { path: "view.obstacle", deps: obstacle::DEPS, compute: obstacle::obstacle_view },
    View { path: "view.landingPattern", deps: landing::DEPS, compute: landing::landing_view },
    View { path: "view.vibration", deps: vibration::DEPS, compute: vibration::vibration_view },
    View { path: "view.sensors", deps: sensors::DEPS, compute: sensors::sensors_view },
    View { path: "view.control", deps: control::DEPS, compute: control::control_view },
    View { path: "view.links", deps: links::DEPS, compute: links::links_view },
    View { path: "view.linkForm", deps: &[], compute: links::link_form_view },
    View { path: "view.supportHost", deps: &[], compute: links::support_host_view },
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
    View { path: "view.transports", deps: linkhost::DEPS, compute: linkhost::transports_view },
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
    View { path: "view.frame", deps: frame::DEPS, compute: frame::frame_view },
    View { path: "view.gcsPosition", deps: gcsposition::DEPS, compute: gcsposition::gcs_position_view },
    View { path: "view.gimbal", deps: &[], compute: gimbal::gimbal_view },
    View { path: "view.debugApi", deps: debugapi::DEPS, compute: debugapi::debug_api_view },
    View { path: "view.geoTag", deps: geotag::DEPS, compute: geotag::geotag_view },
    View { path: "view.packetRadio", deps: &[], compute: packetradio::packet_radio_view },
    View { path: "view.mavlinkConsole", deps: console::DEPS, compute: console::console_view },
    View { path: "view.itemCamera", deps: itemcamera::DEPS, compute: itemcamera::item_camera_view },
    View { path: "view.gpsRtkBase", deps: gpsrtk::DEPS, compute: gpsrtk::base_view },
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

pub fn unknown(path: &str) -> Value {
    let (base, _) = split(path);
    let near: Vec<&str> = VIEWS
        .iter()
        .map(|view| view.path)
        .filter(|known| known.starts_with(base) || base.starts_with(known))
        .collect();
    crate::read::refused(&match near.is_empty() {
        true => format!("no such view: {base}"),
        false => format!("no such view: {base} - did you mean {}?", near.join(" or ")),
    })
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
    if ARGUMENT_MODES.iter().any(|(view, mode)| *view == base && *mode == "<file path>") {
        return (base, vec![inner.to_string()]);
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
            "view.vehicles" => vehicles::deps(),
            "view.followMe" => followme::deps(),
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
        "views": VIEWS.iter().map(|view| json!({ "path": view.path, "deps": view.deps_for(&[]) })).collect::<Vec<_>>(),
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

    // One list. A parallel const of just the names drifted from this within minutes of being
    // written: dropping a module here left the completeness check passing, which is the drift it
    // exists to prevent.
    const MODULES: &[(&str, &str)] = &[
            ("actions", include_str!("actions.rs")),
            ("adsb", include_str!("adsb.rs")),
            ("altitude", include_str!("altitude.rs")),
            ("altitudeedit", include_str!("altitudeedit.rs")),
            ("altitudemodes", include_str!("altitudemodes.rs")),
            ("apmmeta", include_str!("apmmeta.rs")),
            ("autoconnect", include_str!("autoconnect.rs")),
            ("battery", include_str!("battery.rs")),
            ("batteryfacts", include_str!("batteryfacts.rs")),
            ("boards", include_str!("boards.rs")),
            ("calibration", include_str!("calibration.rs")),
            ("cameracalc", include_str!("cameracalc.rs")),
            ("cameradef", include_str!("cameradef.rs")),
            ("cameratrack", include_str!("cameratrack.rs")),
            ("cameraproto", include_str!("cameraproto.rs")),
            ("cmdinfo", include_str!("cmdinfo.rs")),
            ("compinfo", include_str!("compinfo.rs")),
            ("compmeta", include_str!("compmeta.rs")),
            ("compression", include_str!("compression.rs")),
        ("console", include_str!("console.rs")),
            ("connect", include_str!("connect.rs")),
            ("contract", include_str!("contract.rs")),
            ("control", include_str!("control.rs")),
            ("corridorscan", include_str!("corridorscan.rs")),
            ("debugapi", include_str!("debugapi.rs")),
            ("detections", include_str!("detections.rs")),
            ("factmeta", include_str!("factmeta.rs")),
            ("factwrite", include_str!("factwrite.rs")),
            ("fenceedit", include_str!("fenceedit.rs")),
            ("fences", include_str!("fences.rs")),
            ("flightmodes", include_str!("flightmodes.rs")),
            ("flystate", include_str!("flystate.rs")),
            ("followme", include_str!("followme.rs")),
        ("frame", include_str!("frame.rs")),
            ("ftp", include_str!("ftp.rs")),
            ("gcsposition", include_str!("gcsposition.rs")),
            ("geo", include_str!("geo.rs")),
            ("geotag", include_str!("geotag.rs")),
            ("geotagjob", include_str!("geotagjob.rs")),
            ("gimbal", include_str!("gimbal.rs")),
            ("gpsfacts", include_str!("gpsfacts.rs")),
            ("gpsrtk", include_str!("gpsrtk.rs")),
            ("guided", include_str!("guided.rs")),
            ("guidedcmd", include_str!("guidedcmd.rs")),
            ("guidedexec", include_str!("guidedexec.rs")),
            ("hostnotice", include_str!("hostnotice.rs")),
            ("hub", include_str!("hub.rs")),
            ("inspector", include_str!("inspector.rs")),
            ("instrumentgroups", include_str!("instrumentgroups.rs")),
            ("instruments", include_str!("instruments.rs")),
        ("itemcamera", include_str!("itemcamera.rs")),
            ("joystick", include_str!("joystick.rs")),
            ("kml", include_str!("kml.rs")),
            ("label", include_str!("label.rs")),
            ("landing", include_str!("landing.rs")),
            ("linkconfig", include_str!("linkconfig.rs")),
            ("linkhost", include_str!("linkhost.rs")),
            ("links", include_str!("links.rs")),
            ("linkconnect", include_str!("linkconnect.rs")),
            ("linkremove", include_str!("linkremove.rs")),
            ("logs", include_str!("logs.rs")),
            ("mappolygon", include_str!("mappolygon.rs")),
            ("mappolyline", include_str!("mappolyline.rs")),
            ("mapscale", include_str!("mapscale.rs")),
            ("mavcmd", include_str!("mavcmd.rs")),
            ("mavout", include_str!("mavout.rs")),
            ("messages", include_str!("messages.rs")),
            ("metacache", include_str!("metacache.rs")),
            ("mission", include_str!("mission.rs")),
            ("missionitems", include_str!("missionitems.rs")),
            ("missionkinds", include_str!("missionkinds.rs")),
            ("missionsummary", include_str!("missionsummary.rs")),
            ("modes", include_str!("modes.rs")),
            ("modeslots", include_str!("modeslots.rs")),
            ("obstacle", include_str!("obstacle.rs")),
            ("operatorcontrol", include_str!("operatorcontrol.rs")),
            ("operatorrequest", include_str!("operatorrequest.rs")),
            ("orbit", include_str!("orbit.rs")),
            ("packetradio", include_str!("packetradio.rs")),
            ("params", include_str!("params.rs")),
            ("plan", include_str!("plan.rs")),
            ("planfile", include_str!("planfile.rs")),
            ("planselect", include_str!("planselect.rs")),
            ("plantransfer", include_str!("plantransfer.rs")),
            ("preflight", include_str!("preflight.rs")),
            ("px4meta", include_str!("px4meta.rs")),
            ("radio", include_str!("radio.rs")),
            ("rallyedit", include_str!("rallyedit.rs")),
            ("rcoverride", include_str!("rcoverride.rs")),
            ("read", include_str!("read.rs")),
            ("remoteid", include_str!("remoteid.rs")),
            ("remoteidview", include_str!("remoteidview.rs")),
            ("renamed", include_str!("renamed.rs")),
            ("replay", include_str!("replay.rs")),
            ("router", include_str!("router.rs")),
            ("rtcm", include_str!("rtcm.rs")),
            ("sensorcal", include_str!("sensorcal.rs")),
            ("sensorfacts", include_str!("sensorfacts.rs")),
            ("sensors", include_str!("sensors.rs")),
            ("seriallink", include_str!("seriallink.rs")),
            ("settings", include_str!("settings.rs")),
            ("settingsgroups", include_str!("settingsgroups.rs")),
            ("settingsini", include_str!("settingsini.rs")),
            ("setup", include_str!("setup.rs")),
            ("shp", include_str!("shp.rs")),
            ("signing", include_str!("signing.rs")),
            ("speed", include_str!("speed.rs")),
            ("standardmodes", include_str!("standardmodes.rs")),
            ("statustext", include_str!("statustext.rs")),
            ("structurescan", include_str!("structurescan.rs")),
            ("survey", include_str!("survey.rs")),
            ("surveygrid", include_str!("surveygrid.rs")),
            ("surveyitems", include_str!("surveyitems.rs")),
            ("sysstatus", include_str!("sysstatus.rs")),
            ("takeoff", include_str!("takeoff.rs")),
            ("tcplink", include_str!("tcplink.rs")),
            ("terrain", include_str!("terrain.rs")),
            ("terraintile", include_str!("terraintile.rs")),
            ("tilecache", include_str!("tilecache.rs")),
            ("tlog", include_str!("tlog.rs")),
            ("track", include_str!("track.rs")),
            ("transport", include_str!("transport.rs")),
            ("udplink", include_str!("udplink.rs")),
            ("ulogstream", include_str!("ulogstream.rs")),
            ("vehiclefacts", include_str!("vehiclefacts.rs")),
            ("vehiclelinks", include_str!("vehiclelinks.rs")),
            ("vehicles", include_str!("vehicles.rs")),
            ("vibration", include_str!("vibration.rs")),
            ("video", include_str!("video.rs")),
            ("videosource", include_str!("videosource.rs")),
            ("videostate", include_str!("videostate.rs")),
            ("view", include_str!("view.rs")),
            ("warnings", include_str!("warnings.rs")),
            ("waypoints", include_str!("waypoints.rs")),
    ];


    const UNWATCHED_BECAUSE_CONSTANT: &[&str] = &[
        "video.gstreamerEnabled",
        "vehicle.flightModeSetAvailable",
        "vehicle.rtlFlightMode",
        "vehicle.landFlightMode",
        "links.linkTypeStrings",
    "vehicle.supports.roiMode",
    "vehicle.supports.orbitMode",
    "vehicle.supports.radio",
    "links.linkTypeIds",
        "links.serialBaudRates",
        "radioCal.channelCount",
        "vehicle.supports.guidedMode",
        "vehicle.supports.guidedTakeoffWithAltitude",
        "vehicle.supports.guidedTakeoffWithoutAltitude",
        "vehicle.supports.pauseVehicle",
        "vehicle.hasGripper",
        "vehicle.smartRTLFlightMode",
        "vehicle.missionFlightMode",
        "vehicle.pauseFlightMode",
    ];

    fn deps_of(module: &str) -> Option<BTreeSet<String>> {
        let found: BTreeSet<String> = module
            .match_indices("DEPS: &[&str] = &[")
            .filter_map(|(at, _)| {
                let rest = &module[at..];
                let end = rest.find("];")?;
                Some(literals(&rest[..end]))
            })
            .flatten()
            .collect();
        (!found.is_empty()).then_some(found)
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

    fn whole_reads(body: &str) -> Vec<String> {
        body.match_indices("backend.get(\"")
            .filter_map(|(at, _)| {
                let tail = &body[at + "backend.get(\"".len()..];
                let close = tail.find('"')?;
                tail[close + 1..].starts_with(')').then(|| tail[..close].to_string())
            })
            .collect()
    }

    #[test]
    fn every_path_a_view_reads_whole_is_one_it_watches() {
        const WATCHED_THROUGH_A_SIBLING: &[(&str, &str)] = &[
            ("vehicle.healthAndArmingCheckReport.problemsForCurrentMode", "vehicle.healthAndArmingCheckReport.supported"),
        ];

        let seen: usize = MODULES
            .iter()
            .filter_map(|(_, source)| Some(whole_reads(source.split("#[cfg(test)]").next()?).len()))
            .sum();
        assert!(seen > 25, "the parser found only {seen} whole-path reads, so a clean result would mean nothing");

        let unwatched: Vec<String> = MODULES
            .iter()
            .filter_map(|(name, source)| {
                let body = source.split("#[cfg(test)]").next()?;
                let deps = deps_of(body)?;
                Some(whole_reads(body)
                    .into_iter()
                    .filter(|read| {
                        !WATCHED_THROUGH_A_SIBLING.iter().any(|(path, sibling)| path == read && deps.contains(*sibling))
                            && !deps.iter().any(|dep| {
                                let dep = dep.split('@').next().unwrap_or(dep);
                                dep == read || dep.starts_with(&format!("{read}.")) || read.starts_with(&format!("{dep}."))
                            })
                    })
                    .map(|read| format!("{name} reads {read} and nothing it watches covers it"))
                    .collect::<Vec<_>>())
            })
            .flatten()
            .collect();

        assert!(unwatched.is_empty(), "a view reading a subject it does not watch renders once and then holds whatever that read said: {}", unwatched.join("; "));
    }

    #[test]
    fn a_view_that_shows_a_unit_watches_the_setting_that_chooses_it() {
        const CHOSEN_BY: &[(&str, &str)] = &[
            ("Unit::horizontal(", "settings.unitsSettings.horizontalDistanceUnits"),
            ("Unit::vertical(", "settings.unitsSettings.verticalDistanceUnits"),
            ("Unit::area(", "settings.unitsSettings.areaUnits"),
            ("Unit::speed(", "settings.unitsSettings.speedUnits"),
        ];

        let unwatched: Vec<String> = MODULES
            .iter()
            .filter_map(|(name, source)| {
                let body = source.split("#[cfg(test)]").next()?;
                let deps = deps_of(body)?;
                Some(CHOSEN_BY
                    .iter()
                    .filter(|(call, dep)| body.contains(call) && !deps.contains(*dep))
                    .map(|(_, dep)| format!("{name} converts a measurement for display but does not watch {dep}, the Fact that chooses the unit; Unit::read asks units.* and never the setting, so nothing else wakes the view and switching to feet leaves the numbers metric until an unrelated dep fires"))
                    .collect::<Vec<_>>())
            })
            .flatten()
            .collect();

        assert!(unwatched.is_empty(), "{}", unwatched.join("; "));
    }

    #[test]
    fn every_field_a_view_reads_is_one_it_watches() {
        let modules = MODULES;
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
    fn a_view_that_reads_the_backend_declares_where_it_reads_from() {
        // deps_of returns None for a module with no `pub const DEPS`, and filter_map SKIPS it - so
        // a view registered with an inline `deps: &[]` was never checked at all, however much it
        // read. view.gpsRtkBase read seven settings that way. A module reading the backend and
        // declaring nothing is the case the check above cannot reach, because there is nothing to
        // compare its reads against.
        // Only modules that actually back a registered view: read.rs and router.rs read the
        // backend too and have no view to recompute, so flagging them would be a check wider than
        // the thing it measures - which is the error this whole family of guards keeps making.
        let backing: BTreeSet<&str> = include_str!("view.rs")
            .split("compute: ")
            .skip(1)
            .filter_map(|tail| tail.split("::").next())
            .collect();
        let silent: Vec<&str> = MODULES
            .iter()
            .filter(|(name, source)| {
                let body = source.split("#[cfg(test)]").next().unwrap_or("");
                backing.contains(name) && body.contains("backend.get") && !body.contains("DEPS: &[&str]")
            })
            .map(|(name, _)| *name)
            .collect();
        assert!(
            silent.is_empty(),
            "these read the backend and declare no DEPS, so the check that every read is watched skips them entirely and passes. \
             Declare the paths, or move the read behind something that does: {silent:?}"
        );
    }

    #[test]
    fn a_view_that_lists_a_runtime_sized_collection_derives_its_dependencies_at_call_time() {
        let overridden: BTreeSet<&str> = include_str!("view.rs")
            .split("pub fn deps_for")
            .nth(1)
            .and_then(|tail| tail.split("_ =>").next())
            .map(|arms| arms.split("=> ").skip(1).filter_map(|arm| arm.split("::").next()).map(str::trim).collect())
            .unwrap_or_default();

        let indexed = |body: &str| {
            body.match_indices("&format!(\"").any(|(at, _)| {
                let tail = &body[at + "&format!(\"".len()..];
                tail.find('"').is_some_and(|close| tail[..close].contains("{index}") || tail[..close].contains("{i}"))
            })
        };

        let frozen: Vec<&str> = MODULES
            .iter()
            .filter(|(name, source)| {
                let body = source.split("#[cfg(test)]").next().unwrap_or("");
                deps_of(body).is_some_and(|deps| !deps.iter().any(|dep| dep.contains('@')))
                    && body.contains(".count\")")
                    && indexed(body)
                    && !overridden.contains(name)
            })
            .map(|(name, _)| *name)
            .collect();

        assert!(
            frozen.is_empty(),
            "these read a count from the backend and then index a path with it, so the set they depend on is only known at run time - \
             a static DEPS list cannot name it, and the view is recomputed only when the count itself changes. Every member field is \
             then a snapshot from whenever a member was last added or removed. Watch the members - give the module a `pub fn deps()` \
             and route deps_for to it, the way view.battery does for packs - or watch a controller signal that fires when a member \
             changes, which is what a module with an @ dep is already doing: {frozen:?}"
        );
    }

    #[test]
    fn the_runtime_dependency_check_can_fail() {
        let overridden: BTreeSet<&str> = include_str!("view.rs")
            .split("pub fn deps_for")
            .nth(1)
            .and_then(|tail| tail.split("_ =>").next())
            .map(|arms| arms.split("=> ").skip(1).filter_map(|arm| arm.split("::").next()).map(str::trim).collect())
            .unwrap_or_default();
        assert!(overridden.contains("vehicles"), "the parser has to actually find the arms, or the check above passes because it found none");
        assert!(overridden.contains("battery"));
        assert!(!overridden.contains("preflight"), "a module with no arm must not read as overridden");
    }

    #[test]
    fn no_view_holds_a_lock_across_a_call_into_the_backend() {
        let held_across_a_backend_call = |body: &str| -> Vec<usize> {
            body.match_indices("= lock();")
                .filter_map(|(at, _)| {
                    let line_start = body[..at].rfind('\n').map_or(0, |n| n + 1);
                    let indent = body[line_start..at].len() - body[line_start..at].trim_start().len();
                    let name = body[line_start..at].trim().trim_start_matches("let ").trim_start_matches("mut ").trim();
                    let scope_start = body[at..].find('\n').map_or(body.len(), |n| at + n + 1);
                    let scope_end = body[scope_start..]
                        .match_indices('\n')
                        .map(|(n, _)| scope_start + n + 1)
                        .find(|start| {
                            let line = &body[*start..];
                            let content = line.trim_start();
                            !content.is_empty() && line.len() - content.len() < indent
                        })
                        .unwrap_or(body.len());
                    let scope = &body[scope_start..scope_end];
                    let live = match scope.find(&format!("drop({name})")) {
                        Some(cut) => &scope[..cut],
                        None => scope,
                    };
                    (live.contains("backend.") || live.contains("(backend") || live.contains("(&backend")).then(|| body[..at].matches('\n').count() + 1)
                })
                .collect()
        };

        const CAUGHT: &str = "fn v(backend: &dyn Backend) {\n    let mut g = lock();\n    g.set(read(backend));\n}\n";
        const RELEASED: &str = "fn v(backend: &dyn Backend) {\n    let mut g = lock();\n    g.tick();\n    drop(g);\n    read(backend);\n}\n";
        assert_eq!(held_across_a_backend_call(CAUGHT).len(), 1, "the scanner has to catch the shape it is about, or the clean result below means nothing");
        assert!(held_across_a_backend_call(RELEASED).is_empty(), "a guard dropped before the call is the fix, and flagging it would make the rule unfollowable");

        let offenders: Vec<String> = MODULES
            .iter()
            .filter_map(|(name, source)| {
                let body = source.split("#[cfg(test)]").next()?;
                let lines = held_across_a_backend_call(body);
                (!lines.is_empty()).then(|| format!("{name}.rs:{lines:?}"))
            })
            .collect();

        assert!(
            offenders.is_empty(),
            "a guard held across a backend call is a deadlock, not a slow path: off the Qt thread every backend call blocks until Qt services it, and Qt reaches these same views through Watcher::_notified, so the two wait on each other and every bridge read, write and invoke is dead for the life of the process. gcsposition and adsb both shipped this and no unit test could see it, because the test backend answers without blocking. {offenders:?}"
        );
    }

    #[test]
    fn a_path_a_view_builds_with_format_is_under_something_it_watches() {
        const UNVERIFIABLE: &[&str] = &["vehicle", "settings", "plan", "vehicles"];

        let prefixes = |body: &str| -> Vec<String> {
            body.match_indices("&format!(\"")
                .filter_map(|(at, _)| {
                    let tail = &body[at + "&format!(\"".len()..];
                    let brace = tail.find('{')?;
                    let quote = tail.find('"')?;
                    (brace < quote).then(|| tail[..brace].trim_end_matches('.').to_string())
                })
                .filter(|prefix| !prefix.is_empty() && prefix.contains('.') && !prefix.contains('('))
                .collect()
        };

        let under = |prefix: &str, path: &str| path == prefix || path.starts_with(&format!("{prefix}."));

        assert_eq!(
            prefixes(r#"backend.get_fields(&format!("vehicles.vehicles.{index}"), FIELDS)"#),
            vec!["vehicles.vehicles".to_string()],
            "the parser has to find the shape this test is about, or a clean result below means nothing"
        );
        assert!(
            prefixes(r#"backend.get(&format!("vehicles.count"))"#).is_empty(),
            "a format! with nothing interpolated is a literal read, not a constructed one"
        );
        assert!(
            prefixes(r#"backend.get(&format!("{root}.count"))"#).is_empty(),
            "a prefix that is entirely interpolated names no object, so there is nothing to check it against"
        );

        let unwatched: Vec<String> = MODULES
            .iter()
            .filter_map(|(name, source)| {
                let body = source.split("#[cfg(test)]").next()?;
                let deps = deps_of(body)?;
                Some(prefixes(body)
                    .into_iter()
                    .filter(|prefix| !UNVERIFIABLE.contains(&prefix.as_str()))
                    .filter(|prefix| !deps.iter().any(|dep| {
                        under(prefix, dep) || dep.split_once('@').is_some_and(|(object, _)| under(object, prefix))
                    }))
                    .map(|prefix| format!("{name} builds a read under {prefix} and watches nothing there"))
                    .collect::<Vec<_>>())
            })
            .flatten()
            .collect();

        assert!(
            unwatched.is_empty(),
            "a path built with format! is invisible to the literal sweeps - that is how vehicle.vehicleLinkManager.communicationLostEnabled was read and unwatched in two views, seen as a literal in one and not as a constructed read in the other. A prefix match is weaker than a full path match and strictly stronger than not looking. vehicle. and settings. match nearly every dep there is, so a prefix that short is reported as unverifiable rather than passed silently, and a prefix containing ( is a call expression rather than a path. An @signal dep on an ancestor counts because Watcher::_notified in src/Bridge/QGCBridgeCore.cc emits every path bound to the sending object, not only the one whose signal fired - narrowing that fan-out for performance leaves this green on views that have stopped waking: {}",
            unwatched.join("; ")
        );
    }

    #[test]
    fn the_module_list_covers_every_module_that_declares_dependencies() {
        // The list above is hand-written and include_str! needs a literal, so it cannot enumerate
        // itself. It had drifted to 18 of 55 modules - the check that every field a view reads is
        // one it watches was running over a third of the views and passing, which is the shape of
        // an instrument that answers a narrower question than the one being asked of it. It cannot
        // build its own list, but it can refuse to stay quiet about what is missing from it.
        let listed: BTreeSet<&str> = MODULES.iter().map(|(name, _)| *name).collect();
        let missing: Vec<&str> = include_str!("lib.rs")
            .lines()
            .filter_map(|line| line.trim().strip_prefix("pub mod ").and_then(|rest| rest.strip_suffix(';')))
            .filter(|name| !listed.contains(name))
            .collect();
        assert!(
            missing.is_empty(),
            "these modules are not in the list the dependency check reads, so anything they read is unchecked and the check passes anyway. \
             Add them with include_str!, whether or not they declare DEPS - a module with none is skipped harmlessly: {missing:?}"
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

    #[test]
    fn every_view_a_message_names_is_one_that_exists() {
        let registered: Vec<&str> = VIEWS.iter().map(|view| view.path).collect();
        let quoted = |line: &str| -> Vec<String> {
            line.split('"').skip(1).step_by(2).map(str::to_string).collect()
        };
        let named = |text: &str| -> Vec<String> {
            text.match_indices("view.")
                .map(|(at, _)| {
                    let tail = &text[at..];
                    let end = tail.char_indices().find(|(i, c)| *i > 4 && !c.is_ascii_alphanumeric()).map(|(i, _)| i).unwrap_or(tail.len());
                    tail[..end].to_string()
                })
                .filter(|name| name.len() > 5 && !name.ends_with(".rs"))
                .collect()
        };

        let all: Vec<(String, String)> = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|e| e == "rs"))
            .filter_map(|path| {
                let source = std::fs::read_to_string(&path).ok()?;
                let file = path.file_name()?.to_string_lossy().to_string();
                Some(source
                    .split("#[cfg(test)]")
                    .next()?
                    .lines()
                    .flat_map(quoted)
                    .flat_map(|literal| named(&literal))
                    .map(|name| (file.clone(), name))
                    .collect::<Vec<_>>())
            })
            .flatten()
            .collect();

        assert!(all.len() > 20, "only {} view names found in messages, so a clean result would mean nothing", all.len());
        let wrong: Vec<String> = all
            .iter()
            .filter(|(_, name)| !registered.contains(&name.as_str()))
            .map(|(file, name)| format!("{file} names {name}"))
            .collect();

        assert!(wrong.is_empty(), "a refusal exists to tell a caller how to call correctly, so a name in one that resolves to nothing is worse than no message: {}", wrong.join("; "));
    }

    #[test]
    fn a_view_declaring_one_argument_never_reads_a_second() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let read = |name: &str| std::fs::read_to_string(format!("{dir}/{name}.rs")).ok();
        let table = read("view").expect("view.rs is readable from its own test");
        let computes: Vec<(String, String)> = table
            .lines()
            .filter_map(|line| {
                let path = line.split_once("View { path: \"")?.1.split_once('"')?.0;
                let compute = line.rsplit_once("compute: ")?.1.split_once(' ')?.0;
                Some((path.to_string(), compute.to_string()))
            })
            .collect();
        assert!(computes.len() > 60, "parsed only {} rows of the VIEWS table, so a clean result would mean nothing", computes.len());

        let overreaching: Vec<String> = ARGUMENT_MODES
            .iter()
            .filter(|(_, mode)| !mode.contains(','))
            .filter_map(|(view, mode)| {
                let compute = &computes.iter().find(|(path, _)| path == view)?.1;
                let (module, function) = compute.split_once("::").unwrap_or(("view", compute));
                let source = read(module)?;
                let body = source.split("#[cfg(test)]").next()?.to_string();
                let at = body.find(&format!("fn {function}("))?;
                let segment = body[at..body.len().min(at + 2500)].to_string();
                (segment.contains("args.get(1)") || segment.contains("args[1]"))
                    .then(|| format!("{view} declares \"{mode}\" but {compute} reads a second argument"))
            })
            .collect();

        assert!(overreaching.is_empty(), "the declared shape is what a head builds its call from, and view::split trusts it to decide whether a comma separates arguments or belongs to the value: {}", overreaching.join("; "));
    }

    #[test]
    fn a_lone_file_path_argument_survives_the_commas_and_brackets_a_filename_may_hold() {
        let awkward = "/Users/p/Flights, 2026/log (2).tlog";
        assert_eq!(split(&format!("view.tlog({awkward})")), ("view.tlog", vec![awkward.to_string()]));
        assert_eq!(split("view.kmlFile(/a/b,c/d.kml)").1, vec!["/a/b,c/d.kml".to_string()]);

        assert_eq!(
            split("view.terrainTile(/a/b.tif,47.4,8.5)").1,
            vec!["/a/b.tif".to_string(), "47.4".to_string(), "8.5".to_string()],
            "a file path followed by declared arguments still splits, so this is not a licence to stop splitting wherever a path appears"
        );
        assert_eq!(split("view.altitudeModes(item,4)").1, vec!["item".to_string(), "4".to_string()]);
        assert_eq!(
            split("view.missionItems(geometry,fields)").1,
            vec!["geometry".to_string(), "fields".to_string()],
            "items_view reads both flags independently, so the pair has to arrive as two even though no caller sends it today"
        );

        ARGUMENT_MODES.iter().filter(|(_, mode)| *mode == "<file path>").for_each(|(view, _)| {
            assert_eq!(split(&format!("{view}(/x/y,z.dat)")).1.len(), 1, "{view} declares a single file path, so a comma in it is part of the name");
        });
    }

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
