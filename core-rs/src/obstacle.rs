use serde_json::{Value, json};

use crate::read::{Unit, format_measure, integer, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.objectAvoidance.available",
    "vehicle.objectAvoidance.enabled",
    "vehicle.objectAvoidance.distances",
    "vehicle.objectAvoidance.msSinceUpdate",
    "settings.unitsSettings.horizontalDistanceUnits",
];

const NO_READING: i64 = 65535;
const STALE_AFTER_MS: i64 = 3000;
const CENTIMETRES_PER_METRE: f64 = 100.0;
const SECTORS: [(f64, &str, &str); 8] = [
    (22.5, "ahead", "ahead"),
    (67.5, "aheadRight", "ahead right"),
    (112.5, "right", "right"),
    (157.5, "behindRight", "behind right"),
    (202.5, "behind", "behind"),
    (247.5, "behindLeft", "behind left"),
    (292.5, "left", "left"),
    (337.5, "aheadLeft", "ahead left"),
];

pub fn nearest(distances: &[i64], max_distance: i64) -> Option<(usize, i64)> {
    distances
        .iter()
        .enumerate()
        .filter(|(_, cm)| **cm < max_distance)
        .min_by_key(|(_, cm)| **cm)
        .map(|(at, cm)| (at, *cm))
}

pub fn bearing(index: usize, increment: f64, angle_offset: f64) -> f64 {
    (increment * index as f64 + angle_offset).rem_euclid(360.0)
}

pub fn sector(bearing: f64) -> (&'static str, &'static str) {
    let wrapped = bearing.rem_euclid(360.0);
    SECTORS
        .iter()
        .find(|(edge, _, _)| wrapped < *edge)
        .map(|(_, id, text)| (*id, *text))
        .unwrap_or(("ahead", "ahead"))
}

pub fn obstacle_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let avoidance = object(&backend.get_fields(
        "vehicle.objectAvoidance",
        "available,enabled,distances,increment,minDistance,maxDistance,angleOffset,msSinceUpdate",
    ));
    let flag = |key: &str| avoidance.get(key).and_then(Value::as_bool) == Some(true);
    let whole = |key: &str| integer(&avoidance, key);
    let real = |key: &str| avoidance.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
    let available = flag("available");
    let ring: Vec<i64> = avoidance.get("distances").and_then(Value::as_array).map(|list| list.iter().filter_map(Value::as_i64).collect()).unwrap_or_default();
    let (max_distance, min_distance) = (whole("maxDistance").unwrap_or(0), whole("minDistance").unwrap_or(0));
    let since = whole("msSinceUpdate");
    let stale = since.map(|ms| ms > STALE_AFTER_MS);
    let found = match available && max_distance > 0 {
        true => nearest(&ring, max_distance),
        false => None,
    };
    let unit = Unit::horizontal(backend);
    let spacing = real("increment").filter(|degrees| *degrees > 0.0);
    let reading = found.zip(spacing).map(|((at, cm), increment)| {
        let metres = cm as f64 / CENTIMETRES_PER_METRE;
        let heading = bearing(at, increment, real("angleOffset").unwrap_or(0.0));
        let (id, text) = sector(heading);
        json!({
            "distanceMetres": metres,
            "distanceText": format_measure(unit.show(metres), &unit.name),
            "bearing": heading,
            "sector": id,
            "sectorText": text,
            "close": min_distance > 0 && cm < min_distance * 2,
        })
    });
    json!({
        "kind": "object",
        "class": "ObstacleDistance",
        "available": available,
        "enabled": flag("enabled"),
        "stale": stale,
        "msSinceUpdate": since,
        "nearest": reading,
        "sectors": ring.iter().filter(|cm| **cm != NO_READING).count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ring(Value);
    impl Backend for Ring {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicle.objectAvoidance" => self.0.to_string(),
                _ => json!({ "kind": "object" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn ring(distances: Vec<i64>) -> Value {
        json!({ "kind": "object", "available": true, "enabled": true, "distances": distances,
                "increment": 45.0, "minDistance": 100, "maxDistance": 1000, "angleOffset": 0.0, "msSinceUpdate": 200 })
    }

    #[test]
    fn a_sector_with_no_reading_is_not_the_nearest_obstacle() {
        assert_eq!(nearest(&[NO_READING, 500, NO_READING], 1000), Some((1, 500)), "65535 is what the ring carries where the sensor saw nothing, and it is the smallest number in the array if you sort it as one");
        assert_eq!(nearest(&[NO_READING, NO_READING], 1000), None, "nothing seen anywhere is no reading, not a reading of zero");
        assert_eq!(nearest(&[1200, 1500], 1000), None, "QGC's own grid drops anything at or past maxDistance, so the core drops it too");
        assert_eq!(nearest(&[50, 800], 1000), Some((0, 50)), "and it does not drop a reading below minDistance, which QGC also keeps - the aircraft is closer than the sensor is rated for, which is the last thing to hide");
        assert_eq!(nearest(&[NO_READING, 40000], NO_READING), Some((1, 40000)), "QGC checks the empty marker and the range separately; the second implies the first, because max_distance is a uint16 and 65535 is never below it. One test, not two guards.");
    }

    #[test]
    fn the_bearing_is_relative_to_the_aircraft_and_names_a_sector() {
        assert_eq!(bearing(0, 45.0, 0.0), 0.0);
        assert_eq!(bearing(2, 45.0, 0.0), 90.0);
        assert_eq!(bearing(7, 45.0, 45.0), 0.0, "the offset wraps rather than running past a full turn");
        assert_eq!(sector(0.0).0, "ahead");
        assert_eq!(sector(90.0).0, "right");
        assert_eq!(sector(180.0).0, "behind");
        assert_eq!(sector(270.0).0, "left");
        assert_eq!(sector(350.0).0, "ahead", "the last sector wraps back to dead ahead rather than falling off the end of the table");
        assert_eq!(sector(22.5).0, "aheadRight", "a bearing exactly on a boundary belongs to the sector it opens, not the one it closes");
        assert_eq!(sector(22.4).0, "ahead");
        assert_eq!(sector(90.0).1, "right", "the identifier is what a head keys on and the text is English prose, so a translation never changes the key");
    }

    #[test]
    fn the_view_reports_the_nearest_obstacle_or_says_there_is_none() {
        let seen = obstacle_view(&Ring(ring(vec![NO_READING, 320, 900])), &[]);
        assert_eq!(seen["nearest"]["distanceMetres"], 3.2);
        assert_eq!(seen["nearest"]["distanceText"], "3.2 m", "the sensor resolves centimetres and the operator is judging clearance, so this keeps the tenth that distance_text drops - 3.4 m and 2.6 m both reading \"3 m\" is the wrong trade for a proximity warning");
        assert_eq!(seen["nearest"]["sector"], "aheadRight");
        assert_eq!(seen["nearest"]["close"], false, "3.2 m is more than twice the sensor's 1 m minimum");
        assert_eq!(seen["stale"], false);
        assert_eq!(seen["sectors"], 2);

        struct Feet(Value);
        impl Backend for Feet {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, path: &str, f: &str) -> String {
                match path {
                    "units" => json!({ "kind": "object", "appSettingsHorizontalDistanceUnitsString": "ft" }).to_string(),
                    _ => Ring(self.0.clone()).get_fields(path, f),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                match path {
                    "units.metersToAppSettingsHorizontalDistanceUnits" => json!({ "ok": true, "result": serde_json::from_str::<Vec<f64>>(args).unwrap()[0] * 3.2808399 }).to_string(),
                    _ => String::new(),
                }
            }
            fn watch(&self, _p: &[String]) {}
        }
        let feet = obstacle_view(&Feet(ring(vec![NO_READING, 320, 900])), &[]);
        assert_eq!(feet["nearest"]["distanceText"], "10.5 ft", "the unit follows the operator, which was the whole defect - and the tenth survives the conversion");
        assert_eq!(feet["nearest"]["distanceMetres"], 3.2, "the metres stay raw beside the text, so nothing downstream parses a spelling back");

        let near = obstacle_view(&Ring(ring(vec![150, NO_READING])), &[]);
        assert_eq!(near["nearest"]["close"], true, "inside twice the sensor's minimum is the band the operator is warned about");

        let mut spaceless = ring(vec![NO_READING, 320]);
        spaceless["increment"] = json!(0.0);
        assert_eq!(obstacle_view(&Ring(spaceless), &[])["nearest"], Value::Null, "without the angle between sectors an index names no direction, and defaulting it to zero would point every reading dead ahead - a bearing that looks like a measurement");

        let empty = obstacle_view(&Ring(ring(vec![NO_READING, NO_READING])), &[]);
        assert_eq!(empty["nearest"], Value::Null, "no reading at all is absent rather than a distance of zero, which would read as a collision");
        assert_eq!(empty["sectors"], 0);

        let mut old = ring(vec![320]);
        old["msSinceUpdate"] = json!(4000);
        assert_eq!(obstacle_view(&Ring(old), &[])["stale"], true, "the ring stops arriving when the sensor stops, and a frozen reading looks exactly like a live one");

        let off = obstacle_view(&Ring(json!({ "kind": "object", "available": false })), &[]);
        assert_eq!(off["nearest"], Value::Null);
        assert_eq!(off["stale"], Value::Null, "with no sensor there is nothing whose age could be judged");
    }
}
