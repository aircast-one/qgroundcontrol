use crate::geo::geo_to_ned;
use crate::surveygrid::{at_distance_and_azimuth, azimuth_to, crossing_point, distance_between, flatten, line_angle, set_angle, set_length, to_geo};

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

pub fn area(polygon: &[Point]) -> f64 {
    if polygon.len() < 3 {
        return 0.0;
    }
    let flat = flatten(polygon);
    let last = flat[flat.len() - 1];
    let sum: f64 = flat
        .iter()
        .enumerate()
        .map(|(index, at)| {
            let previous = if index == 0 { last } else { flat[index - 1] };
            previous.0 * at.1 - at.0 * previous.1
        })
        .sum();
    0.5 * sum.abs()
}

fn screen_point(coord: Point, origin: Point) -> Point {
    let (north, east, _) = geo_to_ned(coord.0, coord.1, 0.0, (origin.0, origin.1, 0.0));
    (east, -north)
}

fn fuzzy_same(left: f64, right: f64) -> bool {
    (left - right).abs() * 1_000_000_000_000.0 <= left.abs().min(right.abs())
}

fn crossings(from: Point, to: Point, at: Point) -> i32 {
    if fuzzy_same(from.1, to.1) {
        return 0;
    }
    let ((x1, y1), (x2, y2), direction) = if to.1 < from.1 { (to, from, -1) } else { (from, to, 1) };
    if at.1 < y1 || at.1 >= y2 {
        return 0;
    }
    let crossing = x1 + ((x2 - x1) / (y2 - y1)) * (at.1 - y1);
    if crossing <= at.0 { direction } else { 0 }
}

pub fn contains(polygon: &[Point], coordinate: Point) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let origin = polygon[0];
    let flat: Vec<Point> = polygon.iter().map(|vertex| screen_point(*vertex, origin)).collect();
    let probe = screen_point(coordinate, origin);
    let along: i32 = flat.windows(2).map(|edge| crossings(edge[0], edge[1], probe)).sum();
    let last = flat[flat.len() - 1];
    let closing = if last != flat[0] { crossings(last, flat[0], probe) } else { 0 };
    (along + closing) % 2 != 0
}

pub fn split_segment(polygon: &[Point], index: usize) -> Option<Vec<Point>> {
    if index >= polygon.len() {
        return None;
    }
    let next = (index + 1) % polygon.len();
    let (from, to) = (polygon[index], polygon[next]);
    let middle = at_distance_and_azimuth(from, distance_between(from, to) / 2.0, azimuth_to(from, to));
    Some(match next {
        0 => polygon.iter().copied().chain(std::iter::once(middle)).collect(),
        _ => polygon.iter().take(next).copied().chain(std::iter::once(middle)).chain(polygon.iter().skip(next).copied()).collect(),
    })
}

pub fn wound_clockwise(polygon: &[Point]) -> Vec<Point> {
    if polygon.len() <= 2 {
        return polygon.to_vec();
    }
    let sum: f64 = (0..polygon.len())
        .map(|index| {
            let next = polygon[(index + 1) % polygon.len()];
            (next.1 - polygon[index].1) * (next.0 + polygon[index].0)
        })
        .sum();
    if sum < 0.0 { polygon.iter().rev().copied().collect() } else { polygon.to_vec() }
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

#[cfg(test)]
mod geometry {
    use super::*;
    use serde_json::Value;

    fn oracle() -> Value {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../test/Bridge/fixtures/polygon-geometry.json")).expect("the oracle is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the oracle is JSON")
    }

    fn shapes() -> Vec<(String, Vec<Point>, Value)> {
        oracle()
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, case)| {
                let polygon: Vec<Point> = case["polygon"].as_array().unwrap().iter().map(|v| (v["latitude"].as_f64().unwrap(), v["longitude"].as_f64().unwrap())).collect();
                (name.clone(), polygon, case.clone())
            })
            .collect()
    }

    fn spelled(points: &[Point]) -> String {
        points.iter().map(|(lat, lon)| format!("{lat:.7},{lon:.7}")).collect::<Vec<_>>().join(" ")
    }

    const PROBES: [(f64, f64); 16] = [
        (47.3975, 8.5460),
        (47.3960, 8.5440),
        (47.3990, 8.5480),
        (47.3985, 8.5490),
        (47.3970, 8.5485),
        (47.3999, 8.5460),
        (47.3950, 8.5460),
        (47.3980, 8.5475),
        (47.3960, 8.5460),
        (47.3990, 8.5460),
        (47.3975, 8.5440),
        (47.3975, 8.5480),
        (47.3960, 8.5500),
        (47.3980, 8.5470),
        (47.3995, 8.5470),
        (47.3975, 8.5470),
    ];

    #[test]
    fn every_recorded_polygon_covers_the_same_area() {
        let wrong: Vec<String> = shapes()
            .iter()
            .map(|(name, polygon, case)| (name, format!("{:.4}", area(polygon)), case["area"].as_str().unwrap().to_string()))
            .filter(|(_, ours, theirs)| ours != theirs)
            .map(|(name, ours, theirs)| format!("{name} qt {theirs} rust {ours}"))
            .collect();
        assert!(wrong.is_empty(), "{wrong:?}");
        assert_eq!(shapes().len(), 3);
        assert_eq!(area(&[(47.0, 8.0), (47.1, 8.0)]), 0.0, "a shape with no inside covers nothing");
    }

    #[test]
    fn every_recorded_probe_falls_the_same_side_of_the_edge() {
        let wrong: Vec<String> = shapes()
            .iter()
            .flat_map(|(name, polygon, case)| {
                let expected = case["contains"].as_array().unwrap().clone();
                assert_eq!(expected.len(), PROBES.len(), "{name} was recorded with a different probe set than the one checked here");
                PROBES
                    .iter()
                    .enumerate()
                    .map(|(index, probe)| (format!("{name} {probe:?}"), contains(polygon, *probe), expected[index].as_bool().unwrap()))
                    .filter(|(_, ours, theirs)| ours != theirs)
                    .map(|(where_, ours, theirs)| format!("{where_} qt {theirs} rust {ours}"))
                    .collect::<Vec<String>>()
            })
            .collect();
        assert!(wrong.is_empty(), "{} probes fall differently, and the boundary cases are the point:\n{}", wrong.len(), wrong.join("\n"));
        let answers: Vec<bool> = shapes().iter().flat_map(|(_, polygon, _)| PROBES.iter().map(|probe| contains(polygon, *probe)).collect::<Vec<bool>>()).collect();
        assert!(answers.iter().any(|inside| *inside) && answers.iter().any(|inside| !inside), "the recorded answers are a mix rather than all one way");
        assert!(!contains(&[(47.0, 8.0), (47.1, 8.0)], (47.05, 8.0)), "two points enclose nothing");
    }

    #[test]
    fn every_recorded_split_lands_in_the_same_place() {
        let wrong: Vec<String> = shapes()
            .iter()
            .flat_map(|(name, polygon, case)| {
                let expected = case["splits"].as_object().unwrap().clone();
                (0..polygon.len())
                    .map(|vertex| {
                        let ours = spelled(&split_segment(polygon, vertex).unwrap());
                        (format!("{name} split at {vertex}"), ours, expected[&vertex.to_string()].as_str().unwrap().to_string())
                    })
                    .filter(|(_, ours, theirs)| ours != theirs)
                    .map(|(where_, ours, theirs)| format!("{where_}\n  qt:   {theirs}\n  rust: {ours}"))
                    .collect::<Vec<String>>()
            })
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        assert!(split_segment(&[(47.0, 8.0), (47.1, 8.0)], 2).is_none(), "there is no vertex to split from");
    }

    #[test]
    fn a_counter_clockwise_polygon_is_turned_around_and_a_clockwise_one_is_left_alone() {
        let wrong: Vec<String> = shapes()
            .iter()
            .flat_map(|(name, polygon, case)| {
                let reversed: Vec<Point> = polygon.iter().rev().copied().collect();
                vec![
                    (format!("{name} as given"), spelled(&wound_clockwise(polygon)), case["wound"].as_str().unwrap().to_string()),
                    (format!("{name} reversed"), spelled(&wound_clockwise(&reversed)), case["reversedThenWound"].as_str().unwrap().to_string()),
                ]
                .into_iter()
                .filter(|(_, ours, theirs)| ours != theirs)
                .map(|(where_, ours, theirs)| format!("{where_}\n  qt:   {theirs}\n  rust: {ours}"))
                .collect::<Vec<String>>()
            })
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        let reversed: Vec<Point> = shapes()[0].1.iter().rev().copied().collect();
        assert_eq!(wound_clockwise(&wound_clockwise(&reversed)), wound_clockwise(&reversed), "winding a polygon that is already wound changes nothing");
        assert_eq!(wound_clockwise(&[(47.0, 8.0), (47.1, 8.0)]).len(), 2, "a line has no winding to correct");
    }

    #[test]
    fn which_way_an_offset_moves_depends_on_the_winding_and_not_on_the_sign_alone() {
        let drawn = [(47.3960, 8.5440), (47.3960, 8.5480), (47.3990, 8.5480), (47.3990, 8.5440)];
        assert_ne!(wound_clockwise(&drawn), drawn.to_vec(), "this square is drawn the other way round, which is the case that matters");
        assert!(area(&offset(&drawn, 40.0).unwrap()) < area(&drawn), "on the drawn winding a positive distance pulls the shape in");

        let wound = wound_clockwise(&drawn);
        assert!(area(&offset(&wound, 40.0).unwrap()) > area(&wound), "on the wound one the same distance pushes it out, which is why anything that must end up outside has to wind first");
        assert!(area(&offset(&wound, -40.0).unwrap()) < area(&wound));
        assert!((area(&offset(&wound, 40.0).unwrap()) - area(&offset(&drawn, -40.0).unwrap())).abs() < 1e-6, "reversing the winding is the same as reversing the sign");
    }
}

#[cfg(test)]
mod equivalence {
    use super::*;

    fn variant(polygon: &[Point], coordinate: Point, mirror: bool, skip_flat: bool) -> bool {
        let origin = polygon[0];
        let place = |coord: Point| {
            let (north, east, _) = geo_to_ned(coord.0, coord.1, 0.0, (origin.0, origin.1, 0.0));
            (east, if mirror { -north } else { north })
        };
        let flat: Vec<Point> = polygon.iter().map(|vertex| place(*vertex)).collect();
        let probe = place(coordinate);
        let count = |from: Point, to: Point| -> i32 {
            if skip_flat && fuzzy_same(from.1, to.1) {
                return 0;
            }
            let ((x1, y1), (x2, y2), direction) = if to.1 < from.1 { (to, from, -1) } else { (from, to, 1) };
            if probe.1 < y1 || probe.1 >= y2 || y1 == y2 {
                return 0;
            }
            let crossing = x1 + ((x2 - x1) / (y2 - y1)) * (probe.1 - y1);
            if crossing <= probe.0 { direction } else { 0 }
        };
        let along: i32 = flat.windows(2).map(|edge| count(edge[0], edge[1])).sum();
        let last = flat[flat.len() - 1];
        let closing = if last != flat[0] { count(last, flat[0]) } else { 0 };
        (along + closing) % 2 != 0
    }

    #[test]
    fn no_probe_over_these_shapes_can_see_the_mirror_or_the_horizontal_edge_rule() {
        let square = vec![(47.3960, 8.5440), (47.3960, 8.5480), (47.3990, 8.5480), (47.3990, 8.5440)];
        let triangle = vec![(47.3960, 8.5440), (47.3960, 8.5500), (47.3995, 8.5470)];
        let concave = vec![(47.3960, 8.5440), (47.3960, 8.5500), (47.3980, 8.5500), (47.3975, 8.5470), (47.3995, 8.5470), (47.3995, 8.5440)];
        let shapes = [("square", square), ("triangle", triangle), ("concave", concave)];
        let found: Vec<String> = (473940..474010)
            .step_by(5)
            .flat_map(|lat| (85420..85520).step_by(5).map(move |lon| (lat as f64 / 10000.0, lon as f64 / 10000.0)))
            .flat_map(|probe| {
                shapes
                    .iter()
                    .filter_map(|(name, polygon)| {
                        let truth = variant(polygon, probe, true, true);
                        let no_mirror = variant(polygon, probe, false, true);
                        let no_skip = variant(polygon, probe, true, false);
                        (truth != no_mirror || truth != no_skip).then(|| format!("{name} ({:.4}, {:.4}) mirror {} skip {}", probe.0, probe.1, truth != no_mirror, truth != no_skip))
                    })
                    .collect::<Vec<String>>()
            })
            .collect();
        assert!(
            found.is_empty(),
            "{} probes now tell these forms apart, so the recorded oracle no longer covers what the code does; record these from Qt before trusting either:\n{}",
            found.len(),
            found.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
        );
    }
}
