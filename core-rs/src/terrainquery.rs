use crate::terraintile::{self, Tile};
use crate::tilecache::{self, Cache};

const TILE_SIZE_DEGREES: f64 = 0.01;
const PROVIDER: &str = "Copernicus";
const PROVIDER_URL: &str = "https://terrain-ce.suite.auterion.com";
const TILE_FORMAT: &str = "bin";
const TERRAIN_ZOOM: i32 = 1;

pub fn tile_xy(latitude: f64, longitude: f64) -> (i32, i32) {
    (((longitude + 180.0) / TILE_SIZE_DEGREES).floor() as i32, ((latitude + 90.0) / TILE_SIZE_DEGREES).floor() as i32)
}

fn significant_six(value: f64) -> String {
    let magnitude = if value == 0.0 { 0 } else { value.abs().log10().floor() as i32 };
    let decimals = usize::try_from(5 - magnitude).unwrap_or(0);
    let fixed = format!("{value:.decimals$}");
    match fixed.contains('.') {
        true => fixed.trim_end_matches('0').trim_end_matches('.').to_string(),
        false => fixed,
    }
}

pub fn url(x: i32, y: i32) -> String {
    let corner = |i: i32, origin: f64| significant_six(f64::from(i) * TILE_SIZE_DEGREES - origin);
    format!("{PROVIDER_URL}/api/v1/carpet?points={},{},{},{}", corner(y, 90.0), corner(x, 180.0), corner(y + 1, 90.0), corner(x + 1, 180.0))
}

pub fn tile_hash(x: i32, y: i32) -> Option<String> {
    tilecache::provider_hash(PROVIDER).map(|provider| tilecache::tile_hash(provider, x, y, TERRAIN_ZOOM))
}

pub fn elevation(latitude: f64, longitude: f64, cache: Option<&Cache>, fetch: &dyn Fn(&str) -> Result<String, String>) -> Result<f64, String> {
    let (x, y) = tile_xy(latitude, longitude);
    let hash = tile_hash(x, y).ok_or("The Copernicus provider is not in the tile table.")?;
    let cached = cache.and_then(|c| c.tile(&hash).ok().flatten()).map(|t| t.image);
    let bytes = match cached {
        Some(bytes) => bytes,
        None => {
            let bytes = terraintile::serialize(&fetch(&url(x, y))?)?;
            if let Some(c) = cache {
                let _ = c.save(&tilecache::Tile { hash, format: TILE_FORMAT.to_string(), image: bytes.clone(), kind: tilecache::provider_hash(PROVIDER).unwrap_or(0) }, None);
            }
            bytes
        }
    };
    let tile: Tile = terraintile::decode(&bytes).ok_or("The terrain tile could not be read.")?;
    tile.elevation(latitude, longitude).ok_or_else(|| "The terrain tile does not cover that position.".to_string())
}

pub fn fetch_over_http(url: &str) -> Result<String, String> {
    ureq::get(url).call().map_err(|e| format!("The terrain server did not answer: {e}"))?.body_mut().read_to_string().map_err(|e| format!("The terrain answer could not be read: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serving<'a>(body: &'static str, asked: &'a std::cell::RefCell<Vec<String>>) -> impl Fn(&str) -> Result<String, String> + 'a {
        move |url: &str| {
            asked.borrow_mut().push(url.to_string());
            Ok(body.to_string())
        }
    }

    #[test]
    fn the_url_spells_the_tile_corners_as_qt_formats_a_double() {
        assert_eq!(tile_xy(47.633389756176875, -122.09076300000001), (5790, 13763));
        assert_eq!(url(5790, 13763), "https://terrain-ce.suite.auterion.com/api/v1/carpet?points=47.63,-122.1,47.64,-122.09");
        assert_eq!(url(32916, 5463), "https://terrain-ce.suite.auterion.com/api/v1/carpet?points=-35.37,149.16,-35.36,149.17", "y * 0.01 - 90 is -35.370000000000005 in binary; QString::arg prints six significant digits");
    }

    #[test]
    fn the_elevation_under_home_is_the_altitude_qt_gave_the_planned_home() {
        let asked = std::cell::RefCell::new(Vec::new());
        let seattle = elevation(47.633389756176875, -122.09076300000001, None, &serving(include_str!("../tests/fixtures/copernicus-sectiontest.json"), &asked)).unwrap();
        assert_eq!(seattle, 35.0, "Qt re-saved SectionTest.plan with home at 35");
        let canberra = elevation(-35.363262399999996, 149.1652378, None, &serving(include_str!("../tests/fixtures/copernicus-survey.json"), &asked)).unwrap();
        assert_eq!(canberra, 585.0, "and the survey plan at 585");
    }

    #[test]
    fn a_cached_tile_is_not_fetched_again() {
        let cache = Cache::open_in_memory().unwrap();
        let asked = std::cell::RefCell::new(Vec::new());
        let fetch = serving(include_str!("../tests/fixtures/copernicus-sectiontest.json"), &asked);
        elevation(47.6334, -122.0907, Some(&cache), &fetch).unwrap();
        elevation(47.6335, -122.0906, Some(&cache), &fetch).unwrap();
        assert_eq!(asked.borrow().len(), 1);
    }
}
