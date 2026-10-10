use serde_json::{Value, json};

use crate::factmeta::{MetaData, ValueType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    Altitude,
    Distance,
    Speed,
}

impl Quantity {
    pub fn named(text: &str) -> Option<Quantity> {
        match text.trim() {
            "altitude" => Some(Quantity::Altitude),
            "distance" => Some(Quantity::Distance),
            "speed" => Some(Quantity::Speed),
            _ => None,
        }
    }

    fn units(self) -> &'static str {
        match self {
            Quantity::Altitude => "vertical m",
            Quantity::Distance => "m",
            Quantity::Speed => "m/s",
        }
    }
}

pub fn raw_per_base(raw_units: &str) -> Option<f64> {
    match raw_units.trim().to_lowercase().as_str() {
        "cm" | "cm/s" => Some(100.0),
        "m" | "meter" | "meters" | "vertical m" | "m/s" => Some(1.0),
        _ => None,
    }
}

fn shown_decimals(cooked: f64) -> i64 {
    i64::from((cooked * 10.0).round() % 10.0 != 0.0)
}

#[derive(Debug, Clone)]
pub struct Pilot {
    meta: MetaData,
    raw_per_base: f64,
    whole_raw: bool,
}

impl Pilot {
    pub fn new(quantity: Quantity, meta: &MetaData) -> Option<Pilot> {
        let raw_per_base = raw_per_base(meta.units.as_deref()?)?;
        let scaled = |bound: &Option<Value>| bound.as_ref().and_then(Value::as_f64).map(|v| json!(v / raw_per_base));
        Some(Pilot {
            meta: MetaData {
                value_type: ValueType::Double,
                units: Some(quantity.units().to_string()),
                min: scaled(&meta.min),
                max: scaled(&meta.max),
                default: scaled(&meta.default),
                increment: meta.increment.map(|v| v / raw_per_base),
                user_min: meta.user_min.map(|v| v / raw_per_base),
                user_max: meta.user_max.map(|v| v / raw_per_base),
                ..meta.clone()
            },
            raw_per_base,
            whole_raw: !matches!(meta.value_type, ValueType::Float | ValueType::Double),
        })
    }

    pub fn fact(&self, raw: f64) -> Value {
        let base = raw / self.raw_per_base;
        let cooked = crate::units::for_fact(&self.meta, crate::units::cooking).map_or(base, |unit| (unit.shown)(base));
        let meta = MetaData { decimal_places: Some(shown_decimals(cooked)), ..self.meta.clone() };
        crate::vehiclefact::fact(&meta, &json!(base), None)
    }

    pub fn raw(&self, written: f64, cooked: bool) -> f64 {
        let base = match cooked {
            true => crate::units::for_fact(&self.meta, crate::units::cooking).map_or(written, |unit| (unit.base)(written)),
            false => written,
        };
        let raw = base * self.raw_per_base;
        if self.whole_raw { raw.round() } else { raw }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(value_type: ValueType, units: &str, min: f64, max: f64) -> MetaData {
        MetaData { units: Some(units.to_string()), min: Some(json!(min)), max: Some(json!(max)), ..crate::px4meta::bare(value_type) }
    }

    #[test]
    fn arducopter_centimetres_read_and_write_as_metres() {
        let rtl = Pilot::new(Quantity::Altitude, &meta(ValueType::Int32, "cm", 30.0, 300_000.0)).unwrap();
        let shown = rtl.fact(1500.0);
        assert_eq!((shown["valueString"].as_str(), shown["units"].as_str()), (Some("15"), Some("m")), "RTL_ALT 1500 cm is 15 m, not 1500 cm");
        assert_eq!((shown["min"].as_f64(), shown["max"].as_f64()), (Some(0.3), Some(3000.0)));
        assert_eq!(rtl.fact(1550.0)["valueString"].as_str(), Some("15.5"), "half a metre still shows");
        assert_eq!((rtl.raw(20.0, true), rtl.raw(15.55, true)), (2000.0, 1555.0), "a write in metres lands in whole centimetres");
    }

    #[test]
    fn speeds_in_centimetres_per_second_show_in_metres_per_second() {
        let loiter = Pilot::new(Quantity::Speed, &meta(ValueType::Float, "cm/s", 20.0, 3500.0)).unwrap();
        assert_eq!((loiter.fact(1250.0)["valueString"].as_str(), loiter.fact(1250.0)["units"].as_str()), (Some("12.5"), Some("m/s")));
        assert_eq!(loiter.raw(10.0, true), 1000.0);
    }

    #[test]
    fn metres_lose_the_raw_decimals_and_unknown_units_stay_raw() {
        let radius = Pilot::new(Quantity::Distance, &meta(ValueType::Float, "m", 30.0, 10_000.0)).unwrap();
        assert_eq!(radius.fact(150.0)["valueString"].as_str(), Some("150"), "FENCE_RADIUS showed 150.000 m");
        assert!(Pilot::new(Quantity::Altitude, &meta(ValueType::Float, "deg", 0.0, 90.0)).is_none(), "a unit this does not know is left to the raw fact");
        assert_eq!(Quantity::named("speed"), Some(Quantity::Speed));
        assert_eq!(Quantity::named("volts"), None);
    }
}
