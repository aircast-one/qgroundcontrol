use crate::surveygrid::{crossing_point, flatten, line_angle, set_angle, set_length, to_geo};

type Point = (f64, f64);
type Line = (Point, Point);

pub fn offset(polygon: &[Point], distance: f64) -> Option<Vec<Point>> {
    if polygon.len() < 3 {
        return None;
    }
    let flat = flatten(polygon);
    let edges: Vec<Line> = (0..flat.len())
        .map(|index| {
            let original: Line = (flat[index], flat[(index + 1) % flat.len()]);
            let forward = set_length(original, distance);
            let start = set_angle(forward, line_angle(forward) - 90.0);
            let backward = set_length((original.1, original.0), distance);
            let end = set_angle(backward, line_angle(backward) + 90.0);
            (start.1, end.1)
        })
        .collect();
    (0..edges.len())
        .map(|index| {
            let previous = if index == 0 { edges.len() - 1 } else { index - 1 };
            crossing_point(edges[previous], edges[index]).map(|corner| to_geo(corner, polygon[0]))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn oracle() -> Value {
        let text = std::fs::read_to_string("../test/Bridge/fixtures/survey-transects.json").expect("the oracle is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the oracle is JSON")
    }

    fn spelled(points: &[Point]) -> String {
        points.iter().map(|(lat, lon)| format!("{lat:.7},{lon:.7}")).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn every_recorded_offset_moves_the_polygon_identically() {
        let cases = oracle();
        let cases = cases.as_object().expect("the oracle is an object of cases");
        let checked: Vec<(String, bool, String)> = cases
            .iter()
            .filter(|(_, case)| case["kind"] == "offset")
            .map(|(name, case)| {
                let polygon: Vec<Point> = case["polygon"].as_array().unwrap().iter().map(|v| (v["latitude"].as_f64().unwrap(), v["longitude"].as_f64().unwrap())).collect();
                let expected = case["moved"].as_str().unwrap();
                let ours = offset(&polygon, case["offset"].as_f64().unwrap()).map(|moved| spelled(&moved)).unwrap_or_default();
                (name.clone(), ours == expected, format!("{name}\n  qt:   {expected}\n  rust: {ours}"))
            })
            .collect();
        assert_eq!(checked.len(), 5, "every recorded offset case is checked");
        let wrong: Vec<&str> = checked.iter().filter(|(_, matched, _)| !matched).map(|(_, _, report)| report.as_str()).collect();
        assert!(wrong.is_empty(), "{} of {} offsets differ from the Qt polygon:\n{}", wrong.len(), checked.len(), wrong.join("\n"));
    }

    #[test]
    fn a_positive_offset_walks_the_edges_to_the_right_of_their_direction() {
        let square = [(47.3960, 8.5440), (47.3960, 8.5480), (47.3990, 8.5480), (47.3990, 8.5440)];
        let smaller = offset(&square, 40.0).unwrap();
        let larger = offset(&square, -40.0).unwrap();
        assert!(smaller[0].0 > square[0].0 && smaller[0].1 > square[0].1, "on this winding a positive distance pulls the polygon in, which is the opposite of what the word offset suggests");
        assert!(larger[0].0 < square[0].0 && larger[0].1 < square[0].1);
        assert_eq!(smaller.len(), square.len(), "an offset polygon keeps its corners rather than collapsing them");
    }

    #[test]
    fn a_shape_with_no_area_cannot_be_offset() {
        assert!(offset(&[(47.0, 8.0), (47.1, 8.0)], 10.0).is_none(), "two points are a line, and a line has no inside to move");
        assert!(offset(&[], 10.0).is_none());
        assert!(offset(&[(47.0, 8.0), (47.1, 8.0), (47.1, 8.1)], 10.0).is_some());
    }
}
