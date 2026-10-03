use serde_json::{Value, json};

use crate::control::decode;
use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.px4Firmware", "vehicle.apmFirmware", "links.serialPorts", "links.serialPortStrings", "video.isStreamSource", "video.autoStreamConfigured", PERSISTENCE_OFF];
const PERSISTENCE_OFF: &str = "settings.appSettings.disableAllPersistence";

struct Page {
    title: &'static str,
    sections: &'static [(&'static str, &'static str)],
    shows_links: bool,
    shows_about: bool,
    shows_video_sources: bool,
    shows_packet_radio: bool,
    shows_console: bool,
    shows_ntrip: bool,
    shows_px4_logs: bool,
}

const PAGES: &[Page] = &[
    Page { title: "General", sections: &[("Application", "appSettings"), ("Units", "unitsSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Fly View", sections: &[("Fly View", "flyViewSettings"), ("MAVLink Actions", "mavlinkActionsSettings"), ("Battery Indicator", "batteryIndicatorSettings"), ("Gimbal Controller", "gimbalControllerSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Plan View", sections: &[("Plan View", "planViewSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Video", sections: &[("Video", "videoSettings")], shows_links: false, shows_about: false, shows_video_sources: true, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Maps", sections: &[("Maps", "mapsSettings"), ("Flight Map", "flightMapSettings"), ("Map Providers", MAP_PROVIDERS), ("Offline Maps", "offlineMapsSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Connections", sections: &[("Auto Connect", "autoConnectSettings")], shows_links: true, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "MAVLink", sections: &[("MAVLink", "mavlinkSettings"), ("APM Stream Rates", "apmMavlinkStreamRateSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Flight Modes", sections: &[("Flight Modes", "flightModeSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "ADSB Server", sections: &[("ADSB Server", "adsbVehicleManagerSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Packet Radio", sections: &[("Packet Radio", "packetRadioSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: true, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Remote ID", sections: &[("Remote ID", "remoteIDSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "PX4 Log Transfer", sections: &[], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: true },
    Page { title: "NTRIP / RTK", sections: &[("NTRIP", "ntripSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: true, shows_px4_logs: false },
    Page { title: "RTK GPS", sections: &[("RTK GPS", "rtkSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Firmware Upgrade", sections: &[("Firmware Upgrade", "firmwareUpgradeSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "3D Viewer", sections: &[("3D Viewer", "viewer3DSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "About", sections: &[], shows_links: false, shows_about: true, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "Console", sections: &[], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: true, shows_ntrip: false, shows_px4_logs: false },
    Page { title: "App Logging", sections: &[("Save To Disk", "logManagerSettings"), ("Log Viewer", APP_LOG_VIEWER)], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false, shows_console: false, shows_ntrip: false, shows_px4_logs: false },
];

// deviceName is drawn by the Packet Radio page's own block as a picker over the adapters the
// radio reports. Left in the generic list it renders a second control writing the same fact - a
// free-text field beside the picker, where a name that is not an adapter gets no feedback at all.
// Same shape as extraVideoSources: when a bespoke block owns a fact, the fact leaves the list.
const HIDDEN: &[&str] = &[
    "androidUsePosixSerial",
    "keepSceneAlive",
    "qLocaleLanguage",
    "overlayGlassFrost",
    "batteryPercentRemainingAnnounce",
    "loginAirLink",
    "passAirLink",
    "videoSavePath",
    "showRecControl",
    "preferredFirmwareClass",
    "preferredVehicleClass",
    "audioVolume",
    "uiScalePercent",
    "gstDebugLevel",
    "operatorIDType",
    "clearSettingsNextBoot",
    "coreLinks",
    "detectionsHttpPort",
    "favoriteParameters",
    "activeVideoSource",
    "rtpJitterLatencyMs",
    "rtspAutoReconnect",
    "forceCpuVideoPath",
    "videoConversionElement",
    "disablePixelAspectRatio",
    "instrumentQmlFile2",
    "enableAutomaticMissionPopups",
    "px4HiddenFlightModesMultiRotor",
    "px4HiddenFlightModesFixedWing",
    "px4HiddenFlightModesVTOL",
    "px4HiddenFlightModesRoverBoat",
    "px4HiddenFlightModesSub",
    "px4HiddenFlightModesAirship",
    "apmHiddenFlightModesMultiRotor",
    "apmHiddenFlightModesFixedWing",
    "apmHiddenFlightModesVTOL",
    "apmHiddenFlightModesRoverBoat",
    "apmHiddenFlightModesSub",
    "apmHiddenFlightModesAirship",
    "firstRunPromptIdsShown",
    "deviceName",
    "ntripServerConnectEnabled",
    "flyViewActionsFile",
    "joystickActionsFile",
    "autoConnectZeroConf",
    "udpListenPort",
    "udpTargetHostIP",
    "udpTargetHostPort",
    "showMissionItemStatus",
    "showGimbalOnlyWhenSet",
];
const DESKTOP_ONLY: &[(&str, &str)] = &[("rcControls", "on-screen RC controls"), ("extraVideoSources", "additional cameras")];

fn choices_json(fact: &Value, labels: Vec<String>, raws: Vec<Value>) -> Value {
    let current = fact.get("value").cloned().unwrap_or(Value::Null);
    let index = raws.iter().position(|raw| raw.to_string() == current.to_string() || raw.as_str().is_some_and(|r| current.as_str() == Some(r)));
    let mut chosen = fact.clone();
    chosen["enumStrings"] = json!(labels);
    chosen["enumValues"] = json!(raws);
    chosen["enumIndex"] = json!(index.map_or(-1, |i| i as i64));
    chosen
}

fn with_choices(backend: &dyn Backend, fact: &Value) -> Value {
    match fact.get("name").and_then(Value::as_str) {
        Some("autoConnectNmeaPort") => {
            let ports = object(&backend.get_fields("links", "serialPorts,serialPortStrings"));
            let listed = crate::links::serial_ports(ports.get("serialPorts"), ports.get("serialPortStrings"));
            let current = fact.get("value").and_then(Value::as_str).filter(|v| !v.is_empty()).map(str::to_string);
            let extra = current.filter(|c| !listed.iter().any(|p| p["port"].as_str() == Some(c.as_str())));
            let labels = listed.iter().map(|p| p["label"].as_str().unwrap_or("").to_string()).chain(extra.clone()).collect();
            let raws = listed.iter().map(|p| p["port"].clone()).chain(extra.map(Value::from)).collect();
            choices_json(fact, labels, raws)
        }
        Some("autoConnectNmeaBaud") => {
            let listed = crate::links::serial_baud_rates(backend);
            let current = fact.get("value").and_then(Value::as_i64).filter(|rate| *rate > 0 && !listed.contains(rate));
            let rates: Vec<i64> = listed.into_iter().chain(current).collect();
            choices_json(fact, rates.iter().map(i64::to_string).collect(), rates.iter().map(|r| json!(r)).collect())
        }
        Some("appFontPointSize") => ui_scaling(fact),
        _ => fact.clone(),
    }
}

const SCALE_PERCENTS: [u32; 8] = [80, 90, 100, 110, 125, 150, 175, 200];
const PLATFORM_FONT_POINT_SIZE: f64 = 14.0;

fn point_size_for(percent: u32) -> i64 {
    (PLATFORM_FONT_POINT_SIZE * f64::from(percent) / 100.0).round() as i64
}

pub fn scaled_point_size(index: usize) -> Option<i64> {
    SCALE_PERCENTS.get(index).map(|percent| point_size_for(*percent))
}

pub fn nearest_scale_index(point_size: f64) -> usize {
    let current = if point_size > 0.0 { point_size / PLATFORM_FONT_POINT_SIZE * 100.0 } else { 100.0 };
    SCALE_PERCENTS.iter().enumerate().min_by(|(_, a), (_, b)| (f64::from(**a) - current).abs().total_cmp(&(f64::from(**b) - current).abs())).map_or(2, |(i, _)| i)
}

fn ui_scaling(fact: &Value) -> Value {
    let mut chosen = choices_json(fact, SCALE_PERCENTS.iter().map(|p| format!("{p}%")).collect(), SCALE_PERCENTS.iter().map(|p| json!(point_size_for(*p))).collect());
    chosen["enumIndex"] = json!(nearest_scale_index(fact.get("value").and_then(Value::as_f64).unwrap_or(0.0)));
    chosen["label"] = json!("UI Scaling");
    chosen["units"] = json!("");
    chosen
}

const CHECKLIST_OFF: &str = "Has no effect while the preflight checklist is off.";

const NTRIP_ACTIVE: &str = "Disconnect from the NTRIP server to change this.";
const NTRIP_NO_TLS: &str = "Only applies with TLS encryption on.";
const NTRIP_NO_FORWARD: &str = "Has no effect while UDP forwarding is off.";
const RTCM_NO_INPUT: &str = "Has no effect while UDP RTCM input is off.";
const FORWARDING_OFF: &str = "Has no effect while MAVLink forwarding is off.";
const TELEMETRY_SAVE_OFF: &str = "Has no effect while saving telemetry logs is off.";
const STORAGE_LIMIT_OFF: &str = "Has no effect while the storage limit is off.";
const STREAMS_FROM_VEHICLE: &str = "Stream rates are controlled by the vehicle.";
const ADSB_SERVER_OFF: &str = "Has no effect while the ADS-B server connection is off.";
const BASIC_ID_OFF: &str = "Has no effect while Basic ID broadcast is off.";
const SELF_ID_OFF: &str = "Has no effect while Self ID broadcast is off.";

const GATED: &[(&str, &str, bool, &str)] = &[
    ("diskLoggingMaxFileSizeMB", "diskLoggingEnabled", true, DISK_LOGGING_OFF),
    ("diskLoggingMaxBackupFiles", "diskLoggingEnabled", true, DISK_LOGGING_OFF),
    ("enforceChecklist", "useChecklist", true, CHECKLIST_OFF),

    ("virtualJoystickAutoCenterThrottle", "virtualJoystick", true, VIRTUAL_JOYSTICK_OFF),
    ("virtualJoystickLeftHandedMode", "virtualJoystick", true, VIRTUAL_JOYSTICK_OFF),
    ("ntripServerHostAddress", "ntripServerConnectEnabled", false, NTRIP_ACTIVE),
    ("ntripServerPort", "ntripServerConnectEnabled", false, NTRIP_ACTIVE),
    ("ntripUsername", "ntripServerConnectEnabled", false, NTRIP_ACTIVE),
    ("ntripPassword", "ntripServerConnectEnabled", false, NTRIP_ACTIVE),
    ("ntripMountpoint", "ntripServerConnectEnabled", false, NTRIP_ACTIVE),
    ("ntripUseTls", "ntripServerConnectEnabled", false, NTRIP_ACTIVE),
    ("ntripAllowSelfSignedCerts", "ntripServerConnectEnabled", false, NTRIP_ACTIVE),
    ("ntripAllowSelfSignedCerts", "ntripUseTls", true, NTRIP_NO_TLS),
    ("ntripUdpTargetAddress", "ntripUdpForwardEnabled", true, NTRIP_NO_FORWARD),
    ("ntripUdpTargetPort", "ntripUdpForwardEnabled", true, NTRIP_NO_FORWARD),
    ("rtcmUdpInputPort", "rtcmUdpInputEnabled", true, RTCM_NO_INPUT),
    ("rtcmUdpValidate", "rtcmUdpInputEnabled", true, RTCM_NO_INPUT),
    ("forwardMavlinkHostName", "forwardMavlink", true, FORWARDING_OFF),
    ("telemetrySaveNotArmed", "telemetrySave", true, TELEMETRY_SAVE_OFF),
    ("adsbServerHostAddress", "adsbServerConnectEnabled", true, ADSB_SERVER_OFF),
    ("adsbServerPort", "adsbServerConnectEnabled", true, ADSB_SERVER_OFF),
    ("maxVideoSize", "enableStorageLimit", true, STORAGE_LIMIT_OFF),
    ("basicIDType", "sendBasicID", true, BASIC_ID_OFF),
    ("basicIDUaType", "sendBasicID", true, BASIC_ID_OFF),
    ("basicID", "sendBasicID", true, BASIC_ID_OFF),
    ("selfIDType", "sendSelfID", true, SELF_ID_OFF),
    ("selfIDFree", "sendSelfID", true, SELF_ID_OFF),
    ("selfIDExtended", "sendSelfID", true, SELF_ID_OFF),
    ("streamRateRawSensors", "apmStartMavlinkStreams", true, STREAMS_FROM_VEHICLE),
    ("streamRateExtendedStatus", "apmStartMavlinkStreams", true, STREAMS_FROM_VEHICLE),
    ("streamRateRCChannels", "apmStartMavlinkStreams", true, STREAMS_FROM_VEHICLE),
    ("streamRatePosition", "apmStartMavlinkStreams", true, STREAMS_FROM_VEHICLE),
    ("streamRateExtra1", "apmStartMavlinkStreams", true, STREAMS_FROM_VEHICLE),
    ("streamRateExtra2", "apmStartMavlinkStreams", true, STREAMS_FROM_VEHICLE),
    ("streamRateExtra3", "apmStartMavlinkStreams", true, STREAMS_FROM_VEHICLE),
];

const INVERTED: &[(&str, &str)] = &[("apmStartMavlinkStreams", "Controlled by Vehicle"), ("androidDontSaveToSDCard", "Save application data to SD Card")];

const QML_LABELS: &[(&str, &str, &str)] = &[
    ("appSettings", "audioMuted", "Mute all audio output"),
    ("appSettings", "enableMultiVehiclePanel", "Show multi-vehicle panel"),
    ("appSettings", "virtualJoystick", "Enabled"),
    ("appSettings", "virtualJoystickLeftHandedMode", "Left-handed mode (swap sticks)"),
    ("flyViewSettings", "updateHomePosition", "Update return to home position based on device location"),
    ("flyViewSettings", "forwardFlightGoToLocationLoiterRad", "Loiter Radius in Forward Flight Guided Mode"),
    ("flyViewSettings", "goToLocationRequiresConfirmInGuided", "Confirm before Go To Location in guided mode"),
    ("viewer3DSettings", "enabled", "Enabled"),
    ("viewer3DSettings", "osmFilePath", "3D Map File"),
    ("viewer3DSettings", "buildingLevelHeight", "Average Building Level Height"),
    ("viewer3DSettings", "altitudeBias", "Vehicles Altitude Bias"),
    ("autoConnectSettings", "autoConnectPixhawk", "Pixhawk"),
    ("autoConnectSettings", "autoConnectSiKRadio", "SiK Radio"),
    ("autoConnectSettings", "autoConnectLibrePilot", "LibrePilot"),
    ("autoConnectSettings", "autoConnectUDP", "UDP"),
    ("autoConnectSettings", "autoConnectRTKGPS", "RTK"),
    ("autoConnectSettings", "nmeaSource", "Source"),
    ("autoConnectSettings", "autoConnectNmeaPort", "Device"),
    ("autoConnectSettings", "autoConnectNmeaBaud", "Baudrate"),
    ("autoConnectSettings", "nmeaUdpPort", "NMEA stream UDP port"),
    ("videoSettings", "multiViewEnabled", "Show all cameras"),
    ("videoSettings", "rtspTimeout", "Connection Timeout"),
    ("videoSettings", "disableWhenDisarmed", "Stop recording when disarmed"),
    ("videoSettings", "lowLatencyMode", "Low latency mode"),
    ("videoSettings", "forceVideoDecoder", "Video Decode Priority"),
    ("videoSettings", "recordingFormat", "File Format"),
    ("videoSettings", "enableStorageLimit", "Delete old recordings automatically"),
    ("videoSettings", "maxVideoSize", "Storage Limit"),
    ("gimbalControllerSettings", "enableOnScreenControl", "Enabled"),
    ("gimbalControllerSettings", "clickAndDrag", "Click and drag"),
    ("gimbalControllerSettings", "cameraHFov", "Horizontal FOV"),
    ("gimbalControllerSettings", "cameraVFov", "Vertical FOV"),
    ("gimbalControllerSettings", "cameraSlideSpeed", "Max speed"),
    ("gimbalControllerSettings", "zoomMaxSpeed", "Max speed (min zoom)"),
    ("gimbalControllerSettings", "zoomMinSpeed", "Min speed (max zoom)"),
    ("gimbalControllerSettings", "joystickButtonsSpeed", "Joystick buttons speed:"),
    ("gimbalControllerSettings", "showAzimuthIndicatorOnMap", "Show gimbal Azimuth indicator in map"),
    ("gimbalControllerSettings", "toolbarIndicatorShowAzimuth", "Use Azimuth instead of local yaw on top toolbar indicator"),
    ("gimbalControllerSettings", "toolbarIndicatorShowAcquireReleaseControl", "Show Acquire/Release control button"),
];

fn qml_labelled(group: &str, control: Value) -> Value {
    let name = control.get("name").and_then(Value::as_str).unwrap_or_default();
    match QML_LABELS.iter().find(|(in_group, labelled, _)| *in_group == group && *labelled == name) {
        Some((_, _, label)) => {
            let mut control = control;
            control["label"] = json!(label);
            control["shortLabel"] = json!(label);
            control
        }
        None => control,
    }
}

fn inverted(control: Value) -> Value {
    let name = control.get("name").and_then(Value::as_str).unwrap_or_default();
    match INVERTED.iter().find(|(inverted, _)| *inverted == name) {
        Some((_, label)) => {
            let mut control = control;
            control["label"] = json!(label);
            control["shortLabel"] = json!(label);
            control["inverted"] = json!(true);
            control
        }
        None => control,
    }
}

const MOBILE: bool = cfg!(any(target_os = "android", target_os = "ios"));
const LOGGING_ROWS: [&str; 3] = ["telemetrySave", "telemetrySaveNotArmed", "saveCsvTelemetry"];
const VIRTUAL_JOYSTICK_OFF: &str = "Has no effect while on-screen sticks are off.";

const DISK_LOGGING_OFF: &str = "Writing the log to disk is off";

const GATED_FROM: &[(&str, &str)] = &[("apmMavlinkStreamRateSettings", "mavlinkSettings")];

// QGC distinguishes the two, and which binding a page uses is what decides this. FlyViewSettings
// binds the checklist row's `enabled`, so that control is real but inert and says why. The RTK
// page binds `visible` on both groups, because GPSIndicatorPage.qml:113-120 is a Survey-In /
// Specify position radio pair - the two sets are alternatives under a mode selector, not a switch
// with dependents. Four greyed latitude boxes under a Survey-In selection would be a control
// disabled with nothing on screen saying what it is disabled FOR.
const MANUFACTURER_ALL: i64 = 0;
const MANUFACTURER_ROWS: &[(&str, &[i64])] = &[("surveyInAccuracyLimit", &[4]), ("surveyInMinObservationDuration", &[4, 3, 1]), ("fixedBasePositionAccuracy", &[4])];

pub fn shown_for_manufacturer(name: &str, manufacturer: i64) -> bool {
    manufacturer == MANUFACTURER_ALL || MANUFACTURER_ROWS.iter().find(|(row, _)| *row == name).is_none_or(|(_, makers)| makers.contains(&manufacturer))
}

const REGION_EU: i64 = 1;
const LOCATION_FIXED: i64 = 2;
const CLASSIFICATION_EU: i64 = 1;

const SHOWN_WHEN_VALUE: &[(&str, &str, i64, bool)] = &[
    ("operatorIDEU", "region", REGION_EU, true),
    ("operatorIDFAA", "region", REGION_EU, false),
    ("classificationType", "region", REGION_EU, true),
    ("categoryEU", "region", REGION_EU, true),
    ("classEU", "region", REGION_EU, true),
    ("autoConnectNmeaPort", "nmeaSource", NMEA_SOURCE_SERIAL, true),
    ("autoConnectNmeaBaud", "nmeaSource", NMEA_SOURCE_SERIAL, true),
    ("nmeaUdpPort", "nmeaSource", NMEA_SOURCE_UDP, true),
];

const NMEA_SOURCE_UDP: i64 = 1;
const NMEA_SOURCE_SERIAL: i64 = 2;
const VIEWER_3D_OFF: &str = "Has no effect while the 3D view is off.";

const GATED_IN_GROUP: &[(&str, &str, &str, &str)] = &[
    ("viewer3DSettings", "osmFilePath", "enabled", VIEWER_3D_OFF),
    ("viewer3DSettings", "buildingLevelHeight", "enabled", VIEWER_3D_OFF),
    ("viewer3DSettings", "altitudeBias", "enabled", VIEWER_3D_OFF),
];

const OPERATOR_ID_EU: &str = "The operator ID is always broadcast in the EU.";
const LOCATION_NOT_FIXED: &str = "Has no effect unless the location type is Fixed.";
const CLASSIFICATION_NOT_EU: &str = "Has no effect unless the classification type is EU.";

const GATED_VALUE: &[(&str, &str, i64, bool, &str)] = &[
    ("sendOperatorID", "region", REGION_EU, false, OPERATOR_ID_EU),
    ("latitudeFixed", "locationType", LOCATION_FIXED, true, LOCATION_NOT_FIXED),
    ("longitudeFixed", "locationType", LOCATION_FIXED, true, LOCATION_NOT_FIXED),
    ("altitudeFixed", "locationType", LOCATION_FIXED, true, LOCATION_NOT_FIXED),
    ("categoryEU", "classificationType", CLASSIFICATION_EU, true, CLASSIFICATION_NOT_EU),
    ("classEU", "classificationType", CLASSIFICATION_EU, true, CLASSIFICATION_NOT_EU),
];

const HIDDEN_WHEN: &[(&str, &str, bool)] = &[
    ("surveyInAccuracyLimit", "useFixedBasePosition", true),
    ("surveyInMinObservationDuration", "useFixedBasePosition", true),
    ("fixedBasePositionLatitude", "useFixedBasePosition", false),
    ("fixedBasePositionLongitude", "useFixedBasePosition", false),
    ("fixedBasePositionAltitude", "useFixedBasePosition", false),
    ("fixedBasePositionAccuracy", "useFixedBasePosition", false),
    ("clickAndDrag", "enableOnScreenControl", false),
    ("cameraHFov", "enableOnScreenControl", false),
    ("cameraVFov", "enableOnScreenControl", false),
    ("cameraSlideSpeed", "enableOnScreenControl", false),
    ("cameraHFov", "clickAndDrag", true),
    ("cameraVFov", "clickAndDrag", true),
    ("cameraSlideSpeed", "clickAndDrag", false),
];

const SUBSECTIONS: &[(&str, &[(&str, &[&str])])] = &[
    (MAP_PROVIDERS, &[
        ("Tokens", &["mapboxToken", "esriToken", "vworldToken", "tiandituToken", "openaipToken"]),
        ("Mapbox Login", &["mapboxAccount", "mapboxStyle"]),
        ("Custom Map URL", &["customURL"]),
    ]),
    ("gimbalControllerSettings", &[
        ("On-Screen Control", &["enableOnScreenControl", "clickAndDrag", "cameraHFov", "cameraVFov", "cameraSlideSpeed"]),
        ("Zoom speed", &["zoomMaxSpeed", "zoomMinSpeed"]),
        ("", &["joystickButtonsSpeed", "showAzimuthIndicatorOnMap", "toolbarIndicatorShowAzimuth", "toolbarIndicatorShowAcquireReleaseControl"]),
    ]),
    ("autoConnectSettings", &[
        ("", &["autoConnectPixhawk", "autoConnectSiKRadio", "autoConnectLibrePilot", "autoConnectUDP", "autoConnectRTKGPS"]),
        ("NMEA GPS", &["nmeaSource", "autoConnectNmeaPort", "autoConnectNmeaBaud", "nmeaUdpPort"]),
    ]),
    ("remoteIDSettings", &[
        ("Region", &["region"]),
        ("Basic ID", &["sendBasicID", "basicIDType", "basicIDUaType", "basicID"]),
        ("Operator ID", &["sendOperatorID", "operatorIDType", "operatorIDEU", "operatorIDFAA"]),
        ("Self ID", &["sendSelfID", "selfIDType", "selfIDFree", "selfIDExtended", "selfIDEmergency"]),
        ("Ground Station Location", &["locationType", "latitudeFixed", "longitudeFixed", "altitudeFixed"]),
        ("EU Vehicle Info", &["classificationType", "categoryEU", "classEU"]),
    ]),
    ("mavlinkSettings", &[
        ("Telemetry logs", &["telemetrySave", "telemetrySaveNotArmed", "saveCsvTelemetry"]),
        ("Ground Station", &["gcsMavlinkSystemID", "sendGCSHeartbeat", "noInitialDownloadWhenFlying"]),
        ("MAVLink Forwarding", &["forwardMavlink", "forwardMavlinkHostName", "forwardMavlinkAPMSupportHostName"]),
        ("Stream Rates (ArduPilot Only)", &["apmStartMavlinkStreams"]),
    ]),
    ("ntripSettings", &[
        ("Server", &["ntripServerHostAddress", "ntripServerPort", "ntripUsername", "ntripPassword", "ntripUseTls", "ntripAllowSelfSignedCerts"]),
        ("Mountpoint", &["ntripMountpoint"]),
        ("GGA Position Reporting", &["ntripGgaPositionSource", "ntripGgaIntervalSec"]),
        ("Options", &["ntripWhitelist"]),
        ("UDP Forwarding", &["ntripUdpForwardEnabled", "ntripUdpTargetAddress", "ntripUdpTargetPort"]),
        ("UDP RTCM Input", &["rtcmUdpInputEnabled", "rtcmUdpInputPort", "rtcmUdpValidate"]),
    ]),
    ("appSettings", &[
        ("Appearance", &["indoorPalette", "appFontPointSize", "overlayGlassFrost", "qLocaleLanguage"]),
        ("Sound", &["audioMuted", "batteryPercentRemainingAnnounce"]),
        ("Preflight checklist", &["useChecklist", "enforceChecklist"]),
        ("On-screen sticks", &["virtualJoystick", "virtualJoystickAutoCenterThrottle", "virtualJoystickLeftHandedMode"]),
        ("Planning defaults", &["defaultMissionItemAltitude", "offlineEditingFirmwareClass", "offlineEditingVehicleClass", "offlineEditingCruiseSpeed", "offlineEditingHoverSpeed", "offlineEditingAscentSpeed", "offlineEditingDescentSpeed"]),
        ("AirLink", &["loginAirLink", "passAirLink"]),
        ("Ground station position", &["followTarget"]),
        ("Multiple vehicles", &["enableMultiVehiclePanel"]),
        ("Files", &["savePath", "androidDontSaveToSDCard", "disableAllPersistence"]),
    ]),
    ("videoSettings", &[
        ("Cameras", &["videoSource", "primaryCameraName", "multiViewEnabled"]),
        ("Stream", &["udpUrl", "rtspUrl", "tcpUrl", "whepUrl", "rtspTimeout", "streamEnabled", "disableWhenDisarmed", "lowLatencyMode", "forceVideoDecoder"]),
        ("Display", &["videoFit", "aspectRatio", "gridLines", "showRecControl"]),
        ("Local Video Storage", &["videoSavePath", "recordingFormat", "enableStorageLimit", "maxVideoSize"]),
    ]),
    ("viewer3DSettings", &[
        ("General", &["enabled", "mapProvider"]),
        ("Data", &["osmFilePath", "buildingLevelHeight", "altitudeBias"]),
    ]),
    ("flyViewSettings", &[
        ("Guided Commands", &["guidedMinimumAltitude", "guidedMaximumAltitude", "maxGoToLocationDistance", "forwardFlightGoToLocationLoiterRad", "goToLocationRequiresConfirmInGuided", "updateHomePosition"]),
        ("Map and compass", &["keepMapCenteredOnVehicle", "showAdditionalIndicatorsCompass", "lockNoseUpCompass", "showObstacleDistanceOverlay"]),
        ("On-screen controls", &["showPhotoVideoControl", "showSimpleCameraControl", "showLogReplayStatusBar"]),
        ("Camera & Gimbal Control", &["gimbalTiltChannel", "gimbalPanChannel", "cameraZoomChannel", "cameraLightChannel", "cameraRecordChannel"]),
        ("Sharing control", &["requestControlAllowTakeover", "requestControlTimeout"]),
    ]),
];

const HELP_LINKS: [(&str, &str); 4] = [
    ("QGroundControl User Guide", "https://docs.qgroundcontrol.com"),
    ("PX4 Users Discussion Forum", "http://discuss.px4.io/c/qgroundcontrol"),
    ("ArduPilot Users Discussion Forum", "https://discuss.ardupilot.org/c/ground-control-software/qgroundcontrol"),
    ("QGroundControl Discord Channel", "https://discord.com/channels/1022170275984457759/1022185820683255908"),
];

fn link_host(url: &str) -> &str {
    url.split_once("://").map_or(url, |(_, rest)| rest).split('/').next().unwrap_or(url)
}

const SETTINGS_PAGES_MODEL: &str = include_str!("../../src/AppSettings/SettingsPagesModel.qml");
const QGC_PAGE_NAMES: &[(&str, &[&str])] = &[("3D Viewer", &["3D View"]), ("MAVLink", &["Telemetry"]), ("About", &["Help"])];

fn qstr<'a>(element: &'a str, field: &str) -> Option<&'a str> {
    let opening = format!("{field}: qsTr(\"");
    element.lines().map(str::trim_start).find_map(|line| line.strip_prefix(opening.as_str())).and_then(|rest| rest.strip_suffix("\")"))
}

pub fn page_keywords(title: &str) -> String {
    let names = QGC_PAGE_NAMES.iter().find(|(ours, _)| *ours == title).map_or_else(|| vec![title], |(_, theirs)| theirs.to_vec());
    SETTINGS_PAGES_MODEL
        .split("ListElement {")
        .skip(1)
        .filter(|element| qstr(element, "name").is_some_and(|name| names.contains(&name)))
        .flat_map(|element| [qstr(element, "summary"), qstr(element, "keywords")])
        .flatten()
        .collect::<Vec<_>>()
        .join(", ")
        .to_lowercase()
}

const SETTINGS_UI_PAGES: [&str; 3] = [
    include_str!("../../src/AppSettings/pages/Logging.SettingsUI.json"),
    include_str!("../../src/AppSettings/pages/NTRIP.SettingsUI.json"),
    include_str!("../../src/AppSettings/pages/Viewer3D.SettingsUI.json"),
];

fn fact_keywords(group: &str, name: &str) -> String {
    static KEYWORDS: std::sync::OnceLock<std::collections::BTreeMap<(String, String), Vec<String>>> = std::sync::OnceLock::new();
    let from_facts = || {
        crate::settingsstore::objects_json()
            .filter_map(|(object, json)| Some((object, serde_json::from_str::<Value>(json).ok()?)))
            .flat_map(|(object, json)| {
                json["QGC.MetaData.Facts"].as_array().cloned().unwrap_or_default().into_iter().filter_map(move |fact| {
                    Some(((object.to_string(), fact["name"].as_str()?.to_string()), vec![fact["keywords"].as_str()?.to_lowercase()]))
                })
            })
            .collect::<Vec<_>>()
    };
    let from_pages = || {
        SETTINGS_UI_PAGES
            .iter()
            .filter_map(|page| serde_json::from_str::<Value>(page).ok())
            .flat_map(|page| page["groups"].as_array().cloned().unwrap_or_default())
            .flat_map(|ui_group| {
                let words: Vec<String> = std::iter::once(&ui_group["heading"]).chain(ui_group["keywords"].as_array().into_iter().flatten()).filter_map(Value::as_str).map(str::to_lowercase).collect();
                ui_group["controls"].as_array().cloned().unwrap_or_default().into_iter().filter_map(move |control| {
                    let (object, fact) = control["setting"].as_str()?.split_once('.')?;
                    Some(((object.to_string(), fact.to_string()), words.clone()))
                })
            })
            .collect::<Vec<_>>()
    };
    KEYWORDS
        .get_or_init(|| {
            from_facts().into_iter().chain(from_pages()).fold(std::collections::BTreeMap::new(), |mut all, (key, words)| {
                all.entry(key).or_insert_with(Vec::new).extend(words);
                all
            })
        })
        .get(&(group.to_string(), name.to_string()))
        .map(|words| words.join(", "))
        .unwrap_or_default()
}

fn page_json(page: &Page, with_controls: Option<&dyn Backend>) -> Value {
    json!({
        "helpLinks": match page.shows_about {
            true => HELP_LINKS.iter().map(|(name, url)| json!({ "name": name, "url": url, "host": link_host(url) })).collect::<Vec<_>>(),
            false => Vec::new(),
        },
        "title": page.title,
        "keywords": page_keywords(page.title),
        "showsLinks": page.shows_links,
        "showsAbout": page.shows_about,
        "showsVideoSources": page.shows_video_sources,
        "showsPacketRadio": page.shows_packet_radio,
        "showsConsole": page.shows_console,
        "showsNtrip": page.shows_ntrip,
        "showsPx4Logs": page.shows_px4_logs,
        "sections": page.sections.iter().filter(|(_, group)| section_applies(group, with_controls)).map(|(title, group)| section_json(title, group, with_controls)).collect::<Vec<_>>(),
    })
}

pub fn apm_streams_apply(connected: bool, apm_firmware: bool) -> bool {
    !connected || apm_firmware
}

fn section_applies(group: &str, backend: Option<&dyn Backend>) -> bool {
    match (group, backend) {
        ("apmMavlinkStreamRateSettings", Some(backend)) => apm_streams_apply(
            flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable"),
            flag(&object(&backend.get_fields("vehicle", "apmFirmware")), "apmFirmware"),
        ),
        _ => true,
    }
}

pub const AUTO_CONFIGURED: &str = "Configured automatically over MAVLink.";
const PRIMARY_CAMERA: [&str; 6] = ["videoSource", "primaryCameraName", "udpUrl", "rtspUrl", "tcpUrl", "whepUrl"];

fn auto_locked(control: Value) -> Value {
    match control.get("name").and_then(Value::as_str).is_some_and(|name| PRIMARY_CAMERA.contains(&name)) {
        true => Value::Object(control.as_object().cloned().unwrap_or_default().into_iter().chain([("enabled".to_string(), json!(false)), ("disabledReason".to_string(), json!(AUTO_CONFIGURED))]).collect()),
        false => control,
    }
}

const STREAM_ONLY: [&str; 4] = ["rtspTimeout", "disableWhenDisarmed", "lowLatencyMode", "aspectRatio"];

pub fn video_row_shown(name: &str, source: &str, stream_source: bool, auto_configured: bool) -> bool {
    let url_fact = crate::settingsstore::URL_SOURCES.iter().find(|(served, _)| *served == source).map(|(_, fact)| *fact);
    match name {
        "udpUrl" | "rtspUrl" | "tcpUrl" | "whepUrl" => url_fact == Some(name),
        "forceVideoDecoder" => stream_source,
        _ if STREAM_ONLY.contains(&name) => stream_source && !auto_configured,
        _ => true,
    }
}

const MAP_PROVIDERS: &str = "appSettings#mapProviders";
const APP_LOG_VIEWER: &str = "appSettings#logViewer";
const APP_LOG_VIEWER_ROWS: [&str; 1] = ["showAppLogTimestampAsElapsedTime"];
const MAP_PROVIDER_ROWS: [&str; 8] = ["mapboxToken", "esriToken", "vworldToken", "tiandituToken", "openaipToken", "mapboxAccount", "mapboxStyle", "customURL"];

pub fn slice_shows(slice: &str, name: &str) -> bool {
    match slice {
        MAP_PROVIDERS => MAP_PROVIDER_ROWS.contains(&name),
        APP_LOG_VIEWER => APP_LOG_VIEWER_ROWS.contains(&name),
        "appSettings" => !MAP_PROVIDER_ROWS.contains(&name) && !APP_LOG_VIEWER_ROWS.contains(&name),
        _ => true,
    }
}

fn section_json(title: &str, slice: &str, backend: Option<&dyn Backend>) -> Value {
    let group = slice.split_once('#').map_or(slice, |(group, _)| group);
    let path = format!("settings.{group}");
    let Some(backend) = backend else { return json!({ "title": title, "group": group, "path": path }) };
    let facts: Vec<Value> = object(&backend.get(&path)).get("facts").and_then(Value::as_array).cloned().unwrap_or_default();
    let video = (group == "videoSettings").then(|| {
        let manager = object(&backend.get_fields("video", "isStreamSource,autoStreamConfigured"));
        let source = facts.iter().find(|f| f.get("name").and_then(Value::as_str) == Some("videoSource")).and_then(|f| f.get("value")).and_then(Value::as_str).unwrap_or_default().to_string();
        (source, flag(&manager, "isStreamSource"), flag(&manager, "autoStreamConfigured"))
    });
    let persistence_off = group == "mavlinkSettings" && object(&backend.get(PERSISTENCE_OFF)).get("value").and_then(Value::as_bool) == Some(true);
    let apm_streams = group != "mavlinkSettings" || section_applies("apmMavlinkStreamRateSettings", Some(backend));
    let shown: Vec<Value> = facts
        .iter()
        .filter(|f| f.get("visible").and_then(Value::as_bool) != Some(false))
        .filter(|f| slice_shows(slice, f.get("name").and_then(Value::as_str).unwrap_or_default()))
        .filter(|f| !(persistence_off && f.get("name").and_then(Value::as_str).is_some_and(|n| LOGGING_ROWS.contains(&n))))
        .filter(|f| apm_streams || f.get("name").and_then(Value::as_str) != Some("apmStartMavlinkStreams"))
        .filter(|f| video.as_ref().is_none_or(|(source, stream, auto)| video_row_shown(f.get("name").and_then(Value::as_str).unwrap_or_default(), source, *stream, *auto)))
        .filter(|f| {
            let named = f.get("name").and_then(Value::as_str).unwrap_or_default();
            !HIDDEN_WHEN.iter().any(|(hidden, requires, when)| {
                *hidden == named && facts.iter().find(|other| other.get("name").and_then(Value::as_str) == Some(requires)).and_then(|other| other.get("value")).and_then(crate::read::switch_on) == Some(*when)
            })
        })
        .filter(|f| {
            let named = f.get("name").and_then(Value::as_str).unwrap_or_default();
            SHOWN_WHEN_VALUE.iter().filter(|(shown, _, _, _)| *shown == named).all(|(_, requires, value, equal)| number_of(&facts, requires).is_none_or(|v| (v == *value) == *equal))
        })
        .filter(|f| f.get("name").and_then(Value::as_str).is_some_and(|n| !HIDDEN.contains(&n) && !DESKTOP_ONLY.iter().any(|(d, _)| *d == n) && !(MOBILE && n == "savePath")))
        .filter(|f| {
            let maker = facts.iter().find(|other| other.get("name").and_then(Value::as_str) == Some("baseReceiverManufacturers")).and_then(|other| other.get("value")).and_then(Value::as_i64).unwrap_or(MANUFACTURER_ALL);
            group != "rtkSettings" || shown_for_manufacturer(f.get("name").and_then(Value::as_str).unwrap_or_default(), maker)
        })
        .map(|f| with_choices(backend, f))
        .map(|f| decode(&f, &format!("{path}.{}", f.get("name").and_then(Value::as_str).unwrap_or(""))))
        .map(|mut control| {
            control["keywords"] = json!(fact_keywords(group, control.get("name").and_then(Value::as_str).unwrap_or_default()));
            qml_labelled(group, inverted(control))
        })
        .collect();
    let borrowed: Vec<Value> = GATED_FROM
        .iter()
        .filter(|(gated_group, _)| *gated_group == group)
        .flat_map(|(_, from)| object(&backend.get(&format!("settings.{from}"))).get("facts").and_then(Value::as_array).cloned().unwrap_or_default())
        .collect();
    let shown = gated(group, &shown, &[facts.clone(), borrowed].concat());
    let shown = match video.as_ref().is_some_and(|(_, _, auto)| *auto) {
        true => shown.into_iter().map(auto_locked).collect(),
        false => shown,
    };
    let desktop_only: Vec<&str> = facts.iter().filter_map(|f| f.get("name").and_then(Value::as_str)).filter_map(|n| DESKTOP_ONLY.iter().find(|(d, _)| *d == n).map(|(_, label)| *label)).collect();
    let note = match desktop_only.is_empty() {
        true => String::new(),
        false => format!("Set up {} on the desktop - they are stored as JSON that is not editable here.", desktop_only.join(" and ")),
    };
    json!({
        "title": title,
        "group": group,
        "path": path,
        "note": note,
        "subsections": subsections(slice, &named_apart(shown)),
    })
}

fn number_of(facts: &[Value], name: &str) -> Option<i64> {
    facts.iter().find(|f| f.get("name").and_then(Value::as_str) == Some(name)).and_then(|f| f.get("value")).and_then(Value::as_f64).map(|v| v as i64)
}

fn gated(group: &str, controls: &[Value], facts: &[Value]) -> Vec<Value> {
    let value_of = |name: &str| facts.iter().find(|f| f.get("name").and_then(Value::as_str) == Some(name)).and_then(|f| f.get("value").cloned());
    controls
        .iter()
        .map(|control| {
            let name = control.get("name").and_then(Value::as_str).unwrap_or("");
            let blocked = GATED
                .iter()
                .filter(|(gated, _, _, _)| *gated == name)
                .find(|(_, requires, wanted, _)| value_of(requires).as_ref().and_then(Value::as_bool) == Some(!wanted))
                .map(|(_, _, _, reason)| *reason)
                .or_else(|| {
                    GATED_VALUE
                        .iter()
                        .filter(|(gated, _, _, _, _)| *gated == name)
                        .find(|(_, requires, value, equal, _)| number_of(facts, requires).is_some_and(|v| (v == *value) != *equal))
                        .map(|(_, _, _, _, reason)| *reason)
                })
                .or_else(|| {
                    GATED_IN_GROUP
                        .iter()
                        .find(|(in_group, gated, requires, _)| *in_group == group && *gated == name && value_of(requires).as_ref().and_then(Value::as_bool) == Some(false))
                        .map(|(_, _, _, reason)| *reason)
                });
            let mut with_gate = control.clone();
            with_gate["enabled"] = json!(blocked.is_none());
            with_gate["disabledReason"] = blocked.map_or(Value::Null, |reason| json!(reason));
            with_gate
        })
        .collect()
}

// Two settings can carry the same shortDesc - the brand image pair differ only in a longDesc
// nobody shows - and a section then draws two identical rows holding different values. The name
// is the only thing that always differs, so a repeated label falls back to it.
fn named_apart(controls: Vec<Value>) -> Vec<Value> {
    let label_of = |c: &Value| c.get("label").and_then(Value::as_str).unwrap_or("").to_string();
    let repeated = |label: &str| controls.iter().filter(|c| label_of(c) == label).count() > 1;
    controls
        .iter()
        .map(|control| match repeated(&label_of(control)) {
            false => control.clone(),
            true => {
                let mut apart = control.clone();
                apart["label"] = json!(crate::label::humanise(control.get("name").and_then(Value::as_str).unwrap_or("")));
                apart
            }
        })
        .collect()
}

fn subsections(group: &str, controls: &[Value]) -> Vec<Value> {
    let name_of = |c: &Value| c.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let Some((_, table)) = SUBSECTIONS.iter().find(|(g, _)| *g == group) else {
        return vec![json!({ "title": "", "controls": controls })];
    };
    let named: Vec<Value> = table
        .iter()
        .map(|(title, names)| json!({ "title": title, "controls": names.iter().filter_map(|n| controls.iter().find(|c| name_of(c) == *n)).cloned().collect::<Vec<_>>() }))
        .filter(|s| !s["controls"].as_array().unwrap().is_empty())
        .collect();
    let claimed: Vec<&str> = table.iter().flat_map(|(_, names)| names.iter().copied()).collect();
    let leftovers: Vec<Value> = controls.iter().filter(|c| !claimed.contains(&name_of(c).as_str())).cloned().collect();
    match leftovers.is_empty() {
        true => named,
        false => named.into_iter().chain(std::iter::once(json!({ "title": "Other", "controls": leftovers }))).collect(),
    }
}

fn page_shown(page: &Page, connected: bool, px4: bool) -> bool {
    !page.shows_px4_logs || !connected || px4
}

pub fn settings_view(backend: &dyn Backend, args: &[String]) -> Value {
    match args.first() {
        Some(title) => PAGES.iter().find(|p| p.title == title).map(|p| page_json(p, Some(backend))).unwrap_or(json!({ "kind": "null" })),
        None => {
            let connected = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
            let px4 = flag(&object(&backend.get_fields("vehicle", "px4Firmware")), "px4Firmware");
            json!({ "kind": "object", "class": "Settings", "pages": PAGES.iter().filter(|p| page_shown(p, connected, px4)).map(|p| page_json(p, None)).collect::<Vec<_>>() })
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_font_size_is_offered_as_qgcs_ui_scaling_percentages() {
        use serde_json::json;
        let shown = super::ui_scaling(&json!({ "name": "appFontPointSize", "value": 0, "label": "Application font size", "units": "pt" }));
        assert_eq!((shown["label"].clone(), shown["enumIndex"].clone()), (json!("UI Scaling"), json!(2)), "an unset size is the platform size, 100%");
        assert_eq!(shown["enumValues"], json!([11, 13, 14, 15, 18, 21, 25, 28]), "Math.round(platform * percent / 100) with the 14 pt base");
        assert_eq!(super::nearest_scale_index(20.0), 5, "a stored 20 pt is closest to 150%");
        assert_eq!((super::scaled_point_size(7), super::scaled_point_size(8)), (Some(28), None));
    }

    #[test]
    fn every_row_the_general_fly_and_video_pages_show_has_a_heading() {
        let groups = [
            ("appSettings", include_str!("../../src/Settings/App.SettingsGroup.json")),
            ("flyViewSettings", include_str!("../../src/Settings/FlyView.SettingsGroup.json")),
            ("videoSettings", include_str!("../../src/Settings/Video.SettingsGroup.json")),
        ];
        let loose: Vec<String> = groups
            .iter()
            .flat_map(|(group, json)| {
                let headed: Vec<&str> = SUBSECTIONS.iter().filter(|(g, _)| g == group).flat_map(|(_, s)| s.iter().flat_map(|(_, n)| n.iter().copied())).collect();
                serde_json::from_str::<Value>(json).unwrap()["QGC.MetaData.Facts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|f| f["name"].as_str().map(str::to_string))
                    .filter(|n| slice_shows(group, n) && !HIDDEN.contains(&n.as_str()) && !headed.contains(&n.as_str()) && !DESKTOP_ONLY.iter().any(|(d, _)| d == n))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(loose.is_empty(), "these would land in an Other block: {loose:?}");
    }

    #[test]
    fn map_provider_rows_sit_on_the_maps_page_as_map_settings_qml_heads_them() {
        assert!(slice_shows(MAP_PROVIDERS, "mapboxToken") && !slice_shows("appSettings", "mapboxToken"));
        assert!(slice_shows("appSettings", "indoorPalette") && !slice_shows(MAP_PROVIDERS, "indoorPalette"));
        let controls: Vec<Value> = ["customURL", "mapboxAccount", "esriToken"].iter().map(|n| json!({ "name": n })).collect();
        let titles: Vec<String> = subsections(MAP_PROVIDERS, &controls).iter().map(|s| s["title"].as_str().unwrap().to_string()).collect();
        assert_eq!(titles, ["Tokens", "Mapbox Login", "Custom Map URL"]);
    }

    #[test]
    fn the_hidden_mode_lists_are_edited_from_the_mode_picker_not_the_settings_page() {
        assert!(["px4HiddenFlightModesMultiRotor", "apmHiddenFlightModesAirship"].iter().all(|name| HIDDEN.contains(name)), "QGC edits these from FlightModeIndicator, and the head's picker has the per-mode switches");
    }

    #[test]
    fn gimbal_rows_group_as_the_gimbal_indicator_heads_them() {
        let controls: Vec<Value> = ["zoomMinSpeed", "clickAndDrag", "showAzimuthIndicatorOnMap", "enableOnScreenControl"].iter().map(|n| json!({ "name": n })).collect();
        let titles: Vec<String> = subsections("gimbalControllerSettings", &controls).iter().map(|s| s["title"].as_str().unwrap().to_string()).collect();
        assert_eq!(titles, ["On-Screen Control", "Zoom speed", ""]);
        assert!(HIDDEN_WHEN.contains(&("cameraSlideSpeed", "clickAndDrag", false)), "GimbalIndicator.qml shows the slide speed only for click-and-drag");
    }

    #[test]
    fn nmea_rows_sit_under_their_own_heading_with_link_settings_labels() {
        let controls: Vec<Value> = ["nmeaUdpPort", "autoConnectUDP", "nmeaSource", "autoConnectPixhawk"].iter().map(|n| json!({ "name": n })).collect();
        let titled: Vec<(String, Vec<String>)> = subsections("autoConnectSettings", &controls)
            .iter()
            .map(|s| (s["title"].as_str().unwrap().to_string(), s["controls"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap().to_string()).collect()))
            .collect();
        assert_eq!(
            titled,
            vec![("".to_string(), vec!["autoConnectPixhawk".to_string(), "autoConnectUDP".to_string()]), ("NMEA GPS".to_string(), vec!["nmeaSource".to_string(), "nmeaUdpPort".to_string()])]
        );
        let label = |group: &str, name: &str| qml_labelled(group, json!({ "name": name, "label": "fact json" }))["label"].as_str().unwrap().to_string();
        assert_eq!(
            [label("autoConnectSettings", "nmeaSource"), label("autoConnectSettings", "autoConnectSiKRadio"), label("appSettings", "virtualJoystick")],
            ["Source", "SiK Radio", "Enabled"]
        );
    }

    #[test]
    fn remote_id_fields_have_no_effect_while_their_broadcast_is_off_as_qgc_disables_them() {
        let controls: Vec<Value> = ["basicID", "selfIDFree", "selfIDEmergency"].iter().map(|n| json!({ "name": n, "enabled": true })).collect();
        let facts = |on: bool| ["sendBasicID", "sendSelfID"].iter().map(|n| json!({ "name": n, "value": on })).collect::<Vec<_>>();
        let enabled = |on: bool| gated("", &controls, &facts(on)).iter().map(|c| c["enabled"].as_bool().unwrap_or(true)).collect::<Vec<_>>();
        assert_eq!(enabled(false), [false, false, true], "RemoteIDSettings.qml enables the ID fields on their switch, but never gates the emergency text");
        assert_eq!(enabled(true), [true, true, true]);
    }

    #[test]
    fn remote_id_rows_follow_qgcs_groups_with_each_switch_ahead_of_its_fields() {
        let controls: Vec<Value> = ["operatorIDEU", "sendOperatorID", "region", "basicID", "sendBasicID"].iter().map(|n| json!({ "name": n })).collect();
        let order: Vec<String> = subsections("remoteIDSettings", &controls).iter().flat_map(|s| s["controls"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap().to_string()).collect::<Vec<_>>()).collect();
        assert_eq!(order, ["region", "sendBasicID", "basicID", "sendOperatorID", "operatorIDEU"], "RemoteIDSettings.qml order");
    }

    #[test]
    fn the_mavlink_page_groups_its_rows_under_qgcs_headings_so_enable_reads_as_forwarding() {
        let controls: Vec<Value> = ["telemetrySave", "forwardMavlink", "forwardMavlinkHostName", "sendGCSHeartbeat"].iter().map(|n| json!({ "name": n })).collect();
        let titles: Vec<(String, Vec<String>)> = subsections("mavlinkSettings", &controls)
            .iter()
            .map(|s| (s["title"].as_str().unwrap().to_string(), s["controls"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap().to_string()).collect()))
            .collect();
        assert_eq!(
            titles,
            vec![
                ("Telemetry logs".to_string(), vec!["telemetrySave".to_string()]),
                ("Ground Station".to_string(), vec!["sendGCSHeartbeat".to_string()]),
                ("MAVLink Forwarding".to_string(), vec!["forwardMavlink".to_string(), "forwardMavlinkHostName".to_string()]),
            ],
            "TelemetrySettings.qml heads these groups; without them the forwarding switch reads as a bare Enable"
        );
    }

    #[test]
    fn the_video_page_has_no_active_source_index_row_as_qgc_does_not() {
        let video = SUBSECTIONS.iter().find(|(group, _)| *group == "videoSettings").map(|(_, sections)| *sections).unwrap_or_default();
        assert!(video.iter().all(|(_, names)| !names.contains(&"activeVideoSource")), "a bare camera index is meaningless to an operator; the Fly camera sheet switches sources");
    }

    use super::*;

    #[test]
    fn px4_log_transfer_is_listed_only_for_px4_or_with_no_vehicle() {
        let page = PAGES.iter().find(|p| p.shows_px4_logs).unwrap();
        assert!(page_shown(page, false, false), "SettingsPagesModel shows it with no vehicle");
        assert!(page_shown(page, true, true));
        assert!(!page_shown(page, true, false), "an ArduPilot vehicle does not get it");
        assert!(PAGES.iter().filter(|p| !p.shows_px4_logs).all(|p| page_shown(p, true, false)));
    }

    #[test]
    fn the_nmea_port_is_picked_from_the_ports_found_like_nmeagpssettings() {
        let fact = json!({ "name": "autoConnectNmeaPort", "value": "/dev/ttyUSB1", "typeIsString": true });
        let chosen = choices_json(&fact, vec!["GPS".into(), "Radio".into()], vec![json!("/dev/ttyUSB0"), json!("/dev/ttyUSB1")]);
        assert_eq!(chosen["enumIndex"], 1);
        let control = decode(&chosen, "settings.autoConnectSettings.autoConnectNmeaPort");
        assert_eq!(control["control"], "choice");
        assert_eq!(control["display"], "Radio");
    }

    #[test]
    fn video_rows_follow_the_source_like_video_settings() {
        assert!(video_row_shown("rtspUrl", "RTSP Video Stream", true, false));
        let locked = auto_locked(json!({ "name": "videoSource", "enabled": true }));
        assert_eq!((locked["enabled"].clone(), locked["disabledReason"].clone()), (json!(false), json!(AUTO_CONFIGURED)), "VideoSettings.qml locks camera 0's name, source and URL while _videoAutoStreamConfig");
        assert_eq!(auto_locked(json!({ "name": "streamEnabled", "enabled": true }))["enabled"], true);
        assert!(!video_row_shown("udpUrl", "RTSP Video Stream", true, false), "only the selected source's URL");
        assert!(video_row_shown("udpUrl", "MPEG-TS Video Stream", true, false));
        assert!(!video_row_shown("whepUrl", "Video Stream Disabled", false, false));
        assert!(!video_row_shown("lowLatencyMode", "RTSP Video Stream", true, true), "auto-configured streams hide the stream rows");
        assert!(!video_row_shown("rtspTimeout", "UVC Device", false, false), "and so does a source that is not a stream");
        assert!(video_row_shown("videoFit", "UVC Device", false, false));
    }

    #[test]
    fn apm_stream_rates_show_for_an_apm_vehicle_or_none_like_telemetry_settings() {
        assert!(apm_streams_apply(false, false));
        assert!(apm_streams_apply(true, true));
        assert!(!apm_streams_apply(true, false));
    }

    #[test]
    fn the_apm_stream_switch_reads_controlled_by_vehicle_and_flips_like_telemetry_settings() {
        let shown = inverted(json!({ "name": "apmStartMavlinkStreams", "label": "Request start", "value": true }));
        assert_eq!((shown["label"].clone(), shown["shortLabel"].clone(), shown["inverted"].clone(), shown["value"].clone()), (json!("Controlled by Vehicle"), json!("Controlled by Vehicle"), json!(true), json!(true)), "TelemetrySettings.qml checked: !rawValue, the raw value stays as stored");
        assert!(inverted(json!({ "name": "telemetrySave" })).get("inverted").is_none());
    }

    #[test]
    fn the_sd_card_switch_reads_save_to_sd_card_and_flips_like_general_settings() {
        let shown = inverted(json!({ "name": "androidDontSaveToSDCard", "label": "Don't save to SD card, even if available", "value": false }));
        assert_eq!((shown["label"].clone(), shown["inverted"].clone(), shown["value"].clone()), (json!("Save application data to SD Card"), json!(true), json!(false)), "GeneralSettings.qml checkedValue: false, uncheckedValue: true");
        assert_eq!(qml_labelled("appSettings", json!({ "name": "audioMuted", "label": "Mute Audio Output" }))["label"], "Mute all audio output");
    }

    #[test]
    fn gimbal_rows_carry_the_text_gimbal_indicator_qml_gives_them() {
        let label = |name: &str| qml_labelled("gimbalControllerSettings", json!({ "name": name, "label": "fact json" }))["label"].as_str().unwrap().to_string();
        assert_eq!(
            ["enableOnScreenControl", "cameraHFov", "toolbarIndicatorShowAzimuth"].map(label),
            ["Enabled", "Horizontal FOV", "Use Azimuth instead of local yaw on top toolbar indicator"]
        );
    }

    #[test]
    fn video_rows_carry_the_text_video_settings_qml_gives_them() {
        let shown = qml_labelled("videoSettings", json!({ "name": "disableWhenDisarmed", "label": "Disable Video Stream When Disarmed" }));
        assert_eq!((shown["label"].clone(), shown["shortLabel"].clone()), (json!("Stop recording when disarmed"), json!("Stop recording when disarmed")));
        assert_eq!(qml_labelled("videoSettings", json!({ "name": "maxVideoSize" }))["label"], "Storage Limit");
        assert_eq!(qml_labelled("flyViewSettings", json!({ "name": "updateHomePosition", "label": "Home follows this device" }))["label"], "Update return to home position based on device location");
        assert_eq!(qml_labelled("viewer3DSettings", json!({ "name": "enabled", "label": "Enable the 3D viewer" }))["label"], "Enabled");
        assert_eq!(qml_labelled("packetRadioSettings", json!({ "name": "enabled", "label": "Receive over packet radio" }))["label"], "Receive over packet radio");
        assert_eq!(qml_labelled("flyViewSettings", json!({ "name": "maxVideoSize", "label": "Kept" }))["label"], "Kept", "the override is the video page's");
        assert_eq!(qml_labelled("videoSettings", json!({ "name": "videoFit", "label": "Video Display Fit" }))["label"], "Video Display Fit");
    }

    #[test]
    fn telemetry_and_adsb_rows_follow_their_switches_like_the_qml_pages() {
        let controls: Vec<Value> = ["forwardMavlinkHostName", "telemetrySaveNotArmed", "adsbServerHostAddress", "adsbServerPort", "streamRateRawSensors", "streamRateExtra3"].iter().map(|n| json!({ "name": n })).collect();
        let facts = |on: bool| ["forwardMavlink", "telemetrySave", "adsbServerConnectEnabled", "apmStartMavlinkStreams"].iter().map(|n| json!({ "name": n, "value": on })).collect::<Vec<_>>();
        assert!(gated("", &controls, &facts(false)).iter().all(|c| c["enabled"] == false && c["disabledReason"].is_string()));
        assert!(gated("", &controls, &facts(true)).iter().all(|c| c["enabled"] == true));
    }

    struct Fake;
    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.appSettings" => json!({ "kind": "object", "facts": [
                    { "kind": "fact", "name": "audioMuted", "typeIsBool": true, "value": false },
                    { "kind": "fact", "name": "useChecklist", "typeIsBool": true, "value": true },
                    { "kind": "fact", "name": "firstRunPromptIdsShown", "typeIsString": true },
                    { "kind": "fact", "name": "rcControls", "typeIsString": true },
                    { "kind": "fact", "name": "somethingNew", "typeIsString": true },
                ] }),
                _ => json!({ "kind": "object", "facts": [ { "kind": "fact", "name": "verticalDistanceUnits", "enumStrings": ["Feet", "Meters"], "enumIndex": 1 } ] }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn every_name_these_tables_key_on_exists_and_belongs_to_one_group() {
        const GROUPS: &[(&str, &str)] = &[
            ("App", include_str!("../../src/Settings/App.SettingsGroup.json")),
            ("Video", include_str!("../../src/Settings/Video.SettingsGroup.json")),
            ("RTK", include_str!("../../src/Settings/RTK.SettingsGroup.json")),
            ("FlyView", include_str!("../../src/Settings/FlyView.SettingsGroup.json")),
            ("PacketRadio", include_str!("../../src/Settings/PacketRadio.SettingsGroup.json")),
            ("Viewer3D", include_str!("../../src/Settings/Viewer3D.SettingsGroup.json")),
            ("NTRIP", include_str!("../../src/Settings/NTRIP.SettingsGroup.json")),
            ("MavlinkActions", include_str!("../../src/Settings/MavlinkActions.SettingsGroup.json")),
            ("Mavlink", include_str!("../../src/Settings/Mavlink.SettingsGroup.json")),
            ("ADSBVehicleManager", include_str!("../../src/Settings/ADSBVehicleManager.SettingsGroup.json")),
            ("APMMavlinkStreamRate", include_str!("../../src/Settings/APMMavlinkStreamRate.SettingsGroup.json")),
            ("AutoConnect", include_str!("../../src/Settings/AutoConnect.SettingsGroup.json")),
            ("LogManager", include_str!("../../src/Settings/LogManager.SettingsGroup.json")),
            ("PlanView", include_str!("../../src/Settings/PlanView.SettingsGroup.json")),
            ("RemoteID", include_str!("../../src/Settings/RemoteID.SettingsGroup.json")),
            ("GimbalController", include_str!("../../src/Settings/GimbalController.SettingsGroup.json")),
            ("FlightMode", include_str!("../../src/Settings/FlightMode.SettingsGroup.json")),
        ];
        let declares = |name: &str| -> Vec<&str> {
            GROUPS.iter().filter(|(_, body)| body.contains(&format!("\"{name}\""))).map(|(group, _)| *group).collect()
        };
        assert_eq!(declares("useFixedBasePosition"), vec!["RTK"], "the parser has to find a name it should, or every assertion below passes by finding nothing");
        assert_eq!(declares("enabled").len(), 2, "enabled is declared in two groups and that is what makes uniqueness worth asserting rather than assumed");

        let keyed: Vec<&str> = HIDDEN
            .iter()
            .copied()
            .chain(DESKTOP_ONLY.iter().map(|(name, _)| *name))
            .chain(GATED.iter().flat_map(|(gated, requires, _, _)| [*gated, *requires]))
            .chain(HIDDEN_WHEN.iter().flat_map(|(hidden, requires, _)| [*hidden, *requires]))
            .chain(SHOWN_WHEN_VALUE.iter().flat_map(|(shown, requires, _, _)| [*shown, *requires]))
            .chain(GATED_VALUE.iter().flat_map(|(gated, requires, _, _, _)| [*gated, *requires]))
            .collect();

        keyed.iter().for_each(|name| {
            let groups = declares(name);
            assert!(
                !groups.is_empty(),
                "{name} is keyed on by one of these tables and no settings group declares it. The entry stops matching silently, and every one of these fails OPEN - a stale HIDDEN or DESKTOP_ONLY makes a control reappear on a page it was deliberately kept off, a stale GATED turns the enforce-checklist switch back into a live one, and a stale HIDDEN_WHEN puts both RTK mode groups back on screen at once. The page looks fuller and nothing fails"
            );
            assert_eq!(
                groups.len(),
                1,
                "{name} is declared in {groups:?}. These tables key on a BARE fact name and the filters run per group, so an entry naming a shared name applies to every page that declares it - one line that looks like it names one control silently reaching two"
            );
        });
    }

    #[test]
    fn the_ntrip_server_is_locked_while_connected_and_the_connect_switch_lives_in_the_status_block() {
        struct Ntrip(bool, bool);
        impl Backend for Ntrip {
            fn get(&self, path: &str) -> String {
                let facts = json!([
                    { "kind": "fact", "name": "ntripServerConnectEnabled", "typeIsBool": true, "value": self.0 },
                    { "kind": "fact", "name": "ntripServerHostAddress", "typeIsString": true, "value": "caster" },
                    { "kind": "fact", "name": "ntripUseTls", "typeIsBool": true, "value": self.1 },
                    { "kind": "fact", "name": "ntripAllowSelfSignedCerts", "typeIsBool": true, "value": false },
                ]);
                match path {
                    "settings.ntripSettings" => json!({ "kind": "object", "facts": facts }),
                    _ => json!({ "kind": "object", "facts": [] }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let controls = |connected: bool, tls: bool| -> Vec<(String, Value)> {
            let page = settings_view(&Ntrip(connected, tls), &["NTRIP / RTK".to_string()]);
            assert_eq!(page["showsNtrip"], true);
            page["sections"][0]["subsections"].as_array().unwrap().iter().flat_map(|sub| sub["controls"].as_array().unwrap().clone()).map(|c| (c["name"].as_str().unwrap().to_string(), c["disabledReason"].clone())).collect()
        };
        let idle = controls(false, true);
        assert_eq!(idle, [("ntripServerHostAddress".to_string(), Value::Null), ("ntripUseTls".to_string(), Value::Null), ("ntripAllowSelfSignedCerts".to_string(), Value::Null)]);
        assert!(controls(true, true).iter().all(|(_, reason)| *reason == json!(NTRIP_ACTIVE)));
        assert_eq!(controls(false, false)[2].1, json!(NTRIP_NO_TLS));
    }

    #[test]
    fn rtk_rows_follow_the_receiver_maker_like_gps_indicator_page() {
        assert!(super::shown_for_manufacturer("surveyInAccuracyLimit", 4) && !super::shown_for_manufacturer("surveyInAccuracyLimit", 2));
        assert!(super::shown_for_manufacturer("surveyInMinObservationDuration", 1) && !super::shown_for_manufacturer("surveyInMinObservationDuration", 2));
        assert!(super::shown_for_manufacturer("fixedBasePositionLatitude", 2), "position rows show for every maker");
        assert!(super::shown_for_manufacturer("fixedBasePositionAccuracy", 0), "All shows every row");
    }

    #[test]
    fn the_rtk_page_shows_one_mode_at_a_time_rather_than_both_sets_at_once() {
        struct Rtk(Option<bool>);
        impl Backend for Rtk {
            fn get(&self, path: &str) -> String {
                let mode = self.0.map(|fixed| json!({ "kind": "fact", "name": "useFixedBasePosition", "enumStrings": ["Survey-In", "Fixed"], "value": u8::from(fixed) }));
                let facts: Vec<Value> = mode
                    .into_iter()
                    .chain(["surveyInAccuracyLimit", "surveyInMinObservationDuration", "fixedBasePositionLatitude", "fixedBasePositionLongitude", "fixedBasePositionAltitude", "fixedBasePositionAccuracy"].iter().map(|n| json!({ "kind": "fact", "name": n, "typeIsString": true })))
                    .collect();
                match path {
                    "settings.rtkSettings" => json!({ "kind": "object", "facts": facts }),
                    _ => json!({ "kind": "object", "facts": [] }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let shown = |fixed: Option<bool>| -> Vec<String> {
            settings_view(&Rtk(fixed), &["RTK GPS".to_string()])["sections"][0]["subsections"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|sub| sub["controls"].as_array().unwrap().clone())
                .filter_map(|c| c["name"].as_str().map(str::to_string))
                .collect()
        };

        let surveying = shown(Some(false));
        assert!(surveying.contains(&"surveyInAccuracyLimit".to_string()) && surveying.contains(&"surveyInMinObservationDuration".to_string()));
        assert!(
            !surveying.iter().any(|n| n.starts_with("fixedBasePosition")),
            "GPSIndicatorPage.qml binds visible on all six, and the selector above them is a Survey-In / Specify position radio pair - so a head drawing four editable base-position boxes under a Survey-In selection is offering the mode the operator did not choose. Shown: {surveying:?}"
        );

        let specified = shown(Some(true));
        assert_eq!(specified.iter().filter(|n| n.starts_with("fixedBasePosition")).count(), 4);
        assert!(!specified.iter().any(|n| n.starts_with("surveyIn")), "and the mirror: choosing a fixed position puts the survey-in limits away rather than leaving both sets on the page");

        assert_eq!(
            shown(None).len(),
            6,
            "with no useFixedBasePosition in the payload nothing is hidden. A bridge that cannot answer which mode is selected must not empty the page - the same permissive rule as the visible flag, and the direction that fails safe"
        );
    }

    #[test]
    fn remote_id_rows_follow_region_location_and_classification_like_remote_id_settings_qml() {
        struct Rid(i64, i64, i64);
        impl Backend for Rid {
            fn get(&self, path: &str) -> String {
                let numbers = [("region", self.0), ("locationType", self.1), ("classificationType", self.2)];
                let facts: Vec<Value> = numbers
                    .iter()
                    .map(|(n, v)| json!({ "kind": "fact", "name": n, "value": v }))
                    .chain(["sendOperatorID", "operatorIDType", "operatorIDEU", "operatorIDFAA", "latitudeFixed", "categoryEU", "classEU"].iter().map(|n| json!({ "kind": "fact", "name": n, "typeIsString": true })))
                    .collect();
                match path {
                    "settings.remoteIDSettings" => json!({ "kind": "object", "facts": facts }),
                    _ => json!({ "kind": "object", "facts": [] }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let rows = |rid: Rid| -> Vec<(String, bool)> {
            settings_view(&rid, &["Remote ID".to_string()])["sections"][0]["subsections"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|sub| sub["controls"].as_array().unwrap().clone())
                .map(|c| (c["name"].as_str().unwrap().to_string(), c["enabled"].as_bool().unwrap()))
                .collect()
        };
        let faa = rows(Rid(0, 2, 1));
        let names: Vec<&str> = faa.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["region", "sendOperatorID", "operatorIDFAA", "locationType", "latitudeFixed"]);
        assert!(faa.iter().all(|(_, enabled)| *enabled));
        let eu = rows(Rid(1, 0, 0));
        let enabled = |name: &str| eu.iter().find(|(n, _)| n == name).unwrap().1;
        assert!(eu.iter().any(|(n, _)| n == "operatorIDEU") && !eu.iter().any(|(n, _)| n == "operatorIDFAA"));
        assert!(!enabled("sendOperatorID") && !enabled("latitudeFixed") && !enabled("categoryEU") && !enabled("classEU") && enabled("classificationType"));
        assert!(rows(Rid(1, 2, 1)).iter().all(|(n, enabled)| *enabled || n == "sendOperatorID"));
    }

    #[test]
    fn a_setting_qgc_hides_on_this_platform_is_not_offered_as_an_editable_row() {
        struct Group(Vec<Value>);
        impl Backend for Group {
            fn get(&self, path: &str) -> String {
                match path {
                    "settings.appSettings" => json!({ "kind": "object", "facts": self.0 }),
                    _ => json!({ "kind": "object", "facts": [] }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let names = |facts: Vec<Value>| -> Vec<String> {
            settings_view(&Group(facts), &["General".to_string()])["sections"][0]["subsections"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|sub| sub["controls"].as_array().unwrap().clone())
                .filter_map(|c| c["name"].as_str().map(str::to_string))
                .collect()
        };
        let fact = |name: &str, visible: Option<bool>| {
            let mut f = json!({ "kind": "fact", "name": name, "typeIsString": true });
            visible.into_iter().for_each(|v| { f["visible"] = json!(v); });
            f
        };

        assert_eq!(
            names(vec![fact("savePath", Some(false)), fact("audioMuted", Some(true))]),
            vec!["audioMuted"],
            "AppSettings.cc:134 calls setVisible(false) on savePath under Q_OS_ANDROID, and :102 then hardcodes userHasModifiedSavePath = false so the runtime path overwrites whatever was stored. Serving the row let an operator set a save directory, watch the fact read it back, and find it silently reverted on the next launch - the write succeeded and was undone with nothing said"
        );

        assert_eq!(
            names(vec![fact("audioMuted", None)]),
            vec!["audioMuted"],
            "absent must mean visible. Every fact served by a bridge without this property in its allowlist arrives with no visible key, and a filter reading absence as hidden empties every settings page at once - which is how this whole class of field goes wrong in the safe-looking direction"
        );

        assert_eq!(names(vec![fact("savePath", Some(true))]), vec!["savePath"], "and a fact QGC does show is shown");
    }

    struct Group(&'static str, Vec<Value>);
    impl Backend for Group {
        fn get(&self, path: &str) -> String {
            match path.strip_prefix("settings.") == Some(self.0) {
                true => json!({ "kind": "object", "facts": self.1 }),
                false => json!({ "kind": "object", "facts": [] }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn page_rows(backend: &dyn Backend, page: &str) -> Vec<Value> {
        settings_view(backend, &[page.to_string()])["sections"].as_array().unwrap().iter()
            .flat_map(|s| s["subsections"].as_array().cloned().unwrap_or_default())
            .flat_map(|sub| sub["controls"].as_array().cloned().unwrap_or_default())
            .collect()
    }

    #[test]
    fn pages_and_settings_carry_qgc_search_keywords() {
        assert!(page_keywords("General").contains("imperial"), "SettingsPagesModel General keywords");
        assert!(page_keywords("General").contains("appearance"), "the summary is searched too");
        assert!(page_keywords("MAVLink").contains("forwarding"), "Telemetry is QGC's name for this page");
        assert!(page_keywords("3D Viewer").contains("osm"));
        assert_eq!(page_keywords("RTK GPS"), "", "a page QGC does not list has no keywords");
        let facts = vec![json!({ "kind": "fact", "name": "enabled", "typeIsBool": true, "value": true })];
        let row = page_rows(&Group("viewer3DSettings", facts), "3D Viewer").into_iter().find(|c| c["name"] == "enabled").unwrap();
        assert!(row["keywords"].as_str().unwrap().contains("3d"));
        assert!(!fact_keywords("packetRadioSettings", "enabled").contains("3d"), "keywords belong to the group that declares the fact");
        assert!(fact_keywords("logManagerSettings", "diskLoggingEnabled").contains("rotation"), "Logging.SettingsUI.json section keywords reach its controls");
        assert!(fact_keywords("viewer3DSettings", "buildingLevelHeight").contains("openstreetmap"));
    }

    #[test]
    fn nmea_rows_follow_the_source_like_nmea_gps_settings_qml() {
        let names = |source: i64| -> Vec<String> {
            let facts = vec![
                json!({ "kind": "fact", "name": "nmeaSource", "value": source }),
                json!({ "kind": "fact", "name": "autoConnectNmeaPort", "value": "", "typeIsString": true }),
                json!({ "kind": "fact", "name": "autoConnectNmeaBaud", "value": 4800 }),
                json!({ "kind": "fact", "name": "nmeaUdpPort", "value": 10110 }),
            ];
            page_rows(&Group("autoConnectSettings", facts), "Connections").iter().filter_map(|c| c["name"].as_str().map(str::to_string)).collect()
        };
        assert_eq!(names(0), vec!["nmeaSource"]);
        assert_eq!(names(1), vec!["nmeaSource", "nmeaUdpPort"]);
        assert_eq!(names(2), vec!["nmeaSource", "autoConnectNmeaPort", "autoConnectNmeaBaud"]);
    }

    #[test]
    fn the_3d_data_rows_are_greyed_while_the_3d_view_is_off() {
        let rows = |on: bool| {
            let facts = ["enabled", "osmFilePath", "buildingLevelHeight", "altitudeBias"].iter()
                .map(|n| json!({ "kind": "fact", "name": n, "typeIsBool": *n == "enabled", "value": if *n == "enabled" { json!(on) } else { json!(1) } }))
                .collect();
            page_rows(&Group("viewer3DSettings", facts), "3D Viewer")
        };
        let off = rows(false);
        let data = off.iter().find(|c| c["name"] == "altitudeBias").unwrap();
        assert_eq!((data["enabled"].as_bool(), data["disabledReason"].as_str()), (Some(false), Some(VIEWER_3D_OFF)));
        assert!(rows(true).iter().all(|c| c["enabled"] != false));
    }

    #[test]
    fn enforcing_a_checklist_nobody_is_using_is_offered_as_a_switch_that_does_nothing() {
        struct Checklist(Option<bool>);
        impl Backend for Checklist {
            fn get(&self, path: &str) -> String {
                let facts: Vec<Value> = [
                    Some(json!({ "kind": "fact", "name": "enforceChecklist", "typeIsBool": true, "value": true })),
                    self.0.map(|on| json!({ "kind": "fact", "name": "useChecklist", "typeIsBool": true, "value": on })),
                ]
                .into_iter()
                .flatten()
                .collect();
                match path {
                    "settings.appSettings" => json!({ "kind": "object", "facts": facts }),
                    _ => json!({ "kind": "object", "facts": [] }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let enforce = |using: Option<bool>| {
            settings_view(&Checklist(using), &["General".to_string()])["sections"][0]["subsections"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|sub| sub["controls"].as_array().unwrap().clone())
                .find(|c| c["name"] == "enforceChecklist")
                .expect("the control has to be in the section at all, or every assertion below passes by not finding it")
        };

        let on = enforce(Some(true));
        assert_eq!(on["enabled"], true);
        assert_eq!(on["disabledReason"], Value::Null);

        let off = enforce(Some(false));
        assert_eq!(off["enabled"], false, "FlyViewSettings.qml binds this row's enabled to useChecklist.value and GuidedActionsController ANDs the two, so with the checklist off the switch is live, writes a value, and changes nothing anywhere");
        assert_eq!(off["disabledReason"], CHECKLIST_OFF, "the reason states what is true of the setting rather than telling the operator which switch to find - the head owns the call to action and phrases it where the other switch actually is");
        assert_eq!(off["readOnly"], on["readOnly"], "readOnly is the Fact's own property and means the settings file said so; borrowing it for this would tell a head the setting cannot be written when it can");

        let absent = enforce(None);
        assert_eq!(absent["enabled"], true, "a gate whose required setting is not in the payload leaves the control alone, and this case is asserted rather than left to a fixture that happens to omit the key: a rule that disables on absence would grey out every row the moment a group is read with a field list that excludes its gate");
    }

    #[test]
    fn exactly_one_page_carries_the_packet_radio_block() {
        let flagged: Vec<&str> = PAGES.iter().filter(|page| page.shows_packet_radio).map(|page| page.title).collect();
        assert_eq!(flagged, vec!["Packet Radio"], "a head draws its bespoke block from this flag, so a second page carrying it draws the radio twice and none carrying it draws the settings with no status line at all");
        let bespoke: Vec<&str> = PAGES
            .iter()
            .filter(|page| [page.shows_links, page.shows_about, page.shows_video_sources, page.shows_packet_radio, page.shows_ntrip].iter().filter(|on| **on).count() > 1)
            .map(|page| page.title)
            .collect();
        assert!(bespoke.is_empty(), "these pages claim more than one bespoke block and a head has one slot for it: {bespoke:?}");
    }

    #[test]
    fn the_page_list_carries_no_controls_and_the_page_carries_decoded_ones() {
        let list = settings_view(&Fake, &[]);
        assert_eq!(list["pages"].as_array().unwrap().len(), 19);
        assert_eq!(list["pages"][11]["showsPx4Logs"], true, "PX4 Log Transfer follows Remote ID as in SettingsPagesModel");
        assert_eq!(list["pages"][17]["showsConsole"], true, "Console sits under Diagnostics as in SettingsPagesModel");
        assert_eq!(list["pages"][18]["title"], "App Logging", "and App Logging follows it there");
        assert!(list["pages"][0]["sections"][0].get("subsections").is_none());
        let general = settings_view(&Fake, &["General".to_string()]);
        let app = &general["sections"][0];
        assert_eq!(app["path"], "settings.appSettings");
        let subs = app["subsections"].as_array().unwrap();
        assert_eq!(subs[0]["title"], "Sound");
        assert_eq!(subs[0]["controls"][0]["control"], "toggle");
        assert_eq!(subs[1]["title"], "Preflight checklist");
        assert_eq!(subs.last().unwrap()["title"], "Other");
        assert_eq!(subs.last().unwrap()["controls"][0]["name"], "somethingNew");
        assert!(app["note"].as_str().unwrap().contains("on-screen RC controls"));
        let units = &general["sections"][1];
        assert_eq!(units["subsections"][0]["title"], "");
        assert_eq!(units["subsections"][0]["controls"][0]["control"], "choice");
        assert_eq!(settings_view(&Fake, &["Nope".to_string()])["kind"], "null");
    }

    #[test]
    fn two_settings_that_share_a_description_are_told_apart_by_name() {
        let same = |name: &str| json!({ "kind": "object", "name": name, "label": "User-selected brand image" });
        let apart = named_apart(vec![same("userBrandImageIndoor"), same("userBrandImageOutdoor")]);
        assert_eq!(apart[0]["label"], "User Brand Image Indoor");
        assert_eq!(apart[1]["label"], "User Brand Image Outdoor", "two rows reading the same thing and holding different values cannot be told apart at all");

        let single = named_apart(vec![json!({ "name": "audioMuted", "label": "Mute audio output" })]);
        assert_eq!(single[0]["label"], "Mute audio output", "a label nothing else claims is the one the metadata wrote");
    }

    #[test]
    fn the_about_page_carries_help_settings_links() {
        let about = PAGES.iter().find(|p| p.shows_about).map(|p| page_json(p, None)).unwrap();
        assert_eq!(about["helpLinks"][0]["name"], "QGroundControl User Guide");
        assert_eq!(about["helpLinks"][1]["host"], "discuss.px4.io", "the link text is the host, as HelpSettings shows it");
        let general = PAGES.iter().find(|p| !p.shows_about).map(|p| page_json(p, None)).unwrap();
        assert!(general["helpLinks"].as_array().unwrap().is_empty());
    }
}
