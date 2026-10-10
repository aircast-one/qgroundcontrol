use serde_json::Value;

use crate::cameras::Camera;
use crate::linkconfig::{Kind, LinkConfig};

pub const STREAM_CONFIG: &str = "/api/stream/config";
pub const TELEMETRY_CONFIG: &str = "/api/telemetry/config";
pub const WATCH_VIA: &str = "/api/watch/via";
const VIDEO_SOURCE_WEBRTC: &str = "WebRTC (WHEP) Video Stream";
const VIDEO_SOURCE_RTSP: &str = "RTSP Video Stream";
const CLOUDFLARE: &str = "cloudflare";

pub fn bare_host(host: &str) -> &str {
    host.split(':').next().unwrap_or(host)
}

pub fn device_cameras(host: &str, config: &Value, via: &Value) -> Vec<Camera> {
    let bare = bare_host(host);
    let cams: Vec<&String> = config.get("paths").and_then(Value::as_object).map(|paths| paths.iter().filter(|(_, p)| p.get("source").and_then(Value::as_str).is_some_and(|s| !s.is_empty())).map(|(name, _)| name).collect()).unwrap_or_default();
    cams.iter()
        .map(|cam| {
            let name = format!("{cam} ({bare})");
            match via.get(cam.as_str()).and_then(Value::as_str) == Some(CLOUDFLARE) {
                true => Camera::new(&name, VIDEO_SOURCE_WEBRTC, &format!("http://{host}/whep/{CLOUDFLARE}/{cam}")),
                false => Camera::new(&name, VIDEO_SOURCE_RTSP, &format!("rtsp://{bare}:8554/{cam}")),
            }
        })
        .collect()
}

pub fn telemetry_link(host: &str, config: &Value) -> Option<LinkConfig> {
    let bare = bare_host(host);
    let endpoints: Vec<&str> = config.get("endpoints").and_then(Value::as_array).map(|e| e.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let port = |scheme: &str| endpoints.iter().filter(|spec| spec.starts_with(&format!("{scheme}:"))).find_map(|spec| spec.rsplit(':').next()?.parse::<u16>().ok().filter(|p| *p > 0));
    let kind = match (port("udps"), port("tcps")) {
        (Some(udp), _) => Kind::Udp { local_port: 0, hosts: vec![(bare.to_string(), udp)] },
        (None, Some(tcp)) => Kind::Tcp { host: bare.to_string(), port: tcp },
        (None, None) => return None,
    };
    Some(LinkConfig { name: format!("Aircast {bare}"), auto_connect: true, high_latency: false, kind })
}

pub fn cloud_link(host: &str, config: &Value) -> Option<(String, LinkConfig)> {
    let cloud = config.get("cloud")?;
    let text = |key: &str| cloud.get(key).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let (api_base, device_id) = (text("api")?, text("deviceId")?);
    let link = LinkConfig { name: format!("Aircast {} (cloud)", bare_host(host)), auto_connect: true, high_latency: false, kind: Kind::AircastCloud { api_base: api_base.clone(), device_id } };
    Some((api_base, link))
}

pub fn fetch(host: &str, path: &str) -> Option<Value> {
    let request = ureq::get(&format!("http://{host}{path}")).config().timeout_global(Some(std::time::Duration::from_secs(10))).build();
    serde_json::from_str(&request.call().ok()?.body_mut().read_to_string().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_device_camera_becomes_a_camera_of_the_same_kind() {
        let config = json!({ "paths": { "front": { "source": "rtsp://cam" }, "rear": { "source": "rtsp://cam2" }, "idle": { "source": "" } } });
        let direct = device_cameras("10.0.0.5:8080", &config, &json!({}));
        assert_eq!(direct, vec![Camera::new("front (10.0.0.5)", VIDEO_SOURCE_RTSP, "rtsp://10.0.0.5:8554/front"), Camera::new("rear (10.0.0.5)", VIDEO_SOURCE_RTSP, "rtsp://10.0.0.5:8554/rear")], "no camera is second class: each one is reached the same way");
        let cloud = device_cameras("10.0.0.5:8080", &config, &json!({ "front": "cloudflare" }));
        assert_eq!(cloud[0], Camera::new("front (10.0.0.5)", VIDEO_SOURCE_WEBRTC, "http://10.0.0.5:8080/whep/cloudflare/front"), "the cloudflare path goes through the device's own port");
        assert_eq!(cloud[1].source, VIDEO_SOURCE_RTSP);
        assert!(device_cameras("h", &json!({ "paths": {} }), &json!({})).is_empty(), "no camera, nothing added");
    }

    #[test]
    fn a_device_with_a_cloud_account_names_its_backup_link() {
        let (api, link) = cloud_link("10.0.0.5:8080", &json!({ "cloud": { "api": " https://api.aircast.one ", "deviceId": "d-7", "sfu": "x" } })).unwrap();
        assert_eq!((api.as_str(), link.name.as_str(), link.auto_connect), ("https://api.aircast.one", "Aircast 10.0.0.5 (cloud)", true));
        assert_eq!(link.kind, Kind::AircastCloud { api_base: "https://api.aircast.one".into(), device_id: "d-7".into() });
        assert_eq!(cloud_link("h", &json!({ "cloud": { "api": "https://a" } })), None, "no device id, no cloud link");
    }

    #[test]
    fn telemetry_prefers_the_udp_server_endpoint() {
        let both = json!({ "endpoints": ["udps:0.0.0.0:14550", "tcps:0.0.0.0:5760", "udp:1.2.3.4:9"] });
        assert_eq!(telemetry_link("192.168.1.2:80", &both).unwrap().kind, Kind::Udp { local_port: 0, hosts: vec![("192.168.1.2".into(), 14550)] });
        let tcp = telemetry_link("192.168.1.2", &json!({ "endpoints": ["tcps:0.0.0.0:5760"] })).unwrap();
        assert_eq!((tcp.name.as_str(), tcp.auto_connect, tcp.kind), ("Aircast 192.168.1.2", true, Kind::Tcp { host: "192.168.1.2".into(), port: 5760 }));
        assert_eq!(telemetry_link("h", &json!({ "endpoints": ["udp:1.2.3.4:9"] })), None);
    }
}
