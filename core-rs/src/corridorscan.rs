use crate::surveygrid::{Coord, retype, typed};

const NO_SPACING_M: f64 = 100_000.0;
const MIN_SPACING_M: f64 = 0.5;

type Point = (f64, f64);

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

pub fn typed_transects(polyline: &[Point], params: &Params) -> Vec<Vec<Coord>> {
    if polyline.len() < 2 {
        return Vec::new();
    }
    let spacing = transect_spacing(params.spacing);
    let count = transect_count(params.width, params.spacing);
    let half_width = params.width / 2.0;
    let laid: Vec<Vec<Coord>> = (0..count)
        .map(|index| {
            let position = spacing / 2.0 + spacing * index as f64;
            let offset = if count == 1 { 0.0 } else { half_width - position };
            typed(crate::mappolyline::offset(polyline, offset), params.turnaround)
        })
        .collect();
    let ordered = if matches!(params.entry, 1 | 3) { laid.into_iter().rev().collect() } else { laid };
    let placed: Vec<Vec<Coord>> = if matches!(params.entry, 2 | 3) { ordered.into_iter().map(|t| t.into_iter().rev().collect()).collect() } else { ordered };
    placed
        .into_iter()
        .enumerate()
        .map(|(index, transect)| if index % 2 == 1 { transect.into_iter().rev().collect::<Vec<Coord>>() } else { transect })
        .map(retype)
        .collect()
}

pub fn flat_transects(polyline: &[Point], params: &Params) -> Vec<Vec<Point>> {
    typed_transects(polyline, params).into_iter().map(|transect| transect.into_iter().map(|coord| coord.at).collect()).collect()
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
                let ours = spelled(&flat_transects(&polyline, &params));
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
