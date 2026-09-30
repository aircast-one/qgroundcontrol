use serde_json::{Value, json};

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

pub fn camera_writes(host: &str, config: &Value, via: &Value) -> Vec<(&'static str, String)> {
    let bare = bare_host(host);
    let cams: Vec<&String> = config.get("paths").and_then(Value::as_object).map(|paths| paths.iter().filter(|(_, p)| p.get("source").and_then(Value::as_str).is_some_and(|s| !s.is_empty())).map(|(name, _)| name).collect()).unwrap_or_default();
    let Some(primary) = cams.first() else { return Vec::new() };
    let cloudflare = |cam: &str| via.get(cam).and_then(Value::as_str) == Some(CLOUDFLARE);
    let whep = |cam: &str| if cloudflare(cam) { format!("http://{host}/whep/{CLOUDFLARE}/{cam}") } else { format!("http://{bare}:8889/{cam}/whep") };
    let source = match cloudflare(primary) {
        true => vec![("settings.videoSettings.whepUrl", whep(primary)), ("settings.videoSettings.videoSource", VIDEO_SOURCE_WEBRTC.to_string())],
        false => vec![("settings.videoSettings.rtspUrl", format!("rtsp://{bare}:8554/{primary}")), ("settings.videoSettings.videoSource", VIDEO_SOURCE_RTSP.to_string())],
    };
    let extras: Vec<Value> = cams.iter().skip(1).map(|cam| json!({ "name": format!("{cam} ({bare})"), "source": VIDEO_SOURCE_WEBRTC, "url": whep(cam) })).collect();
    source
        .into_iter()
        .chain([("settings.videoSettings.primaryCameraName", format!("{primary} ({bare})")), ("settings.videoSettings.extraVideoSources", Value::Array(extras).to_string())])
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

pub fn fetch(host: &str, path: &str) -> Option<Value> {
    serde_json::from_str(&ureq::get(&format!("http://{host}{path}")).call().ok()?.body_mut().read_to_string().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cameras_follow_apply_device_cameras() {
        let config = json!({ "paths": { "front": { "source": "rtsp://cam" }, "rear": { "source": "rtsp://cam2" }, "idle": { "source": "" } } });
        let direct = camera_writes("10.0.0.5:8080", &config, &json!({}));
        assert_eq!(direct[0], ("settings.videoSettings.rtspUrl", "rtsp://10.0.0.5:8554/front".to_string()));
        assert_eq!(direct[1].1, VIDEO_SOURCE_RTSP);
        assert_eq!(direct[2], ("settings.videoSettings.primaryCameraName", "front (10.0.0.5)".to_string()));
        assert_eq!(serde_json::from_str::<Value>(&direct[3].1).unwrap(), json!([{ "name": "rear (10.0.0.5)", "source": VIDEO_SOURCE_WEBRTC, "url": "http://10.0.0.5:8889/rear/whep" }]));
        let cloud = camera_writes("10.0.0.5:8080", &config, &json!({ "front": "cloudflare" }));
        assert_eq!(cloud[0], ("settings.videoSettings.whepUrl", "http://10.0.0.5:8080/whep/cloudflare/front".to_string()), "the cloudflare path goes through the device's own port");
        assert!(camera_writes("h", &json!({ "paths": {} }), &json!({})).is_empty(), "no camera, nothing written");
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
