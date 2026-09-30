use regex::{Captures, Regex};
use std::collections::BTreeSet;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub struct TileRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Keys {
    pub mapbox_token: String,
    pub mapbox_account: String,
    pub mapbox_style: String,
    pub esri_token: String,
    pub custom_url: String,
    pub tianditu_token: String,
    pub openaip_token: String,
    pub vworld_token: String,
    pub language: String,
}

#[cfg(target_os = "macos")]
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.5; rv:125.0) Gecko/20100101 Firefox/125.0";
#[cfg(target_os = "windows")]
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:100.0) Gecko/20100101 Firefox/112.0";
#[cfg(target_os = "android")]
pub const USER_AGENT: &str = "Mozilla/5.0 (Android 13; Tablet; rv:68.0) Gecko/68.0 Firefox/112.0";
#[cfg(target_os = "linux")]
pub const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/112.0";
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "android", target_os = "linux")))]
pub const USER_AGENT: &str = "Qt Location based application";

const GOOGLE: &str = "http://mt%1.google.com/vt/%2=%3&hl=%4&x=%5%6&y=%7&z=%8&s=%9&scale=%10";
const BING: &str = "http://ecn.t%1.tiles.virtualearth.net/tiles/%2%3.%4?g=%5&mkt=%6";
const TIANDITU: &str = "https://t%1.tianditu.gov.cn/DataServer?tk=%2&T=%3&x=%4&y=%5&l=%6";
const STATKART: &str = "https://cache.kartverket.no/v1/wmts/1.0.0/topo/default/webmercator/%1/%2/%3.png";
const SVALBARD: &str = "https://geodata.npolar.no/arcgis/rest/services/Basisdata/NP_Basiskart_Svalbard_WMTS_3857/MapServer/WMTS/tile/1.0.0/Basisdata_NP_Basiskart_Svalbard_WMTS_3857/default/default028mm/%1/%2/%3";
const ENIRO: &str = "http://map.eniro.com/geowebcache/service/tms1.0.0/map/%1/%2/%3.%4";
const ESRI: &str = "http://services.arcgisonline.com/ArcGIS/rest/services/%1/MapServer/tile/%2/%3/%4";
const MAPBOX: &str = "https://api.mapbox.com/styles/v1/mapbox/%1/tiles/%2/%3/%4?access_token=%5";
const MAPBOX_CUSTOM: &str = "https://api.mapbox.com/styles/v1/%1/%2/tiles/256/%3/%4/%5?access_token=%6";
const MAPQUEST: &str = "http://otile%1.mqcdn.com/tiles/1.0.0/%2/%3/%4/%5.%6";
const VWORLD: &str = "http://api.vworld.kr/req/wmts/1.0.0/%1/%2/%3/%4/%5.%6";
const CYBERJAPAN: &str = "https://cyberjapandata.gsi.go.jp/xyz/%1/%2/%3/%4.%5";
const LINZ: &str = "https://basemaps.linz.govt.nz/v1/tiles/aerial/EPSG:3857/%1/%2/%3.%4?api=d01ev80nqcjxddfvc6amyvkk1ka";
const OPENSTREETMAP: &str = "http://tile.openstreetmap.org/%1/%2/%3.png";
const OPENAIP: &str = "https://api.tiles.openaip.net/api/data/openaip/%1/%2/%3.png";
const COPERNICUS: &str = "https://terrain-ce.suite.auterion.com/api/v1/carpet?points=%1,%2,%3,%4";
const COPERNICUS_TILE_DEGREES: f64 = 0.01;

static ESCAPE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%L?([0-9]{1,2})").expect("escape pattern"));
static PARTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)^(?:([A-Za-z][A-Za-z0-9+.\-]*):)?(?://([^/?#]*))?([^?#]*)(?:\?([^#]*))?(?:#(.*))?$").expect("url pattern")
});
static PERCENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%([0-9A-Fa-f]{2})").expect("percent pattern"));

pub fn tile_request(provider: &str, x: i32, y: i32, zoom: i32, keys: &Keys) -> Option<TileRequest> {
    let (referrer, raw) = source(provider, x, y, zoom, keys)?;
    let url = qurl(&raw);
    let token = if provider.starts_with("Esri ") { keys.esri_token.as_str() } else { "" };
    (!url.is_empty()).then(|| TileRequest {
        url,
        headers: [("Accept", "*/*"), ("User-Agent", USER_AGENT), ("Referer", referrer), ("User-Token", token), ("Connection", "keep-alive")]
            .into_iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect(),
    })
}

fn source(provider: &str, x: i32, y: i32, zoom: i32, keys: &Keys) -> Option<(&'static str, String)> {
    let (xs, ys, zs) = (x.to_string(), y.to_string(), zoom.to_string());
    let server = |max: i32| ((x + 2 * y) % max).to_string();
    let google = |request: &str, version: &str| {
        let seclen = (x * 3 + y) % 8;
        let sec2 = usize::try_from(seclen).map_or("Galileo", |len| &"Galileo"[..len.min(7)]);
        let sec1 = if (10000..100000).contains(&y) { "&s=" } else { "" };
        let head = args(&arg(GOOGLE, &server(4)), &[request, version, &keys.language]);
        Some(("https://www.google.com/maps/preview", args(&chain(&head, &[&xs, sec1, &ys, &zs]), &[sec2, "1"])))
    };
    let bing = |code: &str, format: &str| {
        Some(("https://www.bing.com/maps/", args(&arg(BING, &server(4)), &[code, &quadkey(x, y, zoom), format, "2981", &keys.language])))
    };
    let tianditu = |code: &str| {
        (!keys.tianditu_token.is_empty())
            .then(|| ("https://map.tianditu.gov.cn/", chain(TIANDITU, &[&server(8), &keys.tianditu_token, code, &xs, &ys, &zs])))
    };
    let esri = |id: &str| Some(("", chain(ESRI, &[id, &zs, &ys, &xs])));
    let mapbox = |id: &str| {
        (!keys.mapbox_token.is_empty()).then(|| {
            let url = if id == "mapbox.custom" {
                chain(MAPBOX_CUSTOM, &[&keys.mapbox_account, &keys.mapbox_style, &zs, &xs, &ys, &keys.mapbox_token])
            } else {
                chain(MAPBOX, &[id, &zs, &xs, &ys, &keys.mapbox_token])
            };
            ("https://www.mapbox.com/", url)
        })
    };
    let vworld = |format: &str| {
        let gap = zoom - 6;
        let scale = 2f64.powi(gap);
        let reach = f64::from(2 * gap - 1);
        let covers = |tile: i32, low: f64, high: f64| ((low * scale) as i32..=(high * scale + reach) as i32).contains(&tile);
        ((5..=19).contains(&zoom) && covers(x, 53.0, 55.0) && covers(y, 22.0, 26.0))
            .then(|| ("www.vworld.kr", chain(&args(VWORLD, &[&keys.vworld_token, provider]), &[&zs, &ys, &xs, format])))
    };
    let japan = |format: &str| Some(("https://cyberjapandata.gsi.go.jp/xyz/std", chain(CYBERJAPAN, &[provider, &zs, &xs, &ys, format])));
    let corner = |tile: i32, origin: f64| qt_double(f64::from(tile).mul_add(COPERNICUS_TILE_DEGREES, -origin));
    match provider {
        "Google Street Map" => google("lyrs", "m"),
        "Google Satellite" => google("lyrs", "s"),
        "Google Terrain" => google("v", "t,r"),
        "Google Hybrid" => google("lyrs", "y"),
        "Google Labels" => google("lyrs", "h"),
        "Bing Road" => bing("r", "png"),
        "Bing Satellite" => bing("a", "jpg"),
        "Bing Hybrid" => bing("h", "jpg"),
        "TianDiTu Road" => tianditu("cia_w"),
        "TianDiTu Satellite" => tianditu("img_w"),
        "Statkart Topo" | "Statkart Basemap" => Some(("https://norgeskart.no/", chain(STATKART, &[&zs, &ys, &xs]))),
        "Svalbard Topo" => Some(("https://www.npolar.no/", chain(SVALBARD, &[&zs, &ys, &xs]))),
        "Eniro Topo" => Some(("https://www.eniro.se/", chain(ENIRO, &[&zs, &xs, &((1 << zoom) - 1 - y).to_string(), "png"]))),
        "Esri World Street" => esri("World_Street_Map"),
        "Esri World Satellite" => esri("World_Imagery"),
        "Esri Terrain" => esri("World_Terrain_Base"),
        "Mapbox Streets" => mapbox("streets-v10"),
        "Mapbox Light" => mapbox("light-v9"),
        "Mapbox Dark" => mapbox("dark-v9"),
        "Mapbox Satellite" => mapbox("satellite-v9"),
        "Mapbox Hybrid" => mapbox("satellite-streets-v10"),
        "Mapbox StreetsBasic" => mapbox("basic-v9"),
        "Mapbox Outdoors" => mapbox("outdoors-v10"),
        "Mapbox Bright" => mapbox("bright-v9"),
        "Mapbox Custom" => mapbox("mapbox.custom"),
        "MapQuest Map" | "MapQuest Sat" => Some(("https://mapquest.com", chain(MAPQUEST, &[&server(4), provider, &zs, &xs, &ys, "jpg"]))),
        "VWorld Street Map" => vworld("png"),
        "VWorld Satellite Map" => vworld("jpeg"),
        "Japan-GSI Contour" | "Japan-GSI Anaglyph" | "Japan-GSI Slope" | "Japan-GSI Relief" => japan("png"),
        "Japan-GSI Seamless" => japan("jpg"),
        "LINZ Basemap" => Some(("https://basemaps.linz.govt.nz/v1/tiles/aerial", chain(LINZ, &[&zs, &xs, &ys, "png"]))),
        "Street Map" => Some(("https://www.openstreetmap.org", chain(OPENSTREETMAP, &[&zs, &xs, &ys]))),
        "OpenAIP" => {
            let tile = chain(OPENAIP, &[&zs, &xs, &ys]);
            let key = if keys.openaip_token.is_empty() { String::new() } else { arg("?apiKey=%1", &keys.openaip_token) };
            Some(("https://www.openaip.net", format!("{tile}{key}")))
        }
        "CustomURL Custom" => Some((
            "",
            keys.custom_url.replace("{x}", &xs).replace("{y}", &ys).replace("{z}", &zs).replace("{zoom}", &zs),
        )),
        "Copernicus" => Some((
            "https://terrain-ce.suite.auterion.com",
            chain(COPERNICUS, &[&corner(y, 90.0), &corner(x, 180.0), &corner(y + 1, 90.0), &corner(x + 1, 180.0)]),
        )),
        _ => None,
    }
}

fn quadkey(x: i32, y: i32, zoom: i32) -> String {
    (1..=zoom)
        .rev()
        .map(|level| {
            let mask = 1 << (level - 1);
            char::from(b'0' + u8::from(x & mask != 0) + 2 * u8::from(y & mask != 0))
        })
        .collect()
}

fn escape_number(capture: &Captures) -> Option<u32> {
    capture[1].parse().ok()
}

fn arg(pattern: &str, value: &str) -> String {
    ESCAPE.captures_iter(pattern).filter_map(|capture| escape_number(&capture)).min().map_or_else(
        || pattern.to_string(),
        |lowest| {
            ESCAPE
                .replace_all(pattern, |capture: &Captures| {
                    if escape_number(capture) == Some(lowest) { value.to_string() } else { capture[0].to_string() }
                })
                .into_owned()
        },
    )
}

fn args(pattern: &str, values: &[&str]) -> String {
    let numbers: BTreeSet<u32> = ESCAPE.captures_iter(pattern).filter_map(|capture| escape_number(&capture)).collect();
    let slots: Vec<u32> = numbers.into_iter().take(values.len()).collect();
    ESCAPE
        .replace_all(pattern, |capture: &Captures| {
            escape_number(capture)
                .and_then(|number| slots.iter().position(|slot| *slot == number))
                .map_or_else(|| capture[0].to_string(), |index| values[index].to_string())
        })
        .into_owned()
}

fn chain(pattern: &str, values: &[&str]) -> String {
    values.iter().fold(pattern.to_string(), |text, value| arg(&text, value))
}

fn qt_double(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let exact = format!("{:.800e}", value.abs());
    let (mantissa, exponent) = exact.split_once('e').unwrap_or((&exact, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
    let head = digits[..6].iter().fold(0u64, |number, digit| number * 10 + u64::from(digit - b'0'));
    let rounded = head + u64::from(digits[6] >= b'5');
    let (kept, exponent) = if rounded == 1_000_000 { (100_000, exponent + 1) } else { (rounded, exponent) };
    let kept = kept.to_string();
    let body = if !(-4..6).contains(&exponent) {
        let tail = kept[1..].trim_end_matches('0');
        let lead = if tail.is_empty() { kept[..1].to_string() } else { format!("{}.{tail}", &kept[..1]) };
        format!("{lead}e{}{:02}", if exponent < 0 { '-' } else { '+' }, exponent.abs())
    } else if exponent >= 0 {
        let (whole, fraction) = kept.split_at(exponent as usize + 1);
        let fraction = fraction.trim_end_matches('0');
        if fraction.is_empty() { whole.to_string() } else { format!("{whole}.{fraction}") }
    } else {
        format!("0.{}{}", "0".repeat((-exponent - 1) as usize), kept.trim_end_matches('0'))
    };
    format!("{}{body}", if value < 0.0 { "-" } else { "" })
}

fn qurl(raw: &str) -> String {
    PARTS.captures(raw).map_or_else(String::new, |parts| {
        let scheme = parts.get(1).map_or_else(String::new, |scheme| format!("{}:", scheme.as_str().to_ascii_lowercase()));
        let authority = parts.get(2).map_or_else(String::new, |authority| {
            let lowered = authority
                .as_str()
                .rsplit_once('@')
                .map_or_else(|| authority.as_str().to_ascii_lowercase(), |(user, host)| format!("{user}@{}", host.to_ascii_lowercase()));
            format!("//{}", component(&lowered))
        });
        let path = component(parts.get(3).map_or("", |path| path.as_str()));
        let query = parts.get(4).map_or_else(String::new, |query| format!("?{}", component(query.as_str())));
        let fragment = parts.get(5).map_or_else(String::new, |fragment| format!("#{}", component(fragment.as_str())));
        format!("{scheme}{authority}{path}{query}{fragment}")
    })
}

fn component(part: &str) -> String {
    let bytes = part.as_bytes();
    let hex_at = |index: usize| bytes.get(index).is_some_and(u8::is_ascii_hexdigit);
    let stray = bytes.iter().enumerate().any(|(index, byte)| *byte == b'%' && !(hex_at(index + 1) && hex_at(index + 2)));
    let normal = if stray {
        part.to_string()
    } else {
        PERCENT
            .replace_all(part, |capture: &Captures| {
                u8::from_str_radix(&capture[1], 16)
                    .ok()
                    .filter(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(byte))
                    .map_or_else(|| capture[0].to_ascii_uppercase(), |byte| char::from(byte).to_string())
            })
            .into_owned()
    };
    normal
        .bytes()
        .map(|byte| {
            let encode = byte <= b' ' || byte >= 0x7F || b"\"<>\\^`{|}".contains(&byte) || (stray && byte == b'%');
            if encode { format!("%{byte:02X}") } else { char::from(byte).to_string() }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tilecache::PROVIDERS;

    fn keys() -> Keys {
        Keys {
            mapbox_token: "pk.tok".to_string(),
            mapbox_account: "acct".to_string(),
            mapbox_style: "style1".to_string(),
            esri_token: "esri-tok".to_string(),
            custom_url: "HTTPS://Tiles.Example.com/{z}/{x}/{y}.png?style=a b&k={s}&q={zoom}".to_string(),
            tianditu_token: "tdt".to_string(),
            openaip_token: "oaip".to_string(),
            vworld_token: "vw".to_string(),
            language: "en-US".to_string(),
        }
    }

    fn url(provider: &str, x: i32, y: i32, zoom: i32) -> Option<String> {
        tile_request(provider, x, y, zoom, &keys()).map(|request| request.url)
    }

    fn headers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
    }

    #[test]
    fn every_provider_the_tile_cache_names_builds_a_request() {
        let missing: Vec<&str> = PROVIDERS
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| !name.starts_with("VWorld") && url(name, 8523, 5606, 14).is_none())
            .collect();
        assert!(missing.is_empty(), "{missing:?}");
        assert_eq!(url("Nope", 1, 1, 1), None);
    }

    #[test]
    fn google_rotates_servers_and_appends_the_galileo_prefix() {
        assert_eq!(url("Google Street Map", 8523, 5606, 14).as_deref(), Some("http://mt3.google.com/vt/lyrs=m&hl=en-US&x=8523&y=5606&z=14&s=Galileo&scale=1"));
        assert_eq!(url("Google Terrain", 8523, 5606, 14).as_deref(), Some("http://mt3.google.com/vt/v=t,r&hl=en-US&x=8523&y=5606&z=14&s=Galileo&scale=1"));
        assert_eq!(url("Google Satellite", 1, 12345, 15).as_deref(), Some("http://mt3.google.com/vt/lyrs=s&hl=en-US&x=1&s=&y=12345&z=15&s=Gali&scale=1"));
        assert_eq!(url("Google Labels", 0, 0, 0).as_deref(), Some("http://mt0.google.com/vt/lyrs=h&hl=en-US&x=0&y=0&z=0&s=&scale=1"));
        assert_eq!(
            tile_request("Google Hybrid", 8523, 5606, 14, &keys()).map(|request| request.headers),
            Some(headers(&[("Accept", "*/*"), ("User-Agent", USER_AGENT), ("Referer", "https://www.google.com/maps/preview"), ("Connection", "keep-alive")]))
        );
    }

    #[test]
    fn bing_addresses_tiles_by_quadkey() {
        assert_eq!(url("Bing Road", 8523, 5606, 14).as_deref(), Some("http://ecn.t3.tiles.virtualearth.net/tiles/r12020323201231.png?g=2981&mkt=en-US"));
        assert_eq!(url("Bing Satellite", 3, 5, 3).as_deref(), Some("http://ecn.t1.tiles.virtualearth.net/tiles/a213.jpg?g=2981&mkt=en-US"));
        assert_eq!(url("Bing Hybrid", 0, 0, 0).as_deref(), Some("http://ecn.t0.tiles.virtualearth.net/tiles/h.jpg?g=2981&mkt=en-US"));
    }

    #[test]
    fn tianditu_rotates_over_eight_servers_and_needs_a_token() {
        assert_eq!(url("TianDiTu Road", 8523, 5606, 14).as_deref(), Some("https://t7.tianditu.gov.cn/DataServer?tk=tdt&T=cia_w&x=8523&y=5606&l=14"));
        assert_eq!(url("TianDiTu Satellite", 1, 1, 2).as_deref(), Some("https://t3.tianditu.gov.cn/DataServer?tk=tdt&T=img_w&x=1&y=1&l=2"));
        assert_eq!(tile_request("TianDiTu Road", 1, 1, 2, &Keys::default()), None);
    }

    #[test]
    fn esri_sends_its_token_as_a_header_and_no_referer() {
        let request = tile_request("Esri World Satellite", 8523, 5606, 14, &keys()).expect("esri");
        assert_eq!(request.url, "http://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/14/5606/8523");
        assert_eq!(request.headers, headers(&[("Accept", "*/*"), ("User-Agent", USER_AGENT), ("User-Token", "esri-tok"), ("Connection", "keep-alive")]));
        assert_eq!(
            tile_request("Esri Terrain", 1, 2, 3, &Keys::default()).map(|request| request.headers.len()),
            Some(3)
        );
    }

    #[test]
    fn mapbox_needs_a_token_and_the_custom_style_uses_the_account() {
        assert_eq!(url("Mapbox Streets", 8523, 5606, 14).as_deref(), Some("https://api.mapbox.com/styles/v1/mapbox/streets-v10/tiles/14/8523/5606?access_token=pk.tok"));
        assert_eq!(url("Mapbox Custom", 8523, 5606, 14).as_deref(), Some("https://api.mapbox.com/styles/v1/acct/style1/tiles/256/14/8523/5606?access_token=pk.tok"));
        assert_eq!(tile_request("Mapbox Dark", 1, 1, 1, &Keys::default()), None);
    }

    #[test]
    fn mapbox_account_placeholders_are_rescanned_like_qstring_arg() {
        let tricky = Keys { mapbox_account: "a%3b".to_string(), ..keys() };
        assert_eq!(
            tile_request("Mapbox Custom", 8523, 5606, 14, &tricky).map(|request| request.url).as_deref(),
            Some("https://api.mapbox.com/styles/v1/a14b/style1/tiles/256/14/8523/5606?access_token=pk.tok")
        );
    }

    #[test]
    fn generic_providers_fill_their_templates() {
        assert_eq!(url("Statkart Topo", 8523, 5606, 14).as_deref(), Some("https://cache.kartverket.no/v1/wmts/1.0.0/topo/default/webmercator/14/5606/8523.png"));
        assert_eq!(url("Statkart Basemap", 8523, 5606, 14), url("Statkart Topo", 8523, 5606, 14));
        assert_eq!(
            url("Svalbard Topo", 8523, 5606, 14).as_deref(),
            Some("https://geodata.npolar.no/arcgis/rest/services/Basisdata/NP_Basiskart_Svalbard_WMTS_3857/MapServer/WMTS/tile/1.0.0/Basisdata_NP_Basiskart_Svalbard_WMTS_3857/default/default028mm/14/5606/8523")
        );
        assert_eq!(url("Eniro Topo", 8523, 5606, 14).as_deref(), Some("http://map.eniro.com/geowebcache/service/tms1.0.0/map/14/8523/10777.png"));
        assert_eq!(url("LINZ Basemap", 8523, 5606, 14).as_deref(), Some("https://basemaps.linz.govt.nz/v1/tiles/aerial/EPSG:3857/14/8523/5606.png?api=d01ev80nqcjxddfvc6amyvkk1ka"));
        assert_eq!(url("Street Map", 8523, 5606, 14).as_deref(), Some("http://tile.openstreetmap.org/14/8523/5606.png"));
        assert_eq!(url("OpenAIP", 8523, 5606, 14).as_deref(), Some("https://api.tiles.openaip.net/api/data/openaip/14/8523/5606.png?apiKey=oaip"));
        assert_eq!(
            tile_request("OpenAIP", 8523, 5606, 14, &Keys::default()).map(|request| request.url).as_deref(),
            Some("https://api.tiles.openaip.net/api/data/openaip/14/8523/5606.png")
        );
    }

    #[test]
    fn providers_that_put_their_display_name_in_the_path_get_it_percent_encoded() {
        assert_eq!(url("MapQuest Map", 8523, 5606, 14).as_deref(), Some("http://otile3.mqcdn.com/tiles/1.0.0/MapQuest%20Map/14/8523/5606.jpg"));
        assert_eq!(url("Japan-GSI Contour", 8523, 5606, 14).as_deref(), Some("https://cyberjapandata.gsi.go.jp/xyz/Japan-GSI%20Contour/14/8523/5606.png"));
        assert_eq!(url("Japan-GSI Seamless", 8523, 5606, 14).as_deref(), Some("https://cyberjapandata.gsi.go.jp/xyz/Japan-GSI%20Seamless/14/8523/5606.jpg"));
    }

    #[test]
    fn vworld_only_serves_korea() {
        assert_eq!(url("VWorld Street Map", 13970, 6300, 14).as_deref(), Some("http://api.vworld.kr/req/wmts/1.0.0/vw/VWorld%20Street%20Map/14/6300/13970.png"));
        assert_eq!(url("VWorld Satellite Map", 53, 22, 6).as_deref(), Some("http://api.vworld.kr/req/wmts/1.0.0/vw/VWorld%20Satellite%20Map/6/22/53.jpeg"));
        assert_eq!(url("VWorld Street Map", 8523, 5606, 14), None);
        assert_eq!(url("VWorld Street Map", 55, 22, 6), None);
        assert_eq!(url("VWorld Street Map", 26, 11, 5), None);
        assert_eq!(url("VWorld Street Map", 53, 22, 20), None);
    }

    #[test]
    fn custom_url_substitutes_and_is_normalised_like_qurl() {
        let request = tile_request("CustomURL Custom", 8523, 5606, 14, &keys()).expect("custom");
        assert_eq!(request.url, "https://tiles.example.com/14/8523/5606.png?style=a%20b&k=%7Bs%7D&q=14");
        assert_eq!(request.headers, headers(&[("Accept", "*/*"), ("User-Agent", USER_AGENT), ("Connection", "keep-alive")]));
        assert_eq!(tile_request("CustomURL Custom", 1, 1, 1, &Keys::default()), None);
    }

    #[test]
    fn copernicus_asks_for_a_hundredth_degree_carpet() {
        assert_eq!(url("Copernicus", 20000, 13737, 0).as_deref(), Some("https://terrain-ce.suite.auterion.com/api/v1/carpet?points=47.37,20,47.38,20.01"));
        assert_eq!(url("Copernicus", 0, 0, 0).as_deref(), Some("https://terrain-ce.suite.auterion.com/api/v1/carpet?points=-90,-180,-89.99,-179.99"));
        assert_eq!(url("Copernicus", 18000, 9000, 0).as_deref(), Some("https://terrain-ce.suite.auterion.com/api/v1/carpet?points=1.8735e-15,3.747e-15,0.01,0.01"));
        assert_eq!(url("Copernicus", 12345, 6789, 0).as_deref(), Some("https://terrain-ce.suite.auterion.com/api/v1/carpet?points=-22.11,-56.55,-22.1,-56.54"));
    }

    #[test]
    fn doubles_print_like_qstring_arg() {
        assert_eq!(qt_double(0.0001), "0.0001");
        assert_eq!(qt_double(1e-5), "1e-05");
        assert_eq!(qt_double(-1234567.0), "-1.23457e+06");
        assert_eq!(qt_double(123456.5), "123457");
    }

    #[test]
    fn qurl_encodes_every_percent_in_a_component_with_a_stray_one() {
        assert_eq!(qurl("http://x.com/a%41%2f%7e%2B%20/é?t=%3d%zq"), "http://x.com/aA%2F~%2B%20/%C3%A9?t=%253d%25zq");
        assert_eq!(qurl("no scheme {x}/y"), "no%20scheme%20%7Bx%7D/y");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_mac_build_claims_to_be_firefox() {
        assert_eq!(USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.5; rv:125.0) Gecko/20100101 Firefox/125.0");
    }
}
