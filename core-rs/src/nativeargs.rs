const DEFAULT_ORGANIZATION: &str = match option_env!("QGC_ORG_DOMAIN") {
    Some(domain) => domain,
    None => "aircast.one",
};
const DEFAULT_APPLICATION: &str = match option_env!("QGC_APP_NAME") {
    Some(name) => name,
    None => "Aircast QGC",
};
const DEFAULT_ORGANIZATION_NAME: &str = match option_env!("QGC_ORG_NAME") {
    Some(name) => name,
    None => "Aircast",
};
pub const VIDEO_SOURCE_WEBRTC: &str = "WebRTC (WHEP) Video Stream";
pub const VIDEO_SOURCE_RTSP: &str = "RTSP Video Stream";
const DEEP_LINK_SCHEME: &str = "aircast-qgc";

pub struct Options {
    pub settings: std::path::PathBuf,
    pub map_cache: Option<String>,
    pub application: String,
    pub position_source: String,
    pub autoconnect: bool,
    pub debug_port: Option<u16>,
}

fn default_settings_path(application: &str) -> std::path::PathBuf {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    home.join(".config").join(DEFAULT_ORGANIZATION).join(format!("{application}.ini"))
}

fn default_map_cache(application: &str) -> Option<String> {
    let home = std::path::PathBuf::from(std::env::var_os("HOME")?);
    cfg!(target_os = "macos").then(|| home.join("Library/Caches").join(DEFAULT_ORGANIZATION_NAME).join(application).join("QGCMapCache/qgcMapCache.db").to_string_lossy().into_owned())
}

pub fn options(arguments: &[String]) -> Options {
    let option = |name: &str| arguments.iter().position(|a| a == name).and_then(|i| arguments.get(i + 1)).cloned();
    let application = option("--app-name").unwrap_or_else(|| DEFAULT_APPLICATION.to_string());
    Options {
        settings: option("--settings").map_or_else(|| default_settings_path(&application), std::path::PathBuf::from),
        map_cache: option("--map-cache").or_else(|| default_map_cache(&application)),
        position_source: option("--position-source").unwrap_or_else(|| "none".to_string()),
        autoconnect: !arguments.iter().any(|a| a == "--no-autoconnect"),
        debug_port: option("--port").or_else(|| std::env::var("QGC_DEBUG_API_PORT").ok()).and_then(|p| p.parse().ok()),
        application,
    }
}

pub fn deep_link_writes(link: &str) -> Option<(Option<u16>, Vec<(&'static str, String)>)> {
    let parsed = url::Url::parse(link).ok().filter(|u| u.scheme() == DEEP_LINK_SCHEME)?;
    let query = |key: &str| parsed.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned()).filter(|v| !v.is_empty());
    let debug = query("debug").and_then(|p| p.parse::<u16>().ok()).filter(|p| *p != 0);
    let source = match (query("whep"), query("rtsp")) {
        (Some(whep), _) => vec![("settings.videoSettings.whepUrl", whep), ("settings.videoSettings.videoSource", VIDEO_SOURCE_WEBRTC.to_string())],
        (None, Some(rtsp)) => vec![("settings.videoSettings.rtspUrl", rtsp), ("settings.videoSettings.videoSource", VIDEO_SOURCE_RTSP.to_string())],
        (None, None) => return Some((debug, Vec::new())),
    };
    let named = query("name").map(|name| ("settings.videoSettings.primaryCameraName", name));
    Some((debug, source.into_iter().chain(named).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deep_link_sets_the_video_source_and_the_debug_port_as_qgc_application_does() {
        let (debug, writes) = deep_link_writes("aircast-qgc://open?whep=https%3A%2F%2Fcam%2Fwhep&name=Front&debug=8777").unwrap();
        assert_eq!(debug, Some(8777));
        assert_eq!(writes, vec![("settings.videoSettings.whepUrl", "https://cam/whep".to_string()), ("settings.videoSettings.videoSource", VIDEO_SOURCE_WEBRTC.to_string()), ("settings.videoSettings.primaryCameraName", "Front".to_string())]);
        assert_eq!(deep_link_writes("aircast-qgc://open?rtsp=rtsp%3A%2F%2Fh%3A8554%2Flive").unwrap().1[1].1, VIDEO_SOURCE_RTSP);
        assert_eq!(deep_link_writes("https://example.com/?whep=x"), None, "only the app's own scheme is honoured");
        assert_eq!(deep_link_writes("aircast-qgc://open?debug=0"), Some((None, Vec::new())));
    }

    #[test]
    fn the_settings_file_defaults_to_where_qsettings_keeps_it() {
        let chosen = options(&["app".to_string(), "--app-name".to_string(), "Aircast QGC Daily".to_string()]);
        assert!(chosen.settings.ends_with(".config/aircast.one/Aircast QGC Daily.ini"));
        assert!(chosen.autoconnect && chosen.debug_port.is_none() || std::env::var("QGC_DEBUG_API_PORT").is_ok());
        assert!(!options(&["app".to_string(), "--no-autoconnect".to_string()]).autoconnect);
        if cfg!(target_os = "macos") {
            assert!(chosen.map_cache.is_some_and(|p| p.ends_with("Library/Caches/Aircast/Aircast QGC Daily/QGCMapCache/qgcMapCache.db")), "QStandardPaths::CacheLocation is the organisation name, then the application");
        }
    }
}
