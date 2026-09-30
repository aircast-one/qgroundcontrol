use serde_json::{Value, json};

use crate::read::value_number;
use crate::router::Backend;

pub const DEPS: &[&str] = &["settings.unitsSettings.horizontalDistanceUnits"];

const METRES: &[f64] = &[5.0, 10.0, 25.0, 50.0, 100.0, 150.0, 250.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0, 50000.0, 100000.0, 200000.0, 500000.0, 1000000.0, 2000000.0];
const FEET: &[f64] = &[10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2000.0, 3000.0, 4000.0, 5280.0, 10560.0, 26400.0, 52800.0, 132000.0, 264000.0, 528000.0, 1320000.0, 2640000.0, 5280000.0];
const FEET_PER_METRE: f64 = 3.2808399;
const HORIZONTAL_UNITS_FEET: f64 = 0.0;

pub fn snapped(measured: f64, steps: &[f64]) -> Option<(f64, f64)> {
    if !(measured > 0.0) || !measured.is_finite() {
        return None;
    }
    let chosen = steps
        .windows(2)
        .map(|pair| pair[0])
        .zip(steps.windows(2).map(|pair| (pair[0] + pair[1]) / 2.0))
        .find(|(_, midpoint)| measured < *midpoint)
        .map(|(step, _)| step)
        .or_else(|| steps.last().copied())?;
    Some((chosen, chosen / measured))
}

pub fn metric_text(metres: f64) -> String {
    let whole = metres.round();
    match whole {
        w if w <= 1000.0 => format!("{} m", w as i64),
        w if w > 100000.0 => format!("{} km", (w / 1000.0).round() as i64),
        w => format!("{} km", (w / 100.0).round() / 10.0),
    }
}

pub fn imperial_text(feet: f64) -> String {
    let whole = feet.round();
    match whole {
        w if w < 5280.0 => format!("{} ft", w as i64),
        w => {
            let miles = (w / 5280.0).round() as i64;
            format!("{miles} mile{}", if miles == 1 { "" } else { "s" })
        }
    }
}

pub fn bar(metres_across: f64, imperial: bool) -> Option<(String, f64)> {
    match imperial {
        true => snapped(metres_across * FEET_PER_METRE, FEET).map(|(v, r)| (imperial_text(v), r)),
        false => snapped(metres_across, METRES).map(|(v, r)| (metric_text(v), r)),
    }
}

pub fn map_scale_view(backend: &dyn Backend, args: &[String]) -> Value {
    let imperial = value_number(&backend.get("settings.unitsSettings.horizontalDistanceUnits.rawValue")) == Some(HORIZONTAL_UNITS_FEET);
    let across = args.first().and_then(|a| a.parse::<f64>().ok());
    let drawn = across.and_then(|m| bar(m, imperial));
    json!({
        "kind": "object",
        "class": "MapScale",
        "imperial": imperial,
        "available": drawn.is_some(),
        "text": drawn.as_ref().map(|(t, _)| t.clone()).unwrap_or_default(),
        "fraction": drawn.map(|(_, f)| f),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_snaps_to_qgcs_ladder_and_reports_its_share_of_the_width() {
        let (text, fraction) = bar(120.0, false).unwrap();
        assert_eq!(text, "100 m");
        assert!((fraction - 100.0 / 120.0).abs() < 1e-9);
        assert_eq!(bar(1600.0, false).unwrap().0, "2 km");
        assert_eq!(bar(1200.0, false).unwrap().0, "1000 m");
        assert_eq!(bar(150000.0, false).unwrap().0, "200 km");
        assert_eq!(bar(9e9, false).unwrap().0, "2000 km");
    }

    #[test]
    fn imperial_uses_feet_then_miles() {
        assert_eq!(bar(100.0, true).unwrap().0, "250 ft");
        assert_eq!(bar(1700.0, true).unwrap().0, "1 mile");
        assert_eq!(bar(20000.0, true).unwrap().0, "10 miles");
    }

    #[test]
    fn nothing_is_drawn_for_a_bad_width() {
        assert!(bar(0.0, false).is_none());
        assert!(bar(f64::NAN, true).is_none());
    }
}
