use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use crate::control::decode;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.vehicleTypeString", "vehicle.parameterManager.parametersReady", "vehicle.px4Firmware", "vehicle.apmFirmware", "vehicle.multiRotor"];

const DEFAULT_CHART_SECONDS: f64 = 8.0;
const PAGES: &[(&str, &str)] = &[
    ("PX4TuningComponentCopterAll.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentCopterAll.qml")),
    ("PX4TuningComponentPlaneAll.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentPlaneAll.qml")),
    ("PX4TuningComponentSpacecraftAll.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentSpacecraftAll.qml")),
    ("PX4TuningComponentVTOL.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentVTOL.qml")),
    ("PX4TuningComponentCopterRate.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentCopterRate.qml")),
    ("PX4TuningComponentCopterAttitude.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentCopterAttitude.qml")),
    ("PX4TuningComponentCopterVelocity.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentCopterVelocity.qml")),
    ("PX4TuningComponentCopterPosition.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentCopterPosition.qml")),
    ("PX4TuningComponentPlaneRate.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentPlaneRate.qml")),
    ("PX4TuningComponentPlaneAttitude.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentPlaneAttitude.qml")),
    ("PX4TuningComponentSpacecraftRate.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentSpacecraftRate.qml")),
    ("PX4TuningComponentSpacecraftAttitude.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentSpacecraftAttitude.qml")),
    ("PX4TuningComponentSpacecraftVelocity.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentSpacecraftVelocity.qml")),
    ("PX4TuningComponentSpacecraftPosition.qml", include_str!("../../src/AutoPilotPlugins/PX4/PX4TuningComponentSpacecraftPosition.qml")),
    (APM_ADVANCED, include_str!("../../src/AutoPilotPlugins/APM/APMAdvancedTuningCopterComponent.qml")),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub title: String,
    pub description: String,
    pub param: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub name: String,
    pub plot_title: String,
    pub plot: Vec<Plot>,
    pub params: Vec<Param>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub tab: String,
    pub title: String,
    pub unit: String,
    pub extras: Vec<String>,
    pub tuning_mode: i64,
    pub auto_mode_change: bool,
    pub auto_tuning: bool,
    pub chart_seconds: f64,
    pub axes: Vec<Axis>,
}

pub const SET_TELEMETRY_MODE: &str = "vehicle.setPIDTuningTelemetryMode";
const TUNING_MODES: [&str; 4] = ["ModeDisabled", "ModeRateAndAttitude", "ModeVelocityAndPosition", "ModeAltitudeAndAirspeed"];

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|error| panic!("tuning pattern {pattern}: {error}"))
}

static TAB: LazyLock<Regex> = LazyLock::new(|| regex(r#"ListElement\s*\{\s*buttonText:\s*qsTr\("([^"]+)"\)\s*tuningPage:\s*"([^"]+)"\s*\}"#));
static ELEMENT: LazyLock<Regex> = LazyLock::new(|| regex(r"(?s)ListElement\s*\{(.*?)\}"));
static AXIS_ORDER: LazyLock<Regex> = LazyLock::new(|| regex(r"axis:\s*\[([^\]]*)\]"));
static UNIT: LazyLock<Regex> = LazyLock::new(|| regex(r#"unit:\s*(?:qsTr\()?"([^"]*)""#));
static PLAIN_TITLE: LazyLock<Regex> = LazyLock::new(|| regex(r#"\btitle:\s*"([^"]+)""#));
static TITLE: LazyLock<Regex> = LazyLock::new(|| regex(r#"PIDTuning\s*\{[^}]*?title:\s*qsTr\("([^"]+)"\)"#));
static PLOT: LazyLock<Regex> = LazyLock::new(|| regex(r#"\{\s*name:\s*"([^"]+)",\s*value:\s*globals\.activeVehicle\.([A-Za-z0-9_.]+)\.value\s*\}"#));
static MODE: LazyLock<Regex> = LazyLock::new(|| regex(r"tuningMode:\s*Vehicle\.(Mode\w+)"));
static SECONDS: LazyLock<Regex> = LazyLock::new(|| regex(r"chartDisplaySec:\s*([0-9.]+)"));
static EXTRA: LazyLock<Regex> = LazyLock::new(|| regex(r#"getParameterFact\(-1,\s*"([A-Z0-9_]+)""#));

fn source(file: &str) -> Option<&'static str> {
    PAGES.iter().find(|(name, _)| *name == file).map(|(_, text)| *text)
}

fn uncommented(text: &str) -> String {
    text.lines().filter(|line| !line.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n")
}

fn quoted(block: &str, key: &str) -> String {
    regex(&format!(r#"{key}:\s*qsTr\("((?:[^"\\]|\\.)*)"\)"#)).captures(block).map(|c| c[1].replace("\\\"", "\"")).unwrap_or_default()
}

fn number(block: &str, key: &str) -> Option<f64> {
    regex(&format!(r"\b{key}:\s*(-?[0-9.]+)")).captures(block).and_then(|c| c[1].parse().ok())
}

fn axis_block<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let header = format!("property var {name}: QtObject");
    let start = text.find(&header)? + header.len();
    let rest = &text[start..];
    let end = rest.find(": QtObject").and_then(|next| rest[..next].rfind("property var ")).unwrap_or(rest.len());
    Some(&rest[..end])
}

fn page_of(tab: &str, file: &str) -> Option<Page> {
    let text = uncommented(source(file)?);
    let order: Vec<String> = AXIS_ORDER.captures(&text)?[1].split(',').map(|name| name.trim().to_string()).filter(|name| !name.is_empty()).collect();
    let axes = order
        .iter()
        .filter_map(|name| {
            let block = axis_block(&text, name)?;
            let params = ELEMENT
                .captures_iter(block)
                .filter_map(|element| {
                    let body = &element[1];
                    let param = regex(r#"param:\s*"([^"]+)""#).captures(body)?[1].to_string();
                    Some(Param { title: quoted(body, "title"), description: quoted(body, "description"), param, min: number(body, "min")?, max: number(body, "max")?, step: number(body, "step")? })
                })
                .collect();
            let plot = PLOT.captures_iter(block).map(|c| Plot { name: c[1].to_string(), path: format!("vehicle.{}", &c[2]) }).collect();
            Some(Axis { name: quoted(block, "property string name"), plot_title: quoted(block, "property string plotTitle"), plot, params })
        })
        .collect();
    Some(Page {
        tab: tab.to_string(),
        title: TITLE.captures(&text).or_else(|| PLAIN_TITLE.captures(&text)).map(|c| c[1].to_string()).unwrap_or_default(),
        unit: UNIT.captures(&text).map(|c| c[1].to_string()).unwrap_or_default(),
        extras: EXTRA.captures_iter(&text).map(|c| c[1].to_string()).collect(),
        auto_mode_change: regex(r"showAutoModeChange:\s*true").is_match(&text),
        auto_tuning: regex(r"showAutoTuning:\s*true").is_match(&text),
        tuning_mode: MODE.captures(&text).and_then(|c| TUNING_MODES.iter().position(|m| *m == &c[1])).map_or(0, |i| i as i64),
        chart_seconds: SECONDS.captures(&text).and_then(|c| c[1].parse().ok()).unwrap_or(DEFAULT_CHART_SECONDS),
        axes,
    })
}

pub fn pages(file: &str) -> Vec<Page> {
    let text = source(file).map(uncommented).unwrap_or_default();
    TAB.captures_iter(&text)
        .flat_map(|tab| {
            let (name, page) = (tab[1].to_string(), tab[2].to_string());
            match page.ends_with("All.qml") {
                true => pages(&page),
                false => page_of(&name, &page).into_iter().collect(),
            }
        })
        .collect()
}

const CONTAINERS: [&str; 4] = ["PX4TuningComponentCopterAll.qml", "PX4TuningComponentPlaneAll.qml", "PX4TuningComponentSpacecraftAll.qml", "PX4TuningComponentVTOL.qml"];

const APM_ADVANCED: &str = "APMAdvancedTuningCopterComponent.qml";

static APM_ADVANCED_PAGES: LazyLock<Vec<Page>> = LazyLock::new(|| page_of("", APM_ADVANCED).map(|page| Page { tab: page.title.clone(), ..page }).into_iter().collect());

static PARSED: LazyLock<Vec<(&'static str, Vec<Page>)>> = LazyLock::new(|| CONTAINERS.iter().map(|file| (*file, pages(file))).collect());

fn parsed(container: &str) -> &'static [Page] {
    PARSED.iter().find(|(file, _)| *file == container).map_or(&[], |(_, pages)| pages.as_slice())
}

pub fn container_for(vehicle_type: i64) -> Option<&'static str> {
    match vehicle_type {
        1 => Some("PX4TuningComponentPlaneAll.qml"),
        2 | 3 | 4 | 13 | 14 | 15 => Some("PX4TuningComponentCopterAll.qml"),
        19..=25 => Some("PX4TuningComponentVTOL.qml"),
        45 => Some("PX4TuningComponentSpacecraftAll.qml"),
        _ => None,
    }
}

fn parameter_path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter(-1,{name})")
}

fn control(backend: &dyn Backend, name: &str) -> Option<Value> {
    let path = parameter_path(name);
    let fact = object(&backend.get(&path));
    (fact.get("kind").and_then(Value::as_str) == Some("fact")).then(|| decode(&fact, &path))
}

fn vehicle_pages(backend: &dyn Backend) -> &'static [Page] {
    let vehicle = object(&backend.get_fields("vehicle", "vehicleTypeString,px4Firmware,apmFirmware,multiRotor"));
    let named = vehicle.get("vehicleTypeString").and_then(Value::as_str).unwrap_or("");
    let vehicle_type = (0..=u8::MAX).find(|t| !named.is_empty() && crate::vehiclefacade::mav_type_text(*t) == named).map_or(0, i64::from);
    match (crate::read::flag(&vehicle, "px4Firmware"), crate::read::flag(&vehicle, "apmFirmware") && crate::read::flag(&vehicle, "multiRotor")) {
        (true, _) => container_for(vehicle_type).map_or(&[], parsed),
        (false, true) => APM_ADVANCED_PAGES.as_slice(),
        _ => &[],
    }
}

pub fn opening_tab_params(backend: &dyn Backend) -> Vec<String> {
    vehicle_pages(backend).first().map(|page| page.axes.iter().flat_map(|axis| axis.params.iter().map(|p| p.param.clone())).collect()).unwrap_or_default()
}

pub fn tuning_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let pages = vehicle_pages(backend);
    let tabs: Vec<Value> = pages
        .iter()
        .map(|page| {
            json!({
                "name": page.tab,
                "title": page.title,
                "unit": page.unit,
                "tuningMode": page.tuning_mode,
                "autoModeChange": page.auto_mode_change,
                "autoTuning": page.auto_tuning,
                "chartSeconds": page.chart_seconds,
                "extras": page.extras.iter().filter_map(|name| control(backend, name)).collect::<Vec<_>>(),
                "axes": page.axes.iter().map(|axis| json!({
                    "name": axis.name,
                    "chartTitle": format!("{} {}", if axis.plot_title.is_empty() { &axis.name } else { &axis.plot_title }, page.title),
                    "plot": axis.plot.iter().map(|p| json!({ "name": p.name, "path": p.path })).collect::<Vec<_>>(),
                    "params": axis.params.iter().filter_map(|p| control(backend, &p.param).map(|fact| json!({
                        "title": p.title,
                        "description": p.description,
                        "param": p.param,
                        "min": p.min,
                        "max": p.max,
                        "step": p.step,
                        "fact": fact,
                    }))).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let modes = object(&backend.get_fields("vehicle", "pauseFlightMode,stabilizedFlightMode"));
    let mode = |key: &str| modes.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    json!({
        "kind": "object",
        "class": "Px4Tuning",
        "available": !tabs.is_empty(),
        "stabilizedFlightMode": mode("stabilizedFlightMode"),
        "pauseFlightMode": mode("pauseFlightMode"),
        "tabs": tabs,
    })
}

pub fn set_telemetry_mode(backend: &dyn Backend, args: &str) -> Value {
    let Some(mode) = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_i64()).filter(|m| (0..TUNING_MODES.len() as i64).contains(m)) else {
        return json!({ "ok": false, "reason": "The PID tuning telemetry mode is 0 to 3." });
    };
    crate::guided::dispatch(backend, Some(json!({ "action": "pidTuningMode", "mode": mode })), crate::guided::active_id(backend), SET_TELEMETRY_MODE, &json!([mode]).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_copter_pages_read_out_of_qgcs_own_qml() {
        let copter = pages("PX4TuningComponentCopterAll.qml");
        assert_eq!(copter.iter().map(|p| p.tab.as_str()).collect::<Vec<_>>(), ["Rate Controller", "Attitude Controller", "Velocity Controller", "Position Controller"]);
        let rate = &copter[0];
        assert_eq!((rate.title.as_str(), rate.unit.as_str()), ("Rate", "deg/s"));
        assert_eq!(rate.extras, ["MC_AIRMODE", "THR_MDL_FAC"]);
        assert_eq!(rate.axes.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["Roll", "Pitch", "Yaw"]);
        assert_eq!(rate.axes[0].params[0], Param {
            title: "Overall Multiplier (MC_ROLLRATE_K)".into(),
            description: "Multiplier for P, I and D gains: increase for more responsiveness, reduce if the rates overshoot (and increasing D does not help).".into(),
            param: "MC_ROLLRATE_K".into(),
            min: 0.3,
            max: 3.0,
            step: 0.05,
        });
        assert_eq!(rate.axes.iter().map(|a| a.params.len()).collect::<Vec<_>>(), [3, 3, 2], "each axis holds only its own gains; yaw has no D");
        assert_eq!(rate.tuning_mode, 1);
        assert!(rate.auto_mode_change && rate.auto_tuning, "the rate tab offers mode switching and autotune");
        assert!(!copter[2].auto_tuning);
        assert_eq!(rate.chart_seconds, 3.0);
        assert_eq!(rate.axes[0].plot, [Plot { name: "Response".into(), path: "vehicle.rollRate".into() }, Plot { name: "Setpoint".into(), path: "vehicle.setpoint.rollRate".into() }]);
        assert_eq!(copter[2].tuning_mode, 2, "velocity tunes with the velocity and position streams");
        assert!(copter.iter().all(|page| page.axes.iter().all(|axis| axis.plot.len() == 2)), "every axis plots a response against its setpoint");
        assert!(copter.iter().all(|page| !page.axes.is_empty() && page.axes.iter().all(|axis| !axis.params.is_empty())), "every tab parses to axes with sliders: {copter:#?}");
    }

    #[test]
    fn every_vehicle_class_resolves_to_a_tab_set_and_vtol_uses_the_copter_tabs() {
        [1, 2, 22, 45].iter().for_each(|t| assert!(!pages(container_for(*t).unwrap()).is_empty(), "type {t}"));
        assert_eq!(pages("PX4TuningComponentVTOL.qml"), pages("PX4TuningComponentCopterAll.qml"), "VTOL's only live tab is the copter set; the fixed-wing one is commented out");
        assert_eq!(pages("PX4TuningComponentPlaneAll.qml")[1].axes.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["Roll", "Pitch"]);
        assert_eq!(container_for(10), None);
        let named: Vec<&str> = [1u8, 2, 22, 45].iter().map(|t| crate::vehiclefacade::mav_type_text(*t)).collect();
        assert!(named.iter().all(|n| !n.is_empty()) && named.iter().enumerate().all(|(i, n)| named.iter().skip(i + 1).all(|m| m != n)), "the type string names one MAV_TYPE, so it finds the tab set QGC's switch picks: {named:?}");
    }

    #[test]
    fn ardupilot_advanced_tuning_is_one_rate_page_of_three_axes() {
        let page = APM_ADVANCED_PAGES.first().expect("the APM advanced tuning QML parses");
        assert_eq!((page.tab.as_str(), page.unit.as_str(), page.chart_seconds), ("Rate", "deg/s", 3.0));
        assert_eq!(page.axes.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["Roll", "Pitch", "Yaw"]);
        assert_eq!(page.axes.iter().map(|a| a.params.len()).collect::<Vec<_>>(), [4, 4, 3]);
        assert_eq!(page.axes[0].params[0].param, "ATC_ANG_RLL_P");
        assert_eq!(page.axes[0].plot.len(), 2);
    }
}
