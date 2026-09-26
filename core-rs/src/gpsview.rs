use serde_json::{Value, json};

use crate::instruments::display_units;
use crate::read::{flag, object};
use crate::router::Backend;

// Both heads read vehicle.gps as a raw FactGroup and rebuilt the same detail rows from it: which
// facts, in which order, what counts as unreported, and that gps.lock is spelled by its enum
// ("3D Lock") rather than its index. That rule now lives here once.
pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.gps.lat",
    "vehicle.gps.lon",
    "vehicle.gps.mgrs",
    "vehicle.gps.count",
    "vehicle.gps.lock",
    "vehicle.gps.hdop",
    "vehicle.gps.vdop",
    "vehicle.gps.courseOverGround",
];

const DETAIL: &[(&str, &str)] = &[("count", "Satellites"), ("hdop", "HDOP"), ("vdop", "VDOP"), ("courseOverGround", "Course over ground"), ("mgrs", "MGRS")];
// Fact::invalidValueString spells a NaN as en dashes ("–.–", "––:––:––") since the upstream merge,
// where the heads' lists still carried the old "--.--". The bridge sends such a value as null, which
// is what `unset` reads; the strings stay for a fact that does not say.
const UNREPORTED: &[&str] = &["--.--", "--:--:--", "–.–", "––:––:––", "–", "", "0.00 s"];

#[derive(Debug, Default, Clone, PartialEq)]
struct Reading {
    spelled: String,
    units: String,
    number: Option<f64>,
    unset: bool,
}

fn reading(fact: &Value) -> Option<Reading> {
    if fact.get("kind").and_then(Value::as_str) != Some("fact") {
        return None;
    }
    let spelled = fact.get("enumOrValueString").or_else(|| fact.get("valueString")).and_then(Value::as_str)?.to_string();
    Some(Reading {
        spelled,
        units: fact.get("units").and_then(Value::as_str).unwrap_or_default().to_string(),
        number: fact.get("value").and_then(Value::as_f64).filter(|v| v.is_finite()),
        unset: matches!(fact.get("value"), Some(Value::Null)),
    })
}

fn shown(reading: &Reading) -> Option<String> {
    if reading.unset || UNREPORTED.contains(&reading.spelled.as_str()) {
        return None;
    }
    let units = display_units(&reading.units);
    Some(match units.is_empty() {
        true => reading.spelled.clone(),
        false => format!("{} {units}", reading.spelled),
    })
}

// A receiver that has not computed a dilution sends UINT16_MAX, which GPS_RAW_INT's scale makes
// 655.35, and some send 0; neither is a precision anyone can act on. The Android head dropped a DOP
// outside (0, 100) on its own, which the macOS head never did.
fn dop_usable(name: &str, reading: &Reading) -> bool {
    !matches!(name, "hdop" | "vdop") || reading.number.is_some_and(|d| d > 0.0 && d < 100.0)
}

fn rows(fact: &dyn Fn(&str) -> Option<Reading>) -> Vec<Value> {
    let position = match (fact("lat"), fact("lon")) {
        (Some(lat), Some(lon)) if !lat.unset && !lon.unset => vec![json!({ "label": "Position", "value": format!("{}, {}", lat.spelled, lon.spelled) })],
        _ => Vec::new(),
    };
    position
        .into_iter()
        .chain(DETAIL.iter().filter_map(|(name, label)| {
            let reading = fact(name)?;
            dop_usable(name, &reading).then_some(())?;
            Some(json!({ "label": label, "value": shown(&reading)? }))
        }))
        .collect()
}

fn whole(reading: Option<&Reading>) -> Option<i64> {
    reading.and_then(|r| r.number).map(|n| n as i64)
}

pub fn gps_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let available = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let fact = |name: &str| available.then(|| reading(&object(&backend.get(&format!("vehicle.gps.{name}"))))).flatten();
    let lock = fact("lock");
    let count = fact("count");
    json!({
        "kind": "object",
        "class": "GpsStatus",
        "available": available,
        "satellites": whole(count.as_ref()),
        "lock": whole(lock.as_ref()),
        "lockText": lock.map(|l| l.spelled).unwrap_or_default(),
        "rows": rows(&fact),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Gps(Vec<(&'static str, Value)>);

    impl Backend for Gps {
        fn get(&self, p: &str) -> String {
            match p {
                _ => self.0.iter().find(|(n, _)| p == format!("vehicle.gps.{n}")).map(|(_, f)| f.clone()).unwrap_or(json!({ "kind": "null" })),
            }
            .to_string()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "activeVehicleAvailable": !self.0.is_empty() }).to_string() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn fact(name: &str, value: Value, spelled: &str, units: &str) -> Value {
        json!({ "kind": "fact", "name": name, "value": value, "valueString": spelled, "enumOrValueString": spelled, "units": units })
    }

    #[test]
    fn the_gps_rows_are_the_ones_both_heads_drew_and_the_lock_is_spelled_by_its_meaning() {
        let gps = Gps(vec![
            ("lat", fact("lat", json!(47.3977), "47.3977420", "deg")),
            ("lon", fact("lon", json!(8.5456), "8.5456075", "deg")),
            ("count", fact("count", json!(10), "10", "")),
            ("lock", json!({ "kind": "fact", "name": "lock", "value": 3, "valueString": "3", "enumOrValueString": "3D Lock", "units": "" })),
            ("hdop", fact("hdop", json!(0.7), "0.7", "")),
            ("vdop", fact("vdop", json!(1.1), "1.1", "")),
            ("courseOverGround", fact("courseOverGround", Value::Null, "–.–", "deg")),
            ("mgrs", fact("mgrs", json!("32T MN 64 14"), "32T MN 64 14", "")),
        ]);
        let view = gps_view(&gps, &[]);
        assert_eq!((&view["available"], &view["satellites"], &view["lock"]), (&json!(true), &json!(10), &json!(3)));
        assert_eq!(view["lockText"], "3D Lock", "valueString is the enum's index, which an operator reads as a number of something");
        let labels: Vec<&str> = view["rows"].as_array().unwrap().iter().map(|r| r["label"].as_str().unwrap()).collect();
        assert_eq!(labels, ["Position", "Satellites", "HDOP", "VDOP", "MGRS"], "a course the vehicle has not reported is left out rather than drawn as –.– deg, the en-dash spelling the old --.-- list missed");
        assert_eq!(view["rows"][0]["value"], "47.3977420, 8.5456075");
        let unknown_dop = Reading { spelled: "655.35".into(), units: String::new(), number: Some(655.35), unset: false };
        assert!(!dop_usable("hdop", &unknown_dop), "UINT16_MAX scaled by GPS_RAW_INT is the receiver saying it has no dilution");
        assert!(!dop_usable("vdop", &Reading { number: Some(0.0), ..unknown_dop.clone() }));
        assert!(dop_usable("hdop", &Reading { number: Some(1.4), ..unknown_dop.clone() }) && dop_usable("count", &unknown_dop));

        let none = gps_view(&Gps(Vec::new()), &[]);
        assert_eq!((&none["available"], &none["satellites"], &none["rows"]), (&json!(false), &Value::Null, &json!([])));
    }

    #[test]
    fn a_unit_is_spelled_the_way_the_heads_spell_it() {
        let volts = Reading { spelled: "12.1".into(), units: "v".into(), number: Some(12.1), unset: false };
        assert_eq!(shown(&volts).as_deref(), Some("12.1 V"));
        assert_eq!(shown(&Reading { spelled: "0.00 s".into(), ..volts.clone() }), None);
        assert_eq!(shown(&Reading { spelled: "whatever Qt spells".into(), unset: true, ..volts }), None, "a NaN is unreported however it is spelled");
    }
}
