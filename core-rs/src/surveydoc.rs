use serde_json::{Value, json};

use crate::surveygrid::{self, Coord, Kind, Params};
use crate::surveyitems::{self, Plan};

type Point = (f64, f64);

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub fn polygon(survey: &Value) -> Vec<Point> {
    survey.get("polygon").and_then(Value::as_array).map(|p| p.iter().filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))).collect()).unwrap_or_default()
}

fn camera_shots(transects: &[Vec<Coord>], trigger_distance: f64, in_turnaround: bool, complex_distance: f64) -> i64 {
    match (trigger_distance == 0.0, in_turnaround) {
        (true, _) => 0,
        (false, true) => (complex_distance / trigger_distance).ceil() as i64,
        (false, false) => transects
            .iter()
            .filter_map(|transect| {
                let cameras: Vec<&Coord> = transect.iter().filter(|c| c.kind != Kind::Turnaround).collect();
                Some((cameras.first()?.at, cameras.last()?.at))
            })
            .map(|(first, last)| (surveygrid::distance_between(first, last) / trigger_distance).ceil() as i64)
            .sum(),
    }
}

pub fn regenerate(survey: &Value) -> Value {
    let transect = survey.get("TransectStyleComplexItem").cloned().unwrap_or(Value::Null);
    let calc = transect.get("CameraCalc").cloned().unwrap_or(Value::Null);
    let params = Params {
        grid_angle: number(survey, "angle").unwrap_or(0.0),
        grid_spacing: number(&calc, "AdjustedFootprintSide").unwrap_or(0.0),
        turnaround: number(&transect, "TurnAroundDistance").unwrap_or(0.0),
        refly: flag(&transect, "Refly90Degrees"),
        alternate: flag(survey, "flyAlternateTransects"),
        entry: survey.get("entryLocation").and_then(Value::as_i64).unwrap_or(0),
    };
    let transects = surveygrid::typed_transects(&polygon(survey), &params);
    let trigger_distance = f64::from(number(&calc, "AdjustedFootprintFrontal").unwrap_or(0.0) as f32);
    let in_turnaround = flag(&transect, "CameraTriggerInTurnAround");
    let plan = Plan {
        altitude: number(&calc, "DistanceToSurface").unwrap_or(0.0),
        trigger_distance,
        altitude_mode: calc.get("DistanceMode").and_then(Value::as_i64).unwrap_or(crate::altitudemodes::RELATIVE),
        images_in_turnaround: in_turnaround,
    };
    let visual: Vec<Point> = transects.iter().flatten().map(|c| c.at).collect();
    let complex_distance: f64 = visual.windows(2).map(|pair| surveygrid::distance_between(pair[0], pair[1])).sum();
    let items: Vec<Value> = surveyitems::items(&transects, &plan)
        .iter()
        .enumerate()
        .map(|(i, item)| json!({ "autoContinue": true, "command": item.command, "doJumpId": i + 1, "frame": item.frame, "params": item.params, "type": "SimpleItem" }))
        .collect();
    let mut rebuilt = transect.clone();
    rebuilt["Items"] = Value::Array(items);
    rebuilt["VisualTransectPoints"] = json!(visual.iter().map(|(lat, lon)| json!([lat, lon])).collect::<Vec<_>>());
    rebuilt["CameraShots"] = json!(camera_shots(&transects, trigger_distance, in_turnaround, complex_distance));
    let mut changed = survey.clone();
    changed["TransectStyleComplexItem"] = rebuilt;
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_value(value: &Value) -> Value {
        match value {
            Value::Number(n) => json!((n.as_f64().unwrap() * 1e9).round() / 1e9),
            Value::Array(items) => Value::Array(items.iter().map(by_value).collect()),
            Value::Object(fields) => Value::Object(fields.iter().map(|(k, v)| (k.clone(), by_value(v))).collect()),
            other => other.clone(),
        }
    }

    #[test]
    fn a_survey_regenerates_the_items_points_and_shots_qt_saved_for_it() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let saved = plan["mission"]["items"][0].clone();
        let rebuilt = regenerate(&saved);
        ["Items", "VisualTransectPoints", "CameraShots"].iter().for_each(|key| {
            assert_eq!(by_value(&rebuilt["TransectStyleComplexItem"][key]), by_value(&saved["TransectStyleComplexItem"][key]), "{key}");
        });
    }
}
