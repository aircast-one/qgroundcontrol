use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use base64::Engine;
use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady", "vehicle.sub", "vehicle.apmFirmware", "vehicle.firmwareMajorVersion", "vehicle.firmwareMinorVersion", "vehicle.firmwarePatchVersion"];
pub const SUB_FRAME_SCREEN: &str = "apmSubFrame";
pub const SET_FRAME: &str = "apmSubFrame.set";
pub const LOAD_DEFAULTS: &str = "apmSubFrame.loadDefaults";
const FRAME_CONFIG: &str = "FRAME_CONFIG";
const GITHUB_PARAMS: &str = "https://api.github.com/repos/ArduPilot/ardupilot/contents/Tools/Frame_params";
const PARAMS_TIMEOUT: Duration = Duration::from_secs(20);

pub struct Frame {
    pub name: &'static str,
    pub value: i64,
    pub param_file: Option<&'static str>,
    image: &'static [u8],
}

pub const FRAMES: [Frame; 6] = [
    Frame { name: "BlueROV1", value: 0, param_file: None, image: include_bytes!("../../src/AutoPilotPlugins/APM/Images/bluerov-frame.png") },
    Frame { name: "BlueROV2/Vectored", value: 1, param_file: Some("bluerov2"), image: include_bytes!("../../src/AutoPilotPlugins/APM/Images/vectored-frame.png") },
    Frame { name: "Vectored-6DOF", value: 2, param_file: Some("bluerov2-heavy"), image: include_bytes!("../../src/AutoPilotPlugins/APM/Images/vectored6dof-frame.png") },
    Frame { name: "SimpleROV-3", value: 4, param_file: None, image: include_bytes!("../../src/AutoPilotPlugins/APM/Images/simple3-frame.png") },
    Frame { name: "SimpleROV-4", value: 5, param_file: None, image: include_bytes!("../../src/AutoPilotPlugins/APM/Images/simple4-frame.png") },
    Frame { name: "SimpleROV-5", value: 6, param_file: None, image: include_bytes!("../../src/AutoPilotPlugins/APM/Images/simple5-frame.png") },
];

#[derive(Default)]
struct Loading {
    busy: bool,
    error: String,
}

static LOADING: Mutex<Loading> = Mutex::new(Loading { busy: false, error: String::new() });

type Version = (i64, i64, i64);

fn at_least(version: Version, wanted: Version) -> bool {
    version >= wanted
}

pub fn params_version(version: Version) -> &'static str {
    match () {
        _ if at_least(version, (4, 0, 0)) => "4_0_0",
        _ if at_least(version, (3, 5, 4)) => "3_5_4",
        _ if at_least(version, (3, 5, 2)) => "3_5_2",
        _ => "3_5",
    }
}

pub fn has_sensible_defaults(version: Version) -> bool {
    at_least(version, (4, 1, 0))
}

fn param_path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter(-1,{name})")
}

fn version(backend: &dyn Backend) -> Version {
    let fields = object(&backend.get_fields("vehicle", "firmwareMajorVersion,firmwareMinorVersion,firmwarePatchVersion"));
    let part = |key: &str| fields.get(key).and_then(Value::as_i64).unwrap_or(-1);
    (part("firmwareMajorVersion"), part("firmwareMinorVersion"), part("firmwarePatchVersion"))
}

pub fn apm_sub_frame_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "sub,apmFirmware"));
    let config = object(&backend.get(&param_path(FRAME_CONFIG)));
    let present = config.get("kind").and_then(Value::as_str) == Some("fact") && config.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
    if !(present && flag(&vehicle, "sub") && flag(&vehicle, "apmFirmware")) {
        return json!({ "kind": "object", "class": "ApmSubFrame", "available": false });
    }
    let selected = config.get("rawValue").or(config.get("value")).and_then(Value::as_f64).map(|v| v as i64);
    let loading = LOADING.lock().unwrap_or_else(PoisonError::into_inner);
    json!({
        "kind": "object",
        "class": "ApmSubFrame",
        "available": true,
        "selected": selected,
        "confirmFirst": !has_sensible_defaults(version(backend)),
        "loadingDefaults": loading.busy,
        "loadError": loading.error,
        "frames": FRAMES.iter().map(|f| json!({ "name": f.name, "value": f.value, "hasDefaults": f.param_file.is_some() })).collect::<Vec<_>>(),
    })
}

pub fn apm_sub_frame_image_view(_backend: &dyn Backend, args: &[String]) -> Value {
    match args.first().and_then(|a| a.parse::<i64>().ok()).and_then(|value| FRAMES.iter().find(|f| f.value == value)) {
        Some(frame) => json!({ "kind": "object", "class": "ApmSubFrameImage", "png": base64::engine::general_purpose::STANDARD.encode(frame.image) }),
        None => crate::read::refused("this needs a FRAME_CONFIG value"),
    }
}

pub fn parse_params(text: &str) -> Vec<(String, f64)> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut parts = line.split(',');
            let name = parts.next()?.trim().to_string();
            let value = parts.next()?.trim().parse::<f64>().ok()?;
            Some((name, value))
        })
        .collect()
}

fn fetch_params(file: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(PARAMS_TIMEOUT)).build().into();
    let listing: Value = agent
        .get(&format!("{GITHUB_PARAMS}/{file}?ref=master"))
        .call()
        .and_then(|mut r| r.body_mut().read_to_string())
        .map_err(|e| format!("Param file github json download failed: {e}"))
        .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("Unable to open json document: {e}")))?;
    let url = listing.get("download_url").and_then(Value::as_str).ok_or("Param file github json download failed: no download_url")?;
    agent.get(url).call().and_then(|mut r| r.body_mut().read_to_string()).map_err(|e| format!("Param file download failed: {e}"))
}

fn apply(params: &[(String, f64)]) {
    let now = crate::hub::now_ms();
    let frames: Vec<(u32, Vec<u8>)> = {
        let mut hub = crate::hub::lock();
        let Some((id, component)) = hub.active().map(|v| (v.id, v.component)) else { return };
        let known: Vec<&(String, f64)> = params.iter().filter(|(name, _)| hub.active().is_some_and(|v| v.parameter(component, name).is_some())).collect();
        let written: Vec<(u32, Vec<u8>)> = known
            .into_iter()
            .flat_map(|(name, value)| hub.parameter_request(Some(id), &json!({ "name": name, "value": value }), now).unwrap_or_default())
            .collect();
        let refreshed = hub.guided(Some(id), &json!({ "action": "refreshParameters" }), now).unwrap_or_default();
        written.into_iter().chain(refreshed).collect()
    };
    frames.iter().for_each(|(link, bytes)| {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
    });
}

fn set_frame(backend: &dyn Backend, value: i64) -> Value {
    object(&backend.set(&format!("{}.rawValue", param_path(FRAME_CONFIG)), &json!({ "value": value }).to_string()))
}

pub fn run(backend: &dyn Backend, action: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let Some(frame) = given.first().and_then(Value::as_i64).and_then(|value| FRAMES.iter().find(|f| f.value == value)) else {
        return json!({ "ok": false, "reason": "Pick one of the listed frames." });
    };
    match action {
        SET_FRAME => set_frame(backend, frame.value),
        LOAD_DEFAULTS => {
            let Some(file) = frame.param_file else { return json!({ "ok": false, "reason": "This frame has no default parameter set." }) };
            if !crate::vehiclefacade::switched_on() {
                return json!({ "ok": false, "reason": "Loading frame defaults needs the core vehicle connection." });
            }
            {
                let mut loading = LOADING.lock().unwrap_or_else(PoisonError::into_inner);
                if loading.busy {
                    return json!({ "ok": false, "reason": "The default parameters are still loading." });
                }
                *loading = Loading { busy: true, error: String::new() };
            }
            let written = set_frame(backend, frame.value);
            let path = format!("Sub/{file}-{}.params", params_version(version(backend)));
            std::thread::Builder::new()
                .name("groundstation-sub-frame-params".to_string())
                .spawn(move || {
                    let outcome = fetch_params(&path).map(|text| apply(&parse_params(&text)));
                    *LOADING.lock().unwrap_or_else(PoisonError::into_inner) = Loading { busy: false, error: outcome.err().unwrap_or_default() };
                })
                .expect("sub frame params thread");
            written
        }
        _ => json!({ "ok": false, "reason": format!("{action} is not a Sub frame action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [SET_FRAME, LOAD_DEFAULTS].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parameter_file_and_the_prompt_follow_the_firmware_version() {
        assert_eq!(params_version((4, 0, 5)), "4_0_0");
        assert_eq!(params_version((3, 5, 4)), "3_5_4");
        assert_eq!(params_version((3, 5, 3)), "3_5_2");
        assert_eq!(params_version((-1, -1, -1)), "3_5");
        assert!(has_sensible_defaults((4, 1, 0)));
        assert!(!has_sensible_defaults((4, 0, 9)), "older Sub firmware asks before switching frames");
    }

    #[test]
    fn a_frame_param_file_reads_like_load_parameters_from_download_file() {
        let parsed = parse_params("# BlueROV2\nFRAME_CONFIG,1\n\n  MOT_1_DIRECTION,-1  \nbroken line\nJS_GAIN_DEFAULT,0.5,extra\n");
        assert_eq!(parsed, vec![("FRAME_CONFIG".to_string(), 1.0), ("MOT_1_DIRECTION".to_string(), -1.0), ("JS_GAIN_DEFAULT".to_string(), 0.5)]);
    }

    #[test]
    fn every_frame_carries_its_picture() {
        assert!(FRAMES.iter().all(|f| f.image.starts_with(b"\x89PNG")));
        assert_eq!(FRAMES.iter().filter(|f| f.param_file.is_some()).count(), 2, "only the BlueROV2 frames have a default parameter set");
    }
}
