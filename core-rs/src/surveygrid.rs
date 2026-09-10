use crate::geo::{geo_to_ned, ned_to_geo};

const EARTH_MEAN_RADIUS_M: f64 = 6_371_007.2;
const NO_SPACING_M: f64 = 100_000.0;
const MIN_SPACING_M: f64 = 0.5;
const GRID_MARGIN_M: f64 = 2000.0;
const DIRECTION_TOLERANCE_DEG: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Params {
    pub grid_angle: f64,
    pub grid_spacing: f64,
    pub turnaround: f64,
    pub refly: bool,
    pub alternate: bool,
}

type Point = (f64, f64);
type Line = (Point, Point);

fn fuzzy_is_null(value: f64) -> bool {
    value.abs() <= 1e-12
}

fn fuzzy_compare(a: f64, b: f64) -> bool {
    (a - b).abs() * 1_000_000_000_000.0 <= a.abs().min(b.abs())
}

fn same_point(a: Point, b: Point) -> bool {
    let same = |x: f64, y: f64| if x == 0.0 || y == 0.0 { fuzzy_is_null(x - y) } else { fuzzy_compare(x, y) };
    same(a.0, b.0) && same(a.1, b.1)
}

fn line_angle(line: Line) -> f64 {
    let (dx, dy) = (line.1.0 - line.0.0, line.1.1 - line.0.1);
    let theta = (-dy).atan2(dx).to_degrees();
    let normalised = if theta < 0.0 { theta + 360.0 } else { theta };
    if fuzzy_compare(normalised, 360.0) { 0.0 } else { normalised }
}

fn bounded_intersection(first: Line, second: Line) -> Option<Point> {
    let a = (first.1.0 - first.0.0, first.1.1 - first.0.1);
    let b = (second.0.0 - second.1.0, second.0.1 - second.1.1);
    let c = (first.0.0 - second.0.0, first.0.1 - second.0.1);
    let denominator = a.1 * b.0 - a.0 * b.1;
    if denominator == 0.0 || !denominator.is_finite() {
        return None;
    }
    let reciprocal = 1.0 / denominator;
    let na = (b.1 * c.0 - b.0 * c.1) * reciprocal;
    if !(0.0..=1.0).contains(&na) {
        return None;
    }
    let nb = (a.0 * c.1 - a.1 * c.0) * reciprocal;
    if !(0.0..=1.0).contains(&nb) {
        return None;
    }
    Some((first.0.0 + a.0 * na, first.0.1 + a.1 * na))
}

fn rotate(point: Point, origin: Point, angle_deg: f64) -> Point {
    let radians = (std::f64::consts::PI / 180.0) * -angle_deg;
    let (dx, dy) = (point.0 - origin.0, point.1 - origin.1);
    (dx * radians.cos() - dy * radians.sin() + origin.0, dx * radians.sin() + dy * radians.cos() + origin.1)
}

pub fn clamp_grid_angle_90(angle: f64) -> f64 {
    if angle > 90.0 {
        angle - 180.0
    } else if angle < -90.0 {
        angle + 180.0
    } else {
        angle
    }
}

pub fn azimuth_to(from: Point, to: Point) -> f64 {
    let d_lon = (to.1 - from.1).to_radians();
    let (from_lat, to_lat) = (from.0.to_radians(), to.0.to_radians());
    let y = d_lon.sin() * to_lat.cos();
    let x = from_lat.cos() * to_lat.sin() - from_lat.sin() * to_lat.cos() * d_lon.cos();
    let azimuth = y.atan2(x).to_degrees();
    let whole = azimuth.trunc();
    ((whole as i64 + 360) % 360) as f64 + (azimuth - whole)
}

pub fn at_distance_and_azimuth(from: Point, distance: f64, azimuth: f64) -> Point {
    let (lat_rad, lon_rad) = (from.0.to_radians(), from.1.to_radians());
    let (cos_lat, sin_lat) = (lat_rad.cos(), lat_rad.sin());
    let azimuth_rad = azimuth.to_radians();
    let ratio = distance / EARTH_MEAN_RADIUS_M;
    let (cos_ratio, sin_ratio) = (ratio.cos(), ratio.sin());
    let result_lat = (sin_lat * cos_ratio + cos_lat * sin_ratio * azimuth_rad.cos()).asin();
    let result_lon = lon_rad + (azimuth_rad.sin() * sin_ratio * cos_lat).atan2(cos_ratio - sin_lat * result_lat.sin());
    crate::geo::wrap(result_lat.to_degrees(), result_lon.to_degrees())
}

fn intersect_with_polygon(lines: &[Line], polygon: &[Point]) -> Vec<Line> {
    lines
        .iter()
        .filter_map(|line| {
            let crossings = polygon.windows(2).filter_map(|edge| bounded_intersection(*line, (edge[0], edge[1]))).fold(Vec::new(), |mut kept: Vec<Point>, at| {
                if !kept.iter().any(|seen| same_point(*seen, at)) {
                    kept.push(at);
                }
                kept
            });
            if crossings.len() < 2 {
                return None;
            }
            let widest = crossings
                .iter()
                .flat_map(|first| crossings.iter().map(move |second| (*first, *second)))
                .fold((0.0f64, None), |(longest, best), (first, second)| {
                    let length = (second.0 - first.0).hypot(second.1 - first.1);
                    if length > longest { (length, Some((first, second))) } else { (longest, best) }
                });
            widest.1
        })
        .collect()
}

fn adjust_line_direction(lines: &[Line]) -> Vec<Line> {
    let first_angle = lines.first().map(|line| line_angle(*line)).unwrap_or(0.0);
    lines
        .iter()
        .map(|line| if (line_angle(*line) - first_angle).abs() > DIRECTION_TOLERANCE_DEG { (line.1, line.0) } else { *line })
        .collect()
}

fn alternate_order(transects: Vec<Vec<Point>>) -> Vec<Vec<Point>> {
    let forward = transects.iter().enumerate().filter(|(i, _)| i % 2 == 0).map(|(_, t)| t.clone());
    let back = transects.iter().enumerate().rev().filter(|(i, _)| *i > 0 && i % 2 == 1).map(|(_, t)| t.clone());
    forward.chain(back).collect()
}

fn boustrophedon(transects: Vec<Vec<Point>>) -> Vec<Vec<Point>> {
    transects
        .into_iter()
        .enumerate()
        .map(|(index, transect)| if index % 2 == 1 { transect.into_iter().rev().collect() } else { transect })
        .collect()
}

fn with_turnaround(transect: Vec<Point>, distance: f64) -> Vec<Point> {
    if distance <= 0.0 || transect.len() < 2 {
        return transect;
    }
    let entry = at_distance_and_azimuth(transect[0], -distance, azimuth_to(transect[0], transect[1]));
    let last = transect[transect.len() - 1];
    let exit = at_distance_and_azimuth(last, -distance, azimuth_to(last, transect[transect.len() - 2]));
    std::iter::once(entry).chain(transect).chain(std::iter::once(exit)).collect()
}

pub fn transects(polygon: &[Point], params: &Params) -> Vec<Vec<Point>> {
    if polygon.len() < 3 {
        return Vec::new();
    }
    let origin = (polygon[0].0, polygon[0].1, 0.0);
    let flat: Vec<Point> = polygon
        .iter()
        .enumerate()
        .map(|(index, vertex)| {
            if index == 0 {
                return (0.0, 0.0);
            }
            let (north, east, _) = geo_to_ned(vertex.0, vertex.1, 0.0, origin);
            (east, north)
        })
        .collect();
    let closed: Vec<Point> = flat.iter().copied().chain(std::iter::once(flat[0])).collect();

    let spacing = if params.grid_spacing < MIN_SPACING_M { NO_SPACING_M } else { params.grid_spacing };
    let angle = clamp_grid_angle_90(params.grid_angle) + if params.refly { 90.0 } else { 0.0 };

    let xs = closed.iter().map(|p| p.0);
    let ys = closed.iter().map(|p| p.1);
    let (min_x, max_x) = xs.clone().fold((f64::MAX, f64::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
    let (min_y, max_y) = ys.clone().fold((f64::MAX, f64::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)));
    let centre = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
    let max_width = (max_x - min_x).max(max_y - min_y) + GRID_MARGIN_M;
    let half_width = max_width / 2.0;

    let count = ((max_width / spacing).ceil() as i64).max(1);
    let lines: Vec<Line> = (0..count)
        .map(|step| centre.0 - half_width + spacing * step as f64)
        .take_while(|x| *x < centre.0 - half_width + max_width)
        .map(|x| (rotate((x, centre.1 - half_width), centre, angle), rotate((x, centre.1 + half_width), centre, angle)))
        .collect();

    let crossed = intersect_with_polygon(&lines, &closed);
    let ordered = adjust_line_direction(&crossed);
    let geo: Vec<Vec<Point>> = ordered
        .iter()
        .map(|line| {
            [line.0, line.1]
                .iter()
                .map(|end| {
                    let (lat, lon, _) = ned_to_geo(end.1, end.0, 0.0, origin);
                    (lat, lon)
                })
                .collect()
        })
        .collect();

    let arranged = if params.alternate { alternate_order(geo) } else { geo };
    boustrophedon(arranged).into_iter().map(|transect| with_turnaround(transect, params.turnaround)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn oracle() -> Value {
        let text = std::fs::read_to_string("../test/Bridge/fixtures/survey-transects.json").expect("the survey oracle is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the survey oracle is JSON")
    }

    fn spelled(transects: &[Vec<Point>]) -> String {
        transects.iter().flatten().map(|(lat, lon)| format!("{lat:.7},{lon:.7}")).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn every_recorded_case_is_generated_identically() {
        let cases = oracle();
        let cases = cases.as_object().expect("the oracle is an object of cases");
        assert!(cases.len() >= 12, "the oracle should carry every recorded case");
        let checked: Vec<(String, bool, String)> = cases
            .iter()
            .filter(|(name, _)| !name.contains("refly"))
            .map(|(name, case)| {
                let polygon: Vec<Point> = case["polygon"].as_array().unwrap().iter().map(|v| (v["latitude"].as_f64().unwrap(), v["longitude"].as_f64().unwrap())).collect();
                let params = Params {
                    grid_angle: case["gridAngle"].as_f64().unwrap(),
                    grid_spacing: case["gridSpacing"].as_f64().unwrap(),
                    turnaround: case["turnAround"].as_f64().unwrap(),
                    refly: false,
                    alternate: false,
                };
                let expected = case["transects"].as_str().unwrap();
                let ours = spelled(&transects(&polygon, &params));
                (name.clone(), ours == expected, format!("{name}\n  qt:   {expected}\n  rust: {ours}"))
            })
            .collect();
        assert_eq!(checked.len(), 10, "every case that is not a refly is checked");
        let wrong: Vec<&str> = checked.iter().filter(|(_, matched, _)| !matched).map(|(_, _, report)| report.as_str()).collect();
        assert!(wrong.is_empty(), "{} of {} cases differ from the Qt generator:\n{}", wrong.len(), checked.len(), wrong.join("\n"));
    }
}
