use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const WGS84_A: f64 = 6_378_137.0;

pub const FULL_TURN_DEGREES: f64 = 360.0;

pub fn wrap_bearing(degrees: f64) -> f64 {
    degrees.rem_euclid(FULL_TURN_DEGREES)
}

pub fn wrap_tilt(degrees: f64) -> f64 {
    let wrapped = (degrees + 180.0).rem_euclid(FULL_TURN_DEGREES) - 180.0;
    match wrapped == -180.0 {
        true => 180.0,
        false => wrapped,
    }
}

pub fn wrap_longitude(longitude: f64) -> f64 {
    let wrapped = (longitude + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped == -180.0 { 180.0 } else { wrapped }
}

pub fn clamp_latitude(latitude: f64) -> f64 {
    latitude.clamp(-90.0, 90.0)
}

pub fn wrap(latitude: f64, longitude: f64) -> (f64, f64) {
    (clamp_latitude(latitude), wrap_longitude(longitude))
}

const WGS84_F: f64 = 1.0 / 298.257223563;
const WGS84_E2: f64 = WGS84_F * (2.0 - WGS84_F);

fn sincosd(degrees: f64) -> (f64, f64) {
    let reduced = degrees % 360.0;
    let quadrant = (reduced / 90.0).round();
    let radians = (reduced - 90.0 * quadrant).to_radians();
    let (s, c) = (radians.sin(), radians.cos());
    match (quadrant as i64).rem_euclid(4) {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

fn ecef(lat: f64, lon: f64, alt: f64) -> (f64, f64, f64) {
    let (sin_phi, cos_phi) = sincosd(lat);
    let (sin_lam, cos_lam) = sincosd(lon);
    let n = WGS84_A / (1.0 - WGS84_E2 * sin_phi * sin_phi).sqrt();
    let z = ((1.0 - WGS84_E2) * n + alt) * sin_phi;
    let x = (n + alt) * cos_phi;
    (x * cos_lam, x * sin_lam, z)
}

fn geodetic(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let lon = y.atan2(x);
    let p = (x * x + y * y).sqrt();
    let mut lat = z.atan2(p * (1.0 - WGS84_E2));
    let mut alt = 0.0;
    for _ in 0..12 {
        let sin_lat = lat.sin();
        let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
        alt = if lat.cos().abs() > 1e-12 { p / lat.cos() - n } else { z.abs() - n * (1.0 - WGS84_E2) };
        lat = z.atan2(p * (1.0 - WGS84_E2 * n / (n + alt)));
    }
    (lat.to_degrees(), lon.to_degrees(), alt)
}

fn finite_alt(alt: f64) -> f64 {
    if alt.is_nan() { 0.0 } else { alt }
}

pub fn geo_to_ned(lat: f64, lon: f64, alt: f64, origin: (f64, f64, f64)) -> (f64, f64, f64) {
    let (olat, olon, oalt) = origin;
    if lat == olat && lon == olon && alt == oalt {
        return (0.0, 0.0, 0.0);
    }
    let (ox, oy, oz) = ecef(olat, olon, finite_alt(oalt));
    let (px, py, pz) = ecef(lat, lon, finite_alt(alt));
    let (dx, dy, dz) = (px - ox, py - oy, pz - oz);
    let ((sin_phi, cos_phi), (sin_lam, cos_lam)) = (sincosd(olat), sincosd(olon));
    let east = -sin_lam * dx + cos_lam * dy;
    let north = -sin_phi * cos_lam * dx - sin_phi * sin_lam * dy + cos_phi * dz;
    let up = cos_phi * cos_lam * dx + cos_phi * sin_lam * dy + sin_phi * dz;
    (north, east, -up)
}

pub fn ned_to_geo(x: f64, y: f64, z: f64, origin: (f64, f64, f64)) -> (f64, f64, f64) {
    let (olat, olon, oalt) = origin;
    let (north, east, up) = (x, y, -z);
    let ((sin_phi, cos_phi), (sin_lam, cos_lam)) = (sincosd(olat), sincosd(olon));
    let dx = -sin_lam * east - sin_phi * cos_lam * north + cos_phi * cos_lam * up;
    let dy = cos_lam * east - sin_phi * sin_lam * north + cos_phi * sin_lam * up;
    let dz = cos_phi * north + sin_phi * up;
    let (ox, oy, oz) = ecef(olat, olon, finite_alt(oalt));
    geodetic(ox + dx, oy + dy, oz + dz)
}

pub fn geo_to_utm(lat: f64, lon: f64) -> (u8, f64, f64) {
    let zone = utm::lat_lon_to_zone_number(lat, lon);
    let (northing, easting, _) = utm::to_utm_wgs84(lat, lon, zone);
    (zone, easting, northing)
}

pub fn utm_to_geo(easting: f64, northing: f64, zone: u8, southern: bool) -> Option<(f64, f64)> {
    utm::wsg84_utm_to_lat_lon(easting, northing, zone, if southern { 'M' } else { 'N' }).ok()
}

fn numbers(args: &[String], count: usize) -> Option<Vec<f64>> {
    let parsed: Vec<f64> = args.iter().take(count).filter_map(|a| a.parse::<f64>().ok()).filter(|v| v.is_finite()).collect();
    (parsed.len() == count).then_some(parsed)
}

pub fn geo_to_ned_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 6) else { return crate::read::refused("this needs six numbers: latitude, longitude, altitude, then the origin latitude, longitude and altitude") };
    let (x, y, z) = geo_to_ned(v[0], v[1], v[2], (v[3], v[4], v[5]));
    json!({ "kind": "object", "class": "Ned", "x": x, "y": y, "z": z })
}

pub fn ned_to_geo_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 6) else { return crate::read::refused("this needs six numbers: north, east, down, then the origin latitude, longitude and altitude") };
    let (lat, lon, alt) = ned_to_geo(v[0], v[1], v[2], (v[3], v[4], v[5]));
    json!({ "kind": "coordinate", "valid": true, "latitude": lat, "longitude": lon, "altitude": alt })
}

pub fn geo_to_utm_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 2) else { return crate::read::refused("this needs two numbers: latitude and longitude") };
    let (zone, easting, northing) = geo_to_utm(v[0], v[1]);
    json!({ "kind": "object", "class": "Utm", "zone": zone, "southern": v[0] < 0.0, "easting": easting, "northing": northing })
}

pub fn utm_to_geo_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 3) else { return crate::read::refused("this needs three numbers: easting, northing and zone") };
    let southern = args.get(3).map(|s| s == "true" || s == "1" || s == "S").unwrap_or(false);
    match utm_to_geo(v[0], v[1], v[2] as u8, southern) {
        Some((lat, lon)) => json!({ "kind": "coordinate", "valid": true, "latitude": lat, "longitude": lon, "altitude": 0.0 }),
        None => json!({ "kind": "coordinate", "valid": false }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: (f64, f64, f64) = (47.3764, 8.5481, 0.0);

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 0.00001
    }

    #[test]
    fn ned_matches_geo_test() {
        let (x, y, z) = geo_to_ned(47.364869, 8.594398, 0.0, ORIGIN);
        assert!(close(x, -1280.954612) && close(y, 3497.196961) && close(z, 1.085830), "{x} {y} {z}");
        let (x, y, z) = geo_to_ned(ORIGIN.0, ORIGIN.1, 10.0, ORIGIN);
        assert!(close(x, 0.0) && close(y, 0.0) && close(z, -10.0));
        let (lat, lon, alt) = ned_to_geo(-1280.954612, 3497.196961, 1.085830, ORIGIN);
        assert!(close(lat, 47.364869) && close(lon, 8.594398) && close(alt, 0.0), "{lat} {lon} {alt}");
        let (lat, lon, alt) = ned_to_geo(0.0, 0.0, 0.0, ORIGIN);
        assert!(close(lat, ORIGIN.0) && close(lon, ORIGIN.1) && close(alt, 0.0));
    }

    #[test]
    fn utm_matches_geo_test() {
        let (zone, easting, northing) = geo_to_utm(ORIGIN.0, ORIGIN.1);
        assert_eq!(zone, 32);
        assert!((easting - 465886.092246).abs() < 0.01, "{easting}");
        assert!((northing - 5247092.44892).abs() < 0.01, "{northing}");
        let (lat, lon) = utm_to_geo(465886.092246, 5247092.44892, 32, false).unwrap();
        assert!(close(lat, ORIGIN.0) && close(lon, ORIGIN.1), "{lat} {lon}");
    }
}

#[cfg(test)]
mod wraptests {
    use super::*;

    #[test]
    fn a_coordinate_past_the_edge_of_the_world_comes_back_onto_it() {
        assert_eq!(wrap_longitude(180.0018), -179.9982);
        assert_eq!(wrap_longitude(-180.5), 179.5);
        assert_eq!(wrap_longitude(8.5), 8.5, "an ordinary longitude is untouched");
        assert_eq!(wrap_longitude(180.0), 180.0, "the dateline itself is a real longitude and stays one");
        assert_eq!(wrap_longitude(-180.0), 180.0);
        assert_eq!(clamp_latitude(90.4), 90.0, "there is no latitude past the pole to seed a mission item at");
        assert_eq!(clamp_latitude(-90.4), -90.0);
        assert_eq!(wrap(47.4, 8.5), (47.4, 8.5));
    }
}
