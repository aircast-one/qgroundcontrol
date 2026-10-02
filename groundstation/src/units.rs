use std::sync::atomic::{AtomicU8, Ordering};

use serde_json::{Value, json};

pub const METRIC: u8 = 0;
pub const IMPERIAL_US: u8 = 1;

static MEASUREMENT_SYSTEM: AtomicU8 = AtomicU8::new(METRIC);

const FEET_TO_METERS: f64 = 0.3048;
const MILES_TO_METERS: f64 = 1609.344;
const HOUR_SECONDS: f64 = 3600.0;
const KNOTS_TO_KPH: f64 = 1.852;
const OUNCES_TO_GRAMS: f64 = 28.3495;
const POUNDS_TO_GRAMS: f64 = 453.592;

pub fn set_measurement_system(system: u8) {
    MEASUREMENT_SYSTEM.store(system, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Horizontal,
    Vertical,
    Area,
    Speed,
    Temperature,
    Weight,
}

impl Kind {
    fn setting(self) -> &'static str {
        match self {
            Kind::Horizontal => "horizontalDistanceUnits",
            Kind::Vertical => "verticalDistanceUnits",
            Kind::Area => "areaUnits",
            Kind::Speed => "speedUnits",
            Kind::Temperature => "temperatureUnits",
            Kind::Weight => "weightUnits",
        }
    }

    fn base(self) -> &'static str {
        match self {
            Kind::Horizontal | Kind::Vertical => "m",
            Kind::Area => "m^2",
            Kind::Speed => "m/s",
            Kind::Temperature => "C",
            Kind::Weight => "g",
        }
    }

    fn default_choice(self, system: u8) -> u32 {
        let imperial = system != METRIC;
        match self {
            Kind::Horizontal | Kind::Vertical => u32::from(!imperial),
            Kind::Area => if imperial { 5 } else { 1 },
            Kind::Speed => if imperial { 2 } else { 1 },
            Kind::Temperature => u32::from(imperial),
            Kind::Weight => if system == IMPERIAL_US { 2 } else { 0 },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Conversion {
    pub name: &'static str,
    pub shown: fn(f64) -> f64,
    pub base: fn(f64) -> f64,
}

fn conversion(kind: Kind, choice: u32) -> Option<Conversion> {
    let (name, shown, base): (&'static str, fn(f64) -> f64, fn(f64) -> f64) = match (kind, choice) {
        (Kind::Horizontal | Kind::Vertical, 0) | (Kind::Speed, 0) => (if kind == Kind::Speed { "ft/s" } else { "ft" }, |v| v / FEET_TO_METERS, |v| v * FEET_TO_METERS),
        (Kind::Horizontal | Kind::Vertical, 1) => ("m", |v| v, |v| v),
        (Kind::Area, 0) => ("ft^2", |v| v * 10.7639, |v| v * 0.0929),
        (Kind::Area, 1) => ("m^2", |v| v, |v| v),
        (Kind::Area, 2) => ("km^2", |v| v * 0.000_001, |v| v * 1_000_000.0),
        (Kind::Area, 3) => ("ha", |v| v * 0.0001, |v| v * 10_000.0),
        (Kind::Area, 4) => ("ac", |v| v * 0.000_247_105, |v| v * 4046.86),
        (Kind::Area, 5) => ("mi^2", |v| v * 3.86102e-7, |v| v * 2_589_988.11),
        (Kind::Speed, 1) => ("m/s", |v| v, |v| v),
        (Kind::Speed, 2) => ("mph", |v| (v / MILES_TO_METERS) * HOUR_SECONDS, |v| (v * MILES_TO_METERS) / HOUR_SECONDS),
        (Kind::Speed, 3) => ("km/h", |v| (v / 1000.0) * HOUR_SECONDS, |v| (v * 1000.0) / HOUR_SECONDS),
        (Kind::Speed, 4) => ("kn", |v| (v * HOUR_SECONDS) / (1000.0 * KNOTS_TO_KPH), |v| v * (1000.0 * KNOTS_TO_KPH / HOUR_SECONDS)),
        (Kind::Temperature, 0) => ("C", |v| v, |v| v),
        (Kind::Temperature, 1) => ("F", |v| (v * (9.0 / 5.0)) + 32.0, |v| (v - 32.0) * (5.0 / 9.0)),
        (Kind::Weight, 0) => ("g", |v| v, |v| v),
        (Kind::Weight, 1) => ("kg", |v| v / 1000.0, |v| v * 1000.0),
        (Kind::Weight, 2) => ("oz", |v| v / OUNCES_TO_GRAMS, |v| v * OUNCES_TO_GRAMS),
        (Kind::Weight, 3) => ("lbs", |v| v / POUNDS_TO_GRAMS, |v| v * POUNDS_TO_GRAMS),
        _ => return None,
    };
    Some(Conversion { name, shown, base })
}

fn choice(kind: Kind, stored: &impl Fn(&str) -> Option<String>, system: u8) -> u32 {
    stored(&format!("Units/{}", kind.setting())).and_then(|text| text.trim().parse().ok()).unwrap_or_else(|| kind.default_choice(system))
}

fn chosen(kind: Kind) -> Option<Conversion> {
    conversion(kind, choice(kind, &crate::settingsstore::stored_text, MEASUREMENT_SYSTEM.load(Ordering::Relaxed)))
}

const PER_PIXEL_CENTIMETRES: Conversion = Conversion { name: "cm/px", shown: |v| v, base: |v| v };
const PER_PIXEL_INCHES: Conversion = Conversion { name: "in/px", shown: |v| v / 2.54, base: |v| v * 2.54 };

pub fn cooking_with(raw_units: &str, stored: impl Fn(&str) -> Option<String>, system: u8) -> Option<Conversion> {
    let pick = |kind: Kind| choice(kind, &stored, system);
    match raw_units.to_lowercase().as_str() {
        "m" | "meter" | "meters" => conversion(Kind::Horizontal, pick(Kind::Horizontal)),
        "vertical m" => conversion(Kind::Vertical, pick(Kind::Vertical)),
        "cm/px" => match pick(Kind::Horizontal) {
            0 => Some(PER_PIXEL_INCHES),
            1 => Some(PER_PIXEL_CENTIMETRES),
            _ => None,
        },
        "m/s" => conversion(Kind::Speed, pick(Kind::Speed)),
        "c" => conversion(Kind::Temperature, pick(Kind::Temperature)),
        "m^2" => conversion(Kind::Area, pick(Kind::Area)),
        "g" => conversion(Kind::Weight, pick(Kind::Weight)),
        _ => None,
    }
}

pub fn cooking(raw_units: &str) -> Option<Conversion> {
    cooking_with(raw_units, crate::settingsstore::stored_text, MEASUREMENT_SYSTEM.load(Ordering::Relaxed))
}

const BUILT_IN: [(&str, Conversion); 6] = [
    ("centi-degrees", Conversion { name: "deg", shown: |v| v / 100.0, base: |v| v * 100.0 }),
    ("radians", Conversion { name: "deg", shown: f64::to_degrees, base: f64::to_radians }),
    ("rad", Conversion { name: "deg", shown: f64::to_degrees, base: f64::to_radians }),
    ("gimbal-degrees", Conversion { name: "deg", shown: |v| v * -1.0, base: |v| v * -1.0 }),
    ("norm", Conversion { name: "%", shown: |v| v * 100.0, base: |v| v / 100.0 }),
    ("centi-celsius", Conversion { name: "C", shown: |v| v / 100.0, base: |v| v * 100.0 }),
];

pub fn built_in(raw_units: &str) -> Option<Conversion> {
    let lowered = raw_units.to_lowercase();
    BUILT_IN.iter().find(|(units, _)| *units == lowered).map(|(_, conversion)| *conversion)
}

pub fn for_fact(meta: &crate::factmeta::MetaData, preference: impl Fn(&str) -> Option<Conversion>) -> Option<Conversion> {
    let raw_units = meta.units.as_deref().unwrap_or("");
    let lowered = raw_units.to_lowercase();
    if !meta.enums.is_empty() || !meta.bits.is_empty() {
        return None;
    }
    let real = matches!(meta.value_type, crate::factmeta::ValueType::Float | crate::factmeta::ValueType::Double);
    BUILT_IN
        .iter()
        .find(|(units, _)| *units == lowered)
        .map(|(_, conversion)| *conversion)
        .or_else(|| real.then(|| preference(raw_units)).flatten())
        .or_else(|| (lowered == "vertical m").then_some(Conversion { name: "m", shown: |v| v, base: |v| v }))
}

pub fn metric(raw_units: &str) -> Option<Conversion> {
    cooking_with(raw_units, |_| None, METRIC)
}

fn name_of(property: &str) -> Option<Kind> {
    Some(match property {
        "appSettingsHorizontalDistanceUnitsString" => Kind::Horizontal,
        "appSettingsVerticalDistanceUnitsString" => Kind::Vertical,
        "appSettingsAreaUnitsString" => Kind::Area,
        "appSettingsSpeedUnitsString" => Kind::Speed,
        "appSettingsWeightUnitsString" => Kind::Weight,
        _ => return None,
    })
}

fn conversion_of(method: &str) -> Option<(Kind, bool)> {
    Some(match method {
        "metersToAppSettingsHorizontalDistanceUnits" => (Kind::Horizontal, true),
        "appSettingsHorizontalDistanceUnitsToMeters" => (Kind::Horizontal, false),
        "metersToAppSettingsVerticalDistanceUnits" => (Kind::Vertical, true),
        "appSettingsVerticalDistanceUnitsToMeters" => (Kind::Vertical, false),
        "squareMetersToAppSettingsAreaUnits" => (Kind::Area, true),
        "appSettingsAreaUnitsToSquareMeters" => (Kind::Area, false),
        "metersSecondToAppSettingsSpeedUnits" => (Kind::Speed, true),
        "appSettingsSpeedUnitsToMetersSecond" => (Kind::Speed, false),
        "gramsToAppSettingsWeightUnits" => (Kind::Weight, true),
        "appSettingsWeightUnitsToGrams" => (Kind::Weight, false),
        _ => return None,
    })
}

pub fn fields(path: &str, fields: &str) -> Option<String> {
    (path == "units").then_some(())?;
    let answered: Option<serde_json::Map<String, Value>> = fields
        .split(',')
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(|field| name_of(field).map(|kind| (field.to_string(), json!(chosen(kind).map_or(kind.base(), |c| c.name)))))
        .collect();
    let mut object = Value::Object(answered.filter(|a| !a.is_empty())?);
    object["kind"] = json!("object");
    Some(object.to_string())
}

const UNIT_KINDS: [Kind; 6] = [Kind::Horizontal, Kind::Vertical, Kind::Area, Kind::Speed, Kind::Temperature, Kind::Weight];

fn described_as(kind: Kind) -> (&'static [&'static str], &'static str) {
    match kind {
        Kind::Horizontal => (&["Feet", "Meters"], "Display unit for horizontal distances and ranges."),
        Kind::Vertical => (&["Feet", "Meters"], "Display unit for altitudes and vertical heights."),
        Kind::Area => (&["Square Feet", "Square Meters", "Square Kilometers", "Hectares", "Acres", "Square Miles"], "Display unit for area measurements."),
        Kind::Speed => (&["Feet per Second", "Meters per Second", "Miles per Hour", "Kilometers per Hour", "Knots"], "Display unit for speed and velocity values."),
        Kind::Temperature => (&["Celsius", "Fahrenheit"], "Display unit for temperature readings."),
        Kind::Weight => (&["Grams", "Kilograms", "Ounces", "Pounds"], "Weight"),
    }
}

pub fn fact_metadata(setting: &str) -> Option<crate::factmeta::MetaData> {
    let kind = kind_of_setting(setting)?;
    let (labels, short) = described_as(kind);
    let enums = labels.iter().enumerate().map(|(value, label)| crate::factmeta::EnumEntry { label: (*label).to_string(), value: json!(value) }).collect();
    Some(crate::factmeta::MetaData {
        name: setting.to_string(),
        short_description: short.to_string(),
        enums,
        default: Some(json!(kind.default_choice(MEASUREMENT_SYSTEM.load(Ordering::Relaxed)))),
        qgc_reboot_required: true,
        ..crate::px4meta::bare(crate::factmeta::ValueType::Uint32)
    })
}

fn kind_of_setting(setting: &str) -> Option<Kind> {
    UNIT_KINDS.into_iter().find(|kind| kind.setting() == setting)
}

const UNIT_SYSTEM_PRESETS: [[u32; 5]; 2] = [[1, 1, 1, 1, 0], [0, 0, 5, 2, 1]];
const UNIT_SYSTEM_CUSTOM: usize = 2;

fn unit_system(stored: &impl Fn(&str) -> Option<String>, system: u8) -> usize {
    if stored("Units/customUnits").is_some_and(|v| v == "true" || v == "1") {
        return UNIT_SYSTEM_CUSTOM;
    }
    let current = [Kind::Horizontal, Kind::Vertical, Kind::Area, Kind::Speed, Kind::Temperature].map(|kind| choice(kind, stored, system));
    UNIT_SYSTEM_PRESETS.iter().position(|preset| *preset == current).unwrap_or(UNIT_SYSTEM_CUSTOM)
}

pub fn get(path: &str) -> Option<String> {
    if path == "settings.unitsSettings.unitSystem" {
        return Some(json!({ "kind": "value", "value": unit_system(&crate::settingsstore::stored_text, MEASUREMENT_SYSTEM.load(Ordering::Relaxed)) }).to_string());
    }
    if let Some(setting) = path.strip_prefix("settings.unitsSettings.").and_then(|rest| rest.strip_suffix(".rawValue")) {
        let kind = kind_of_setting(setting)?;
        return Some(json!({ "kind": "value", "value": choice(kind, &crate::settingsstore::stored_text, MEASUREMENT_SYSTEM.load(Ordering::Relaxed)) }).to_string());
    }
    let kind = name_of(path.strip_prefix("units.")?)?;
    Some(json!({ "kind": "value", "value": chosen(kind).map_or(kind.base(), |c| c.name) }).to_string())
}

pub fn invoke(path: &str, args: &str) -> Option<String> {
    let (kind, to_display) = conversion_of(path.strip_prefix("units.")?)?;
    let given = serde_json::from_str::<Value>(args).ok()?.get(0)?.as_f64()?;
    let result = chosen(kind).map_or(given, |c| if to_display { (c.shown)(given) } else { (c.base)(given) });
    Some(json!({ "ok": true, "result": result }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_setting_follows_the_locale_and_a_stored_one_wins() {
        let nothing = |_: &str| None;
        assert_eq!((choice(Kind::Horizontal, &nothing, METRIC), choice(Kind::Horizontal, &nothing, IMPERIAL_US)), (1, 0));
        assert_eq!((choice(Kind::Weight, &nothing, 2), choice(Kind::Weight, &nothing, IMPERIAL_US)), (0, 2), "the UK keeps grams");
        let feet = |key: &str| (key == "Units/verticalDistanceUnits").then(|| "0".to_string());
        assert_eq!(choice(Kind::Vertical, &feet, METRIC), 0);
    }

    #[test]
    fn the_unit_system_is_the_preset_every_unit_matches() {
        let stored = |pairs: &'static [(&'static str, &'static str)]| move |key: &str| pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string());
        assert_eq!(unit_system(&stored(&[]), METRIC), 0);
        assert_eq!(unit_system(&stored(&[("Units/horizontalDistanceUnits", "0"), ("Units/verticalDistanceUnits", "0"), ("Units/areaUnits", "5"), ("Units/speedUnits", "2"), ("Units/temperatureUnits", "1")]), METRIC), 1);
        assert_eq!(unit_system(&stored(&[("Units/speedUnits", "4")]), METRIC), 2, "knots with metric distances is no preset");
        assert_eq!(unit_system(&stored(&[("Units/customUnits", "true")]), METRIC), 2);
    }

    #[test]
    fn conversions_follow_qgcs_table() {
        let feet = conversion(Kind::Horizontal, 0).unwrap();
        assert_eq!(((feet.shown)(100.0), feet.name), (100.0 / 0.3048, "ft"));
        assert_eq!(conversion(Kind::Speed, 0).unwrap().name, "ft/s");
        assert_eq!((conversion(Kind::Speed, 4).unwrap().shown)(10.0), 36000.0 / 1852.0);
        assert_eq!((conversion(Kind::Area, 0).unwrap().base)(1.0), 0.0929, "QGC converts square feet back with its own rounded constant");
        assert_eq!((conversion(Kind::Area, 3).unwrap().base)(1.0), 10_000.0);
        assert!(conversion(Kind::Area, 9).is_none(), "an unknown choice leaves the value in base units");
        let imperial = |_: &str| None;
        assert_eq!(cooking_with("C", imperial, IMPERIAL_US).map(|c| ((c.shown)(100.0), c.name)), Some((212.0, "F")));
        assert_eq!(cooking_with("cm/px", imperial, IMPERIAL_US).map(|c| c.name), Some("in/px"));
        assert!(metric("deg/s").is_none(), "a unit outside the table is shown as it is");
    }
}
