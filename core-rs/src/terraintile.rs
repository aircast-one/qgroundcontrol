use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const HEADER_BYTES: usize = 48;

#[derive(Debug, PartialEq, Clone)]
pub struct Tile {
    pub sw_lat: f64,
    pub sw_lon: f64,
    pub ne_lat: f64,
    pub ne_lon: f64,
    pub min_elevation: i16,
    pub max_elevation: i16,
    pub avg_elevation: f64,
    pub grid_lat: usize,
    pub grid_lon: usize,
    pub elevations: Vec<i16>,
}

pub fn serialize(input: &str) -> Result<Vec<u8>, String> {
    let root: Value = serde_json::from_str(input).map_err(|e| format!("not JSON: {e}"))?;
    if root.get("status").and_then(Value::as_str) != Some("success") {
        return Err("status is not success".to_string());
    }
    let data = root.get("data").ok_or("no data object")?;
    let pair = |key: &str| -> Result<(f64, f64), String> {
        let a = data.get("bounds").and_then(|b| b.get(key)).and_then(Value::as_array).ok_or(format!("no bounds.{key}"))?;
        match (a.first().and_then(Value::as_f64), a.get(1).and_then(Value::as_f64)) {
            (Some(x), Some(y)) => Ok((x, y)),
            _ => Err("Incomplete bounding location".to_string()),
        }
    };
    let (sw_lat, sw_lon) = pair("sw")?;
    let (ne_lat, ne_lon) = pair("ne")?;
    let stat = |key: &str| data.get("stats").and_then(|s| s.get(key)).and_then(Value::as_f64).ok_or(format!("no stats.{key}"));
    let (min, max, avg) = (stat("min")?, stat("max")?, stat("avg")?);
    let carpet = data.get("carpet").and_then(Value::as_array).ok_or("no carpet")?;
    let grid_lat = carpet.len();
    let grid_lon = carpet.first().and_then(Value::as_array).map(|r| r.len()).unwrap_or(0);
    let rows: Vec<Vec<i16>> = carpet
        .iter()
        .map(|row| row.as_array().filter(|r| r.len() >= grid_lon).map(|r| r.iter().take(grid_lon).map(|v| v.as_f64().unwrap_or(0.0) as i16).collect()).ok_or("short carpet row".to_string()))
        .collect::<Result<_, _>>()?;
    let tile = Tile {
        sw_lat, sw_lon, ne_lat, ne_lon,
        min_elevation: min as i16,
        max_elevation: max as i16,
        avg_elevation: avg,
        grid_lat,
        grid_lon,
        elevations: rows.into_iter().flatten().collect(),
    };
    Ok(encode(&tile))
}

pub fn encode(tile: &Tile) -> Vec<u8> {
    [tile.sw_lat, tile.sw_lon, tile.ne_lat, tile.ne_lon]
        .iter()
        .flat_map(|v| v.to_ne_bytes())
        .chain(tile.min_elevation.to_ne_bytes())
        .chain(tile.max_elevation.to_ne_bytes())
        .chain(tile.avg_elevation.to_ne_bytes())
        .chain((tile.grid_lat as i16).to_ne_bytes())
        .chain((tile.grid_lon as i16).to_ne_bytes())
        .chain(tile.elevations.iter().flat_map(|e| e.to_ne_bytes()))
        .collect()
}

pub fn decode(bytes: &[u8]) -> Option<Tile> {
    if bytes.len() < HEADER_BYTES {
        return None;
    }
    let f64_at = |at: usize| f64::from_ne_bytes(bytes[at..at + 8].try_into().unwrap());
    let i16_at = |at: usize| i16::from_ne_bytes(bytes[at..at + 2].try_into().unwrap());
    let (grid_lat, grid_lon) = (i16_at(44).max(0) as usize, i16_at(46).max(0) as usize);
    let count = grid_lat * grid_lon;
    if bytes.len() < HEADER_BYTES + 2 * count {
        return None;
    }
    let tile = Tile {
        sw_lat: f64_at(0),
        sw_lon: f64_at(8),
        ne_lat: f64_at(16),
        ne_lon: f64_at(24),
        min_elevation: i16_at(32),
        max_elevation: i16_at(34),
        avg_elevation: f64_at(36),
        grid_lat,
        grid_lon,
        elevations: (0..count).map(|i| i16_at(HEADER_BYTES + 2 * i)).collect(),
    };
    ((tile.ne_lon - tile.sw_lon) >= 0.0 && (tile.ne_lat - tile.sw_lat) >= 0.0).then_some(tile)
}

impl Tile {
    pub fn elevation(&self, lat: f64, lon: f64) -> Option<f64> {
        let cell_lat = (self.ne_lat - self.sw_lat) / self.grid_lat as f64;
        let cell_lon = (self.ne_lon - self.sw_lon) / self.grid_lon as f64;
        let lat_index = ((lat - self.sw_lat) / cell_lat).floor();
        let lon_index = ((lon - self.sw_lon) / cell_lon).floor();
        let (in_lat, in_lon) = (lat_index >= 0.0 && lat_index < self.grid_lat as f64, lon_index >= 0.0 && lon_index < self.grid_lon as f64);
        match in_lat && in_lon {
            true => self.elevations.get(lat_index as usize * self.grid_lon + lon_index as usize).map(|e| *e as f64),
            false => None,
        }
    }
}

pub fn terrain_tile_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return json!({ "kind": "null" }) };
    let Ok(bytes) = std::fs::read(path) else { return json!({ "kind": "object", "class": "TerrainTile", "path": path, "readable": false }) };
    let Some(tile) = decode(&bytes) else { return json!({ "kind": "object", "class": "TerrainTile", "path": path, "readable": true, "valid": false }) };
    let probe = match (args.get(1).and_then(|a| a.parse::<f64>().ok()), args.get(2).and_then(|a| a.parse::<f64>().ok())) {
        (Some(lat), Some(lon)) => tile.elevation(lat, lon),
        _ => None,
    };
    json!({
        "kind": "object",
        "class": "TerrainTile",
        "path": path,
        "readable": true,
        "valid": true,
        "southWest": { "latitude": tile.sw_lat, "longitude": tile.sw_lon },
        "northEast": { "latitude": tile.ne_lat, "longitude": tile.ne_lon },
        "minElevation": tile.min_elevation,
        "maxElevation": tile.max_elevation,
        "avgElevation": tile.avg_elevation,
        "gridLat": tile.grid_lat,
        "gridLon": tile.grid_lon,
        "elevation": probe,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"status":"success","data":{"bounds":{"sw":[47.0,8.0],"ne":[47.1,8.2]},"stats":{"min":400,"max":460,"avg":430.5},"carpet":[[400,410,420,430],[440,450,455,460]]}}"#;

    #[test]
    fn a_copernicus_reply_serialises_to_the_packed_layout_and_decodes_back() {
        let bytes = serialize(SAMPLE).unwrap();
        assert_eq!(bytes.len(), HEADER_BYTES + 2 * 8);
        let tile = decode(&bytes).unwrap();
        assert_eq!((tile.grid_lat, tile.grid_lon), (2, 4));
        assert_eq!(tile.min_elevation, 400);
        assert_eq!(tile.avg_elevation, 430.5);
        assert_eq!(tile.elevations, vec![400, 410, 420, 430, 440, 450, 455, 460]);
        assert_eq!(encode(&tile), bytes);
    }

    #[test]
    fn elevation_is_the_cell_south_west_of_the_point_and_nothing_outside() {
        let tile = decode(&serialize(SAMPLE).unwrap()).unwrap();
        assert_eq!(tile.elevation(47.01, 8.01), Some(400.0));
        assert_eq!(tile.elevation(47.01, 8.19), Some(430.0));
        assert_eq!(tile.elevation(47.09, 8.11), Some(455.0));
        assert_eq!(tile.elevation(46.99, 8.01), None);
        assert_eq!(tile.elevation(47.01, 8.21), None);
    }

    #[test]
    fn bad_replies_and_short_tiles_are_refused() {
        assert_eq!(serialize(r#"{"status":"error"}"#).unwrap_err(), "status is not success");
        assert!(serialize(r#"{"status":"success","data":{"bounds":{"sw":[1]},"stats":{}}}"#).is_err());
        assert!(decode(&[0u8; 10]).is_none());
        let mut bytes = serialize(SAMPLE).unwrap();
        bytes.truncate(HEADER_BYTES + 3);
        assert!(decode(&bytes).is_none());
    }
}
