use crate::surveygrid::{boustrophedon, crossing_point, flatten, line_angle, reverse_internal_points, reverse_transect_order, set_angle, set_length, to_geo, with_turnaround};

const NO_SPACING_M: f64 = 100_000.0;
const MIN_SPACING_M: f64 = 0.5;

type Point = (f64, f64);
type Line = (Point, Point);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Params {
    pub width: f64,
    pub spacing: f64,
    pub turnaround: f64,
    pub entry: i64,
}

pub fn transect_spacing(spacing: f64) -> f64 {
    if spacing < MIN_SPACING_M { NO_SPACING_M } else { spacing }
}

pub fn transect_count(width: f64, spacing: f64) -> i64 {
    if width > 0.0 { (width / transect_spacing(spacing)).ceil() as i64 } else { 1 }
}

fn offset_polyline(flat: &[Point], origin: Point, distance: f64) -> Vec<Point> {
    if flat.len() < 2 {
        return Vec::new();
    }
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

pub fn transects(polyline: &[Point], params: &Params) -> Vec<Vec<Point>> {
    if polyline.len() < 2 {
        return Vec::new();
    }
    let spacing = transect_spacing(params.spacing);
    let count = transect_count(params.width, params.spacing);
    let half_width = params.width / 2.0;
    let flat = flatten(polyline);
    let origin = polyline[0];
    let laid: Vec<Vec<Point>> = (0..count)
        .map(|index| {
            let position = spacing / 2.0 + spacing * index as f64;
            let offset = if count == 1 { 0.0 } else { half_width - position };
            with_turnaround(offset_polyline(&flat, origin, offset), params.turnaround)
        })
        .collect();
    let ordered = if matches!(params.entry, 1 | 3) { reverse_transect_order(laid) } else { laid };
    let placed = if matches!(params.entry, 2 | 3) { reverse_internal_points(ordered) } else { ordered };
    boustrophedon(placed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn oracle() -> Value {
        let text = std::fs::read_to_string("../test/Bridge/fixtures/survey-transects.json").expect("the oracle is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the oracle is JSON")
    }

    fn spelled(transects: &[Vec<Point>]) -> String {
        transects.iter().flatten().map(|(lat, lon)| format!("{lat:.7},{lon:.7}")).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn every_recorded_corridor_is_generated_identically() {
        let cases = oracle();
        let cases = cases.as_object().expect("the oracle is an object of cases");
        let checked: Vec<(String, bool, String)> = cases
            .iter()
            .filter(|(_, case)| case["kind"] == "corridor")
            .map(|(name, case)| {
                let polyline: Vec<Point> = case["polyline"].as_array().unwrap().iter().map(|v| (v["latitude"].as_f64().unwrap(), v["longitude"].as_f64().unwrap())).collect();
                let params = Params {
                    width: case["corridorWidth"].as_f64().unwrap(),
                    spacing: case["gridSpacing"].as_f64().unwrap(),
                    turnaround: case["turnAround"].as_f64().unwrap(),
                    entry: case["entryPoint"].as_i64().unwrap(),
                };
                let expected = case["transects"].as_str().unwrap();
                let ours = spelled(&transects(&polyline, &params));
                (name.clone(), ours == expected, format!("{name}\n  qt:   {expected}\n  rust: {ours}"))
            })
            .collect();
        assert_eq!(checked.len(), 8, "every recorded corridor case is checked");
        let wrong: Vec<&str> = checked.iter().filter(|(_, matched, _)| !matched).map(|(_, _, report)| report.as_str()).collect();
        assert!(wrong.is_empty(), "{} of {} corridors differ from the Qt generator:\n{}", wrong.len(), checked.len(), wrong.join("\n"));
    }

    #[test]
    fn a_corridor_narrower_than_one_pass_is_flown_down_the_middle() {
        assert_eq!(transect_count(40.0, 60.0), 1, "a corridor narrower than the camera footprint is one pass");
        assert_eq!(transect_count(120.0, 60.0), 2);
        assert_eq!(transect_count(300.0, 60.0), 5);
        assert_eq!(transect_count(0.0, 60.0), 1, "a corridor with no width is still flown once");
        assert_eq!(transect_spacing(0.1), NO_SPACING_M, "a footprint too small to be real means one pass, as the Qt item treats it");
    }
}
