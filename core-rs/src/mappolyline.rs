use crate::surveygrid::{at_distance_and_azimuth, azimuth_to, crossing_point, distance_between, flatten, line_angle, set_angle, set_length, to_geo};

type Point = (f64, f64);
type Line = (Point, Point);

pub fn length(polyline: &[Point]) -> f64 {
    polyline.windows(2).map(|pair| distance_between(pair[0], pair[1])).sum()
}

pub fn offset(polyline: &[Point], distance: f64) -> Vec<Point> {
    if polyline.len() < 2 {
        return Vec::new();
    }
    let flat = flatten(polyline);
    let origin = polyline[0];
    let edges: Vec<Line> = flat
        .windows(2)
        .map(|edge| {
            let original: Line = (edge[0], edge[1]);
            let forward = set_length(original, distance);
            let start = set_angle(forward, line_angle(forward) - 90.0);
            let backward = set_length((original.1, original.0), distance);
            let end = set_angle(backward, line_angle(backward) + 90.0);
            (start.1, end.1)
        })
        .collect();
    let joints = edges.windows(2).map(|pair| crossing_point(pair[0], pair[1]).unwrap_or(pair[1].1));
    std::iter::once(edges[0].0)
        .chain(joints)
        .chain(std::iter::once(edges[edges.len() - 1].1))
        .map(|point| to_geo(point, origin))
        .collect()
}

pub fn split_segment(polyline: &[Point], index: usize) -> Option<Vec<Point>> {
    let next = index + 1;
    if next > polyline.len().saturating_sub(1) {
        return None;
    }
    let (from, to) = (polyline[index], polyline[next]);
    let middle = at_distance_and_azimuth(from, distance_between(from, to) / 2.0, azimuth_to(from, to));
    Some(polyline.iter().take(next).copied().chain(std::iter::once(middle)).chain(polyline.iter().skip(next).copied()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> Vec<Point> {
        vec![(47.3960, 8.5440), (47.3980, 8.5440), (47.3980, 8.5480)]
    }

    #[test]
    fn the_length_is_the_walk_along_the_line_not_around_it() {
        let walked = length(&line());
        let legs = distance_between(line()[0], line()[1]) + distance_between(line()[1], line()[2]);
        assert!((walked - legs).abs() < 1e-9);
        assert!(walked > distance_between(line()[0], line()[2]), "a polyline is not closed, so its length is longer than the straight line across it");
        assert_eq!(length(&[(47.0, 8.0)]), 0.0, "one point is nowhere to walk");
        assert_eq!(length(&[]), 0.0);
    }

    #[test]
    fn an_offset_line_keeps_a_vertex_for_every_vertex() {
        let moved = offset(&line(), 30.0);
        assert_eq!(moved.len(), line().len(), "the ends are carried over and every corner becomes one crossing");
        let back = offset(&line(), -30.0);
        assert_ne!(moved[0], back[0], "the two directions land on opposite sides of the line");
        assert!(offset(&[(47.0, 8.0)], 30.0).is_empty(), "one point has no direction to be offset from");
        assert!(offset(&[], 30.0).is_empty());
    }

    #[test]
    fn an_offset_line_runs_parallel_at_the_distance_asked_for() {
        let straight = vec![(47.3960, 8.5440), (47.3990, 8.5440)];
        let moved = offset(&straight, 50.0);
        assert_eq!(moved.len(), 2);
        assert!((distance_between(straight[0], moved[0]) - 50.0).abs() < 0.5, "an end of the offset line sits the offset distance from the end it came from");
        assert!((distance_between(straight[1], moved[1]) - 50.0).abs() < 0.5);
        assert!((length(&moved) - length(&straight)).abs() < 0.5, "offsetting a straight line does not change its length");
    }

    #[test]
    fn splitting_a_segment_puts_a_vertex_at_its_middle() {
        let split = split_segment(&line(), 0).unwrap();
        assert_eq!(split.len(), line().len() + 1);
        assert_eq!(split[0], line()[0], "the vertices either side of the split are untouched");
        assert_eq!(split[2], line()[1]);
        let first = distance_between(split[0], split[1]);
        let second = distance_between(split[1], split[2]);
        assert!((first - second).abs() < 1e-6, "the new vertex is the middle of the segment rather than somewhere along it");
        assert!((first + second - distance_between(line()[0], line()[1])).abs() < 1e-6);
    }

    #[test]
    fn there_is_nothing_to_split_past_the_last_vertex() {
        assert!(split_segment(&line(), 1).is_some());
        assert!(split_segment(&line(), 2).is_none(), "the last vertex begins no segment");
        assert!(split_segment(&line(), 9).is_none());
        assert!(split_segment(&[(47.0, 8.0)], 0).is_none());
        assert!(split_segment(&[], 0).is_none());
    }
}
