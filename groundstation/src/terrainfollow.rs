use crate::surveygrid::{Coord, Kind, at_distance_and_azimuth, azimuth_to, distance_between};

type Point = (f64, f64);

const RATE_SLACK: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Waypoint {
    pub coord: Coord,
    pub altitude: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adjust {
    pub distance_to_surface: f64,
    pub tolerance: f64,
    pub max_climb_rate: f64,
    pub max_descent_rate: f64,
    pub flight_speed: f64,
}

pub fn path_heights(from: Point, to: Point, height: &dyn Fn(f64, f64) -> Option<f64>) -> Option<Vec<f64>> {
    crate::terrain::samples(from, to).iter().map(|(lat, lon)| height(*lat, *lon)).collect()
}

fn interstitials(from: Point, to: Point, heights: &[f64], distance_to_surface: f64) -> Vec<Waypoint> {
    let count = heights.len();
    let azimuth = azimuth_to(from, to);
    let distance = distance_between(from, to);
    (1..count.saturating_sub(1))
        .map(|i| {
            let toward = (1.0 / (count - 1) as f64) * i as f64;
            Waypoint { coord: Coord { at: at_distance_and_azimuth(from, distance * toward, azimuth), kind: Kind::InteriorTerrainAdded }, altitude: heights[i] + distance_to_surface }
        })
        .collect()
}

pub fn flight_path(transects: &[Vec<Coord>], distance_to_surface: f64, height: &dyn Fn(f64, f64) -> Option<f64>) -> Option<Vec<Waypoint>> {
    let last_transect = transects.len().saturating_sub(1);
    let pieces: Option<Vec<Vec<Waypoint>>> = transects
        .iter()
        .enumerate()
        .map(|(index, transect)| {
            let legs: Option<Vec<Vec<Waypoint>>> = transect
                .windows(2)
                .enumerate()
                .map(|(leg, pair)| {
                    let heights = path_heights(pair[0].at, pair[1].at, height)?;
                    let start = (leg == 0).then(|| Waypoint { coord: pair[0], altitude: distance_to_surface + heights[0] });
                    let end = Waypoint { coord: pair[1], altitude: distance_to_surface + heights[heights.len() - 1] };
                    Some(start.into_iter().chain(interstitials(pair[0].at, pair[1].at, &heights, distance_to_surface)).chain(std::iter::once(end)).collect())
                })
                .collect();
            let turn = match (index < last_transect, transect.last(), transects.get(index + 1).and_then(|next| next.first())) {
                (true, Some(from), Some(to)) => interstitials(from.at, to.at, &path_heights(from.at, to.at, height)?, distance_to_surface),
                _ => Vec::new(),
            };
            Some(legs?.into_iter().flatten().chain(turn).collect())
        })
        .collect();
    pieces.map(|p| p.into_iter().flatten().collect())
}

fn rate(from: &Waypoint, to: &Waypoint, speed: f64) -> (f64, f64) {
    let seconds = distance_between(from.coord.at, to.coord.at) / speed;
    ((to.altitude - from.altitude) / seconds, seconds)
}

fn climb_pass(path: &[Waypoint], max_climb: f64, speed: f64) -> Option<Vec<Waypoint>> {
    let adjusted = (0..path.len().saturating_sub(1)).fold((path.to_vec(), false), |(current, changed), i| {
        let (climb, seconds) = rate(&current[i], &current[i + 1], speed);
        if climb > 0.0 && climb - max_climb > RATE_SLACK {
            let lowered = Waypoint { altitude: current[i + 1].altitude - max_climb * seconds, ..current[i] };
            (current.iter().enumerate().map(|(at, w)| if at == i { lowered } else { *w }).collect(), true)
        } else {
            (current, changed)
        }
    });
    adjusted.1.then_some(adjusted.0)
}

fn descent_pass(path: &[Waypoint], max_descent: f64, speed: f64) -> Option<Vec<Waypoint>> {
    let limit = -max_descent;
    let adjusted = (0..path.len().saturating_sub(1)).fold((path.to_vec(), false), |(current, changed), i| {
        let (descent, seconds) = rate(&current[i], &current[i + 1], speed);
        if descent < 0.0 && descent - limit < -RATE_SLACK {
            let raised = Waypoint { altitude: current[i].altitude + limit * seconds, ..current[i + 1] };
            (current.iter().enumerate().map(|(at, w)| if at == i + 1 { raised } else { *w }).collect(), true)
        } else {
            (current, changed)
        }
    });
    adjusted.1.then_some(adjusted.0)
}

fn until_settled(path: Vec<Waypoint>, pass: impl Fn(&[Waypoint]) -> Option<Vec<Waypoint>>) -> Vec<Waypoint> {
    std::iter::successors(Some(path), |current| pass(current)).last().unwrap_or_default()
}

pub fn adjust_for_max_rates(path: Vec<Waypoint>, adjust: &Adjust) -> Vec<Waypoint> {
    let speed = adjust.flight_speed;
    if speed.is_nan() || (adjust.max_climb_rate <= 0.0 && adjust.max_descent_rate <= 0.0) {
        return path;
    }
    let climbed = match adjust.max_climb_rate > 0.0 {
        true => until_settled(path, |p| climb_pass(p, adjust.max_climb_rate, speed)),
        false => path,
    };
    match adjust.max_descent_rate > 0.0 {
        true => until_settled(climbed, |p| descent_pass(p, adjust.max_descent_rate, speed)),
        false => climbed,
    }
}

pub fn adjust_for_tolerance(path: &[Waypoint], tolerance: f64) -> Vec<Waypoint> {
    let Some(first) = path.first() else { return Vec::new() };
    path[1..]
        .iter()
        .fold((vec![*first], *first), |(kept, last), next| {
            if next.coord.kind != Kind::InteriorTerrainAdded || (last.altitude - next.altitude).abs() > tolerance {
                ([kept, vec![*next]].concat(), *next)
            } else {
                (kept, last)
            }
        })
        .0
}

pub fn follow(transects: &[Vec<Coord>], adjust: &Adjust, height: &dyn Fn(f64, f64) -> Option<f64>) -> Option<Vec<Waypoint>> {
    let path = flight_path(transects, adjust.distance_to_surface, height)?;
    Some(adjust_for_tolerance(&adjust_for_max_rates(path, adjust), adjust.tolerance))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coord(at: Point, kind: Kind) -> Coord {
        Coord { at, kind }
    }

    fn adjust() -> Adjust {
        Adjust { distance_to_surface: 50.0, tolerance: 10.0, max_climb_rate: 0.0, max_descent_rate: 0.0, flight_speed: 5.0 }
    }

    #[test]
    fn the_path_rides_the_ground_and_adds_points_where_the_ground_changes() {
        let transects = vec![vec![coord((47.0, 8.0), Kind::SurveyEntry), coord((47.0, 8.004), Kind::SurveyExit)]];
        let ground = |_: f64, lon: f64| Some(if lon > 8.002 { 120.0 } else { 100.0 });
        let path = flight_path(&transects, 50.0, &ground).unwrap();
        assert_eq!(path.first().unwrap().altitude, 150.0);
        assert_eq!(path.last().unwrap().altitude, 170.0);
        assert!(path[1..path.len() - 1].iter().all(|w| w.coord.kind == Kind::InteriorTerrainAdded));
        let kept = adjust_for_tolerance(&path, 10.0);
        assert_eq!(kept.iter().filter(|w| w.coord.kind == Kind::InteriorTerrainAdded).count(), 1, "flat ground within tolerance drops every added point but the step");
        assert_eq!(kept.first().unwrap().coord.kind, Kind::SurveyEntry);
        assert_eq!(kept.last().unwrap().coord.kind, Kind::SurveyExit, "transect points are never dropped");
    }

    #[test]
    fn unknown_ground_builds_nothing_until_the_tiles_arrive() {
        let transects = vec![vec![coord((47.0, 8.0), Kind::SurveyEntry), coord((47.0, 8.004), Kind::SurveyExit)]];
        assert!(flight_path(&transects, 50.0, &|_, _| None).is_none());
    }

    #[test]
    fn the_turn_between_transects_gets_terrain_points_but_no_end_points_of_its_own() {
        let transects = vec![
            vec![coord((47.0, 8.0), Kind::SurveyEntry), coord((47.0, 8.001), Kind::SurveyExit)],
            vec![coord((47.001, 8.001), Kind::SurveyEntry), coord((47.001, 8.0), Kind::SurveyExit)],
        ];
        let path = flight_path(&transects, 50.0, &|_, _| Some(0.0)).unwrap();
        let ends: Vec<Kind> = path.iter().map(|w| w.coord.kind).filter(|k| *k != Kind::InteriorTerrainAdded).collect();
        assert_eq!(ends, [Kind::SurveyEntry, Kind::SurveyExit, Kind::SurveyEntry, Kind::SurveyExit]);
        assert!(path.len() > 4, "the 111 m turn is sampled at terrain resolution");
    }

    #[test]
    fn a_climb_too_steep_for_the_vehicle_starts_earlier() {
        let path = vec![
            Waypoint { coord: coord((47.0, 8.0), Kind::SurveyEntry), altitude: 100.0 },
            Waypoint { coord: coord((47.0, 8.001), Kind::InteriorTerrainAdded), altitude: 100.0 },
            Waypoint { coord: coord((47.0, 8.002), Kind::SurveyExit), altitude: 200.0 },
        ];
        let leg = distance_between(path[1].coord.at, path[2].coord.at);
        let adjusted = adjust_for_max_rates(path.clone(), &Adjust { max_climb_rate: 2.0, ..adjust() });
        assert!((adjusted[1].altitude - (200.0 - 2.0 * leg / 5.0)).abs() < 1e-6, "{adjusted:?}");
        assert!(adjusted[0].altitude > 100.0, "the earlier leg is pulled up in turn");
        assert_eq!(adjusted[2].altitude, 200.0, "the far end keeps its clearance");
        let descending: Vec<Waypoint> = path.iter().rev().copied().collect();
        let eased = adjust_for_max_rates(descending, &Adjust { max_descent_rate: 2.0, ..adjust() });
        assert!(eased[1].altitude > 100.0 && eased[2].altitude > 100.0, "a steep descent is eased by holding altitude longer: {eased:?}");
        assert_eq!(adjust_for_max_rates(path.clone(), &Adjust { flight_speed: f64::NAN, max_climb_rate: 2.0, ..adjust() }), path, "QGC skips the rate pass without a speed");
    }
}
