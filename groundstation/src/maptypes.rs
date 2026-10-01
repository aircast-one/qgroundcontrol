use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &["settings.flightMapSettings.mapProvider", "settings.flightMapSettings.mapType"];

pub const QGC_ORDER: [&str; 40] = [
    "Google Street Map", "Google Satellite", "Google Terrain", "Google Hybrid", "Google Labels",
    "Bing Road", "Bing Satellite", "Bing Hybrid",
    "TianDiTu Road", "TianDiTu Satellite",
    "Statkart Topo", "Statkart Basemap", "Svalbard Topo",
    "Eniro Topo",
    "Esri World Street", "Esri World Satellite", "Esri Terrain",
    "Mapbox Streets", "Mapbox Light", "Mapbox Dark", "Mapbox Satellite", "Mapbox Hybrid", "Mapbox StreetsBasic", "Mapbox Outdoors", "Mapbox Bright", "Mapbox Custom",
    "MapQuest Map", "MapQuest Sat",
    "VWorld Street Map", "VWorld Satellite Map",
    "Japan-GSI Contour", "Japan-GSI Seamless", "Japan-GSI Anaglyph", "Japan-GSI Slope", "Japan-GSI Relief",
    "LINZ Basemap",
    "Street Map",
    "OpenAIP",
    "CustomURL Custom",
    "Copernicus",
];

pub const ELEVATION_PROVIDERS: &[&str] = &["Copernicus"];

pub fn map_provider_list() -> Vec<String> {
    QGC_ORDER
        .iter()
        .filter(|name| !ELEVATION_PROVIDERS.contains(name))
        .map(|name| name.split_once(' ').map_or(name.to_string(), |(provider, _)| provider.to_string()))
        .fold(Vec::new(), |seen, provider| if seen.contains(&provider) { seen } else { [seen, vec![provider]].concat() })
}

pub fn map_type_list(provider: &str) -> Vec<String> {
    let matches = |name: &str| regex::Regex::new(provider).map_or_else(|_| name.contains(provider), |pattern| pattern.is_match(name));
    QGC_ORDER
        .iter()
        .filter(|name| matches(name))
        .map(|name| name.split_once(' ').map_or(name.to_string(), |(_, kind)| kind.to_string()))
        .fold(Vec::new(), |seen, kind| if seen.contains(&kind) { seen } else { [seen, vec![kind]].concat() })
}

fn setting(backend: &dyn Backend, name: &str) -> String {
    crate::read::object(&backend.get(&format!("settings.flightMapSettings.{name}.rawValue"))).get("value").and_then(Value::as_str).unwrap_or_default().to_string()
}

pub fn map_types_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let provider = setting(backend, "mapProvider");
    json!({
        "kind": "object",
        "class": "MapTypes",
        "provider": provider,
        "current": setting(backend, "mapType"),
        "types": map_type_list(&provider),
        "path": "settings.flightMapSettings.mapType.rawValue",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_providers_types_come_in_qgcs_order_without_the_provider_prefix() {
        assert_eq!(map_type_list("Google"), ["Street Map", "Satellite", "Terrain", "Hybrid", "Labels"]);
        assert_eq!(map_type_list("Bing"), ["Road", "Satellite", "Hybrid"]);
        assert_eq!(map_type_list("Mapbox").len(), 9);
        assert_eq!(map_type_list("Japan-GSI")[0], "Contour");
    }

    #[test]
    fn the_provider_is_a_regex_filter_as_in_map_engine_manager() {
        assert_eq!(map_type_list("Street"), ["Street Map", "World Street", "Streets", "StreetsBasic", "Map"], "QStringList::filter(QRegularExpression) is a substring match over every full name");
        assert!(map_type_list("Nope").is_empty());
        assert_eq!(map_type_list("OpenAIP"), ["OpenAIP"], "a name without a space is its own type");
    }

    #[test]
    fn providers_are_the_first_word_of_each_map_without_the_elevation_ones() {
        let providers = map_provider_list();
        assert_eq!(&providers[..3], ["Google", "Bing", "TianDiTu"]);
        assert!(providers.contains(&"Street".to_string()) && providers.contains(&"OpenAIP".to_string()), "QGCMapEngineManager::mapProviderList keeps the part before the first space");
        assert!(!providers.contains(&"Copernicus".to_string()));
    }

    #[test]
    fn the_order_names_every_core_provider() {
        let mut ours: Vec<&str> = QGC_ORDER.to_vec();
        ours.sort_unstable();
        let mut known: Vec<&str> = crate::tilecache::PROVIDERS.iter().map(|(name, _)| *name).collect();
        known.sort_unstable();
        assert_eq!(ours, known);
    }
}
