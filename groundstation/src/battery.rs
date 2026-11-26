use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::instruments::display_units;
use crate::read::value_number;
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.batteries.count",
    "settings.batteryIndicatorSettings.threshold1",
    "settings.batteryIndicatorSettings.threshold2",
    "settings.batteryIndicatorSettings.valueDisplay",
    "vehicle.flying",
    "vehicle.distanceToHome",
    "vehicle.altitudeRelative",
];

const MAX_PACKS: usize = 8;
const PACK_FACTS: [&str; 10] = [
    "voltage",
    "current",
    "percentRemaining",
    "chargeState",
    "timeRemaining",
    "timeRemainingStr",
    "instantPower",
    "mahConsumed",
    "temperature",
    "function",
];
const DETAIL_FACTS: [&str; 7] = [
    "function",
    "voltage",
    "current",
    "instantPower",
    "mahConsumed",
    "timeRemainingStr",
    "temperature",
];
const FUNCTION_UNKNOWN: f64 = 0.0;
const FUNCTION_ALL: f64 = 1.0;
static PACKS_SEEN: AtomicUsize = AtomicUsize::new(0);

fn pack_fact_path(index: usize, name: &str) -> String {
    format!("vehicle.batteries.{index}.{name}")
}

fn pack_paths(facts: &'static [&'static str]) -> Vec<String> {
    (0..PACKS_SEEN.load(Ordering::Relaxed).clamp(1, MAX_PACKS))
        .flat_map(|i| facts.iter().map(move |name| pack_fact_path(i, name)))
        .collect()
}

pub fn deps() -> Vec<String> {
    DEPS.iter()
        .map(|d| d.to_string())
        .chain(pack_paths(&PACK_FACTS))
        .collect()
}

const CHARGE_DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.batteries.count",
    "settings.batteryIndicatorSettings.threshold1",
    "settings.batteryIndicatorSettings.threshold2",
];
const CHARGE_FACTS: [&str; 2] = ["percentRemaining", "chargeState"];

pub fn charge_deps() -> Vec<String> {
    CHARGE_DEPS
        .iter()
        .map(|d| d.to_string())
        .chain(pack_paths(&CHARGE_FACTS))
        .collect()
}

const CHARGE_UNDEFINED: i64 = 0;
const CHARGE_OK: i64 = 1;
const CHARGE_LOW: i64 = 2;
const PERCENT_ROUNDS_TO_FULL: f64 = 98.9;

#[derive(Debug, PartialEq)]
pub struct Pack {
    pub voltage: Option<f64>,
    pub current: Option<f64>,
    pub percent: Option<f64>,
    pub charge_state: i64,
    pub charge_label: String,
    pub percent_text: String,
    pub voltage_text: String,
    pub current_text: String,
    pub time_remaining_text: Option<String>,
}

const BATTERY_LEVELS: [&str; 4] = ["critical", "warning", "caution", "normal"];

fn rank(level: &str) -> usize {
    BATTERY_LEVELS
        .iter()
        .position(|known| *known == level)
        .unwrap_or(BATTERY_LEVELS.len() - 1)
}

pub fn level(
    charge_state: i64,
    percent: Option<f64>,
    threshold1: f64,
    threshold2: f64,
) -> &'static str {
    let by_charge = match charge_state {
        CHARGE_LOW => "warning",
        3..=6 => "critical",
        _ => "normal",
    };
    let by_percent = match percent {
        Some(p) if p <= threshold2 => "warning",
        Some(p) if p <= threshold1 => "caution",
        _ => "normal",
    };
    [by_charge, by_percent]
        .into_iter()
        .min_by_key(|shown| rank(shown))
        .unwrap_or(by_charge)
}

pub fn text(pack: &Pack) -> String {
    match (pack.percent, pack.voltage, pack.charge_state) {
        (Some(p), _, _) if p > PERCENT_ROUNDS_TO_FULL => "100%".to_string(),
        (Some(_), _, _) => pack.percent_text.clone(),
        (None, Some(_), _) => pack.voltage_text.clone(),
        (None, None, state) if severity(state) > 0 => pack.charge_label.clone(),
        _ => "n/a".to_string(),
    }
}

pub fn secondary_text(pack: &Pack) -> String {
    match (&pack.time_remaining_text, pack.voltage, pack.charge_state) {
        (Some(t), _, _) => t.clone(),
        (None, Some(_), _) => pack.voltage_text.clone(),
        (None, None, state) if severity(state) > 0 => pack.charge_label.clone(),
        _ => "n/a".to_string(),
    }
}

const SHOW_VOLTAGE: i64 = 1;
const SHOW_BOTH: i64 = 2;

pub fn indicator_lines(pack: &Pack, value_display: i64) -> Vec<String> {
    match value_display {
        SHOW_VOLTAGE => vec![secondary_text(pack)],
        SHOW_BOTH => vec![text(pack), secondary_text(pack)],
        _ => vec![text(pack)],
    }
}

fn pack_count(backend: &dyn Backend) -> usize {
    let count = value_number(&backend.value("vehicle.batteries.count"))
        .map(|n| n as usize)
        .unwrap_or(0)
        .min(MAX_PACKS);
    PACKS_SEEN.fetch_max(count, Ordering::Relaxed);
    count
}

fn remembered(backend: &dyn Backend, index: usize) -> impl Fn(&str) -> Value + '_ {
    let read: std::cell::RefCell<std::collections::HashMap<String, Value>> =
        std::cell::RefCell::default();
    move |name: &str| {
        read.borrow_mut()
            .entry(name.to_string())
            .or_insert_with(|| backend.value(&pack_fact_path(index, name)))
            .clone()
    }
}

fn pack(fact: &dyn Fn(&str) -> Value) -> Pack {
    let text = |name: &str, key: &str| {
        fact(name)
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_default()
    };
    let number = |name: &str| {
        fact(name)
            .get("value")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
    let shown = |name: &str| {
        format!(
            "{}{}",
            text(name, "valueString"),
            display_units(&text(name, "units"))
        )
    };
    Pack {
        voltage: number("voltage"),
        current: number("current"),
        percent: number("percentRemaining"),
        charge_state: fact("chargeState")
            .get("value")
            .and_then(Value::as_i64)
            .unwrap_or(CHARGE_UNDEFINED),
        charge_label: text("chargeState", "enumOrValueString"),
        percent_text: shown("percentRemaining"),
        voltage_text: shown("voltage"),
        current_text: shown("current"),
        time_remaining_text: number("timeRemaining")
            .map(|_| text("timeRemainingStr", "valueString"))
            .filter(|t| !t.is_empty()),
    }
}

fn detail_facts(read: &dyn Fn(&str) -> Value) -> Value {
    DETAIL_FACTS
        .iter()
        .filter_map(|name| {
            let fact = read(name);
            let raw = fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64);
            if *name == "function" && raw.is_none_or(|f| f == FUNCTION_UNKNOWN || f == FUNCTION_ALL) {
                return None;
            }
            let key = if *name == "function" { "enumOrValueString" } else { "valueString" };
            let spelled = fact.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())?;
            Some(json!({ "name": name, "valueString": spelled, "value": fact.get("value").cloned().unwrap_or(Value::Null), "units": fact.get("units").and_then(Value::as_str).unwrap_or_default() }))
        })
        .collect()
}

pub fn severity(charge_state: i64) -> i64 {
    match charge_state {
        4..=6 => 3,
        3 => 2,
        CHARGE_LOW => 1,
        _ => 0,
    }
}

pub fn duration_text(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return String::new();
    }
    let total = seconds.round() as i64;
    let (hours, minutes, secs) = (total / 3600, (total % 3600) / 60, total % 60);
    match (total < 60, hours > 0) {
        (true, _) => format!("{total} sec"),
        (false, true) => format!("{hours}:{minutes:02}:{secs:02}"),
        (false, false) => format!("{minutes}:{secs:02}"),
    }
}

pub struct PopupPack {
    pub charge_state: i64,
    pub charge_label: String,
    pub percent: Option<f64>,
    pub time_remaining: Option<f64>,
    pub voltage: Option<String>,
    pub consumed_mah: Option<f64>,
    pub capacity_mah: Option<f64>,
    pub temperature: Option<String>,
    pub function: Option<String>,
    pub voltage_problem: Option<String>,
}

const FULLEST_CELL_VOLTS: f64 = 4.35;
const EMPTIEST_CELL_VOLTS: f64 = 2.5;
const MOST_CELLS: f64 = 30.0;
const CALIBRATE_VOLTAGE: &str =
    "Calibrate the voltage sensor in Power setup; until then the voltage failsafes can't trigger.";

pub fn voltage_problem(voltage: Option<f64>, set_for: Option<f64>) -> Option<String> {
    let ceiling = set_for.map_or(MOST_CELLS * FULLEST_CELL_VOLTS, |volts| {
        volts / EMPTIEST_CELL_VOLTS * FULLEST_CELL_VOLTS
    });
    voltage.filter(|v| *v > ceiling).map(|_| match set_for {
        Some(volts) => {
            format!("No pack set up for {volts:.1} V reads that high. {CALIBRATE_VOLTAGE}")
        }
        None => format!("More than any drone pack holds. {CALIBRATE_VOLTAGE}"),
    })
}

fn set_for_volts(backend: &dyn Backend, index: usize, stages: &[FailsafeStage]) -> Option<f64> {
    let arm = parameter_value(backend, &format!("{}_ARM_VOLT", apm_prefix(index)), "value");
    stages
        .iter()
        .filter(|stage| stage.action != APM_WARN_ONLY)
        .filter_map(|stage| stage.volts)
        .chain(arm)
        .filter(|volts| *volts > 0.0)
        .reduce(f64::max)
}

pub fn unreal_voltage(backend: &dyn Backend) -> Option<String> {
    (0..pack_count(backend)).find_map(|index| {
        let voltage = value_number(&backend.value(&pack_fact_path(index, "voltage")));
        let set_for = set_for_volts(backend, index, &apm_stages(backend, index));
        voltage_problem(voltage, set_for).and(voltage).map(|volts| {
            format!("Battery reads {volts:.1} V, which can't be real. Calibrate its voltage sensor in Power setup.")
        })
    })
}

fn worse(shown: &'static str, floor: &'static str) -> &'static str {
    match rank(floor) < rank(shown) {
        true => floor,
        false => shown,
    }
}

const CHARGE_UNKNOWN: &str = "Charge unknown";
const UNREPORTED_TEXT: &str = "Not reported";
const CHARGE_HINT: &str = "Set the pack capacity and current sensor in Power setup";
const OVER_CAPACITY_HINT: &str =
    "More than the pack holds. Check the current sensor calibration in Power setup";
const SECONDARY: i64 = -1;
const CAUTION: i64 = 1;

fn limiting_percent(pack: &PopupPack) -> f64 {
    pack.percent
        .filter(|p| p.is_finite())
        .unwrap_or(f64::INFINITY)
}

pub fn headline(packs: &[PopupPack]) -> Option<Value> {
    let (index, worst) = packs.iter().enumerate().min_by(|(_, a), (_, b)| {
        severity(b.charge_state)
            .cmp(&severity(a.charge_state))
            .then(limiting_percent(a).total_cmp(&limiting_percent(b)))
    })?;
    let alarming = severity(worst.charge_state) > 0;
    let percent = worst.percent.map(|p| format!("{}%", p.round() as i64));
    let left = worst
        .time_remaining
        .map(duration_text)
        .filter(|t| !t.is_empty())
        .map(|t| format!("{t} left"));
    let lowest = (packs.len() > 1).then(|| format!("lowest of {}", packs.len()));
    let detail = left
        .into_iter()
        .chain(percent.clone().filter(|_| alarming))
        .chain(lowest)
        .collect::<Vec<_>>()
        .join("  \u{00b7}  ");
    let (text, shown_severity) = match (alarming, percent, &worst.voltage) {
        (true, _, _) => (worst.charge_label.clone(), severity(worst.charge_state)),
        (false, Some(p), _) => (p, 0),
        (false, None, Some(volts)) => (format!("{volts} V"), 0),
        (false, None, None) => (CHARGE_UNKNOWN.to_string(), CAUTION),
    };
    Some(json!({
        "text": text,
        "detail": detail,
        "severity": shown_severity,
        "index": index,
    }))
}

const PX4_LOW_BATTERY_WARN: i64 = 0;
const PX4_LOW_BATTERY_RETURN: i64 = 1;
const PX4_LOW_BATTERY_LAND: i64 = 2;
const PX4_LOW_BATTERY_RETURN_THEN_LAND: i64 = 3;

fn minutes_to_go(seconds: f64) -> String {
    if seconds < 60.0 {
        "under a minute to go".to_string()
    } else {
        format!("about {} min to go", (seconds / 60.0).round() as i64)
    }
}

pub fn margin_text(
    action: Option<i64>,
    low: Option<f64>,
    critical: Option<f64>,
    percent: Option<f64>,
    seconds_left: Option<f64>,
) -> Option<String> {
    let (verb, at) = match action? {
        PX4_LOW_BATTERY_WARN => ("Warns", low?),
        PX4_LOW_BATTERY_RETURN | PX4_LOW_BATTERY_RETURN_THEN_LAND => ("Returns home", critical?),
        PX4_LOW_BATTERY_LAND => ("Lands", critical?),
        _ => return None,
    };
    let rule = format!("{verb} at {}%", at.round() as i64);
    let to_go = percent
        .filter(|p| *p > 0.0)
        .map(|p| match seconds_left.filter(|s| *s > 0.0) {
            _ if p <= at => "reached".to_string(),
            Some(seconds) => minutes_to_go(seconds * (p - at) / p),
            None => format!("{} points to go", (p - at).round() as i64),
        });
    Some(to_go.map_or(rule.clone(), |go| format!("{rule}  \u{00b7}  {go}")))
}

#[derive(Debug, Clone, PartialEq)]
pub struct FailsafeStage {
    pub action: i64,
    pub label: String,
    pub volts: Option<f64>,
    pub mah: Option<f64>,
}

const APM_WARN_ONLY: i64 = 0;
const APM_STAGES: [(&str, &str, &str); 2] = [
    ("FS_LOW_ACT", "LOW_VOLT", "LOW_MAH"),
    ("FS_CRT_ACT", "CRT_VOLT", "CRT_MAH"),
];

fn apm_prefix(index: usize) -> String {
    match index {
        0 => "BATT".to_string(),
        _ => format!("BATT{}", index + 1),
    }
}

fn apm_stages(backend: &dyn Backend, index: usize) -> Vec<FailsafeStage> {
    let prefix = apm_prefix(index);
    let trigger = |name: &str| {
        parameter_value(backend, &format!("{prefix}_{name}"), "value").filter(|v| *v > 0.0)
    };
    APM_STAGES
        .iter()
        .filter_map(|(act, volts, mah)| {
            let action = parameter(backend, &format!("{prefix}_{act}"))?;
            Some(FailsafeStage {
                action: action.get("rawValue").and_then(Value::as_f64)? as i64,
                label: crate::read::enum_label(&action),
                volts: trigger(volts),
                mah: trigger(mah),
            })
        })
        .collect()
}

fn mah_text(mah: f64) -> String {
    crate::offlinemaps::grouped(mah.max(0.0).round() as u64)
}

pub fn apm_margin_text(
    stages: &[FailsafeStage],
    volts: Option<f64>,
    mah_left: Option<f64>,
) -> Option<String> {
    let set = |stage: &&FailsafeStage| stage.volts.is_some() || stage.mah.is_some();
    let stage = stages
        .iter()
        .filter(set)
        .find(|stage| stage.action != APM_WARN_ONLY)
        .or_else(|| stages.iter().find(set))?;
    let verb = match (stage.action, stage.label.trim()) {
        (APM_WARN_ONLY, _) => "Warns",
        (_, "") => "Failsafe",
        (_, label) => label,
    };
    let rule = [
        stage.volts.map(|v| format!("{v:.1} V")),
        stage.mah.map(|m| format!("{} mAh left", mah_text(m))),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" or ");
    let by_mah = stage.mah.zip(mah_left).map(|(at, left)| left - at);
    let by_volts = stage.volts.zip(volts).map(|(at, now)| now - at);
    let to_go = match [by_mah, by_volts]
        .into_iter()
        .flatten()
        .any(|gap| gap <= 0.0)
    {
        true => Some("reached".to_string()),
        false => by_mah
            .map(|gap| format!("{} mAh to go", mah_text(gap)))
            .or_else(|| by_volts.map(|gap| format!("{gap:.1} V to go"))),
    };
    let rule = format!("{verb} at {rule}");
    Some(to_go.map_or(rule.clone(), |go| format!("{rule}  \u{00b7}  {go}")))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReturnTrip {
    pub distance_m: f64,
    pub altitude_m: f64,
    pub return_altitude_m: f64,
    pub cruise_mps: f64,
    pub climb_mps: f64,
    pub descent_mps: f64,
}

impl ReturnTrip {
    pub fn flown_at_m(&self) -> f64 {
        self.altitude_m.max(self.return_altitude_m.max(0.0))
    }
}

pub fn return_seconds(trip: ReturnTrip) -> Option<f64> {
    let speeds_known = [trip.cruise_mps, trip.climb_mps, trip.descent_mps]
        .iter()
        .all(|s| s.is_finite() && *s > 0.0);
    let place_known = [trip.distance_m, trip.altitude_m]
        .iter()
        .all(|v| v.is_finite());
    let flown_at = trip.flown_at_m();
    (speeds_known && place_known).then(|| {
        (flown_at - trip.altitude_m) / trip.climb_mps
            + trip.distance_m / trip.cruise_mps
            + flown_at.max(0.0) / trip.descent_mps
    })
}

pub fn return_charge(
    percent: f64,
    seconds_left: f64,
    reserve_percent: f64,
    trip_seconds: f64,
) -> Option<f64> {
    (percent > 0.0 && seconds_left > 0.0)
        .then(|| (reserve_percent + trip_seconds * percent / seconds_left).min(100.0))
}

const PX4_RETURN_ALTITUDE: &[&str] = &["RTL_RETURN_ALT"];
const COPTER_RETURN_ALTITUDE: &[&str] = &["RTL_ALT_M", "RTL_ALT"];
const PLANE_RETURN_ALTITUDE: &[&str] = &["RTL_ALTITUDE", "ALT_HOLD_RTL"];

pub fn return_trip(backend: &dyn Backend) -> Option<ReturnTrip> {
    let si = |names: &[&str]| parameter_si(backend, names);
    let fact = |path: &str| value_number(&backend.value(&format!("{path}.rawValue")));
    let (distance_m, altitude_m) = (
        fact("vehicle.distanceToHome")?,
        fact("vehicle.altitudeRelative")?,
    );
    let px4 = || {
        Some(ReturnTrip {
            distance_m,
            altitude_m,
            return_altitude_m: si(PX4_RETURN_ALTITUDE)?,
            cruise_mps: si(&["MPC_XY_CRUISE"])?,
            climb_mps: si(&["MPC_Z_V_AUTO_UP"])?,
            descent_mps: si(&["MPC_LAND_SPEED"])?,
        })
    };
    let apm = || {
        let rtl = si(&["RTL_SPEED_MS", "RTL_SPEED"])
            .filter(|s| *s > 0.0)
            .or_else(|| si(&["WP_SPD", "WPNAV_SPEED"]))?;
        Some(ReturnTrip {
            distance_m,
            altitude_m,
            return_altitude_m: si(COPTER_RETURN_ALTITUDE)?,
            cruise_mps: rtl,
            climb_mps: si(&["WP_SPD_UP", "WPNAV_SPEED_UP"])?,
            descent_mps: si(&["LAND_SPD_MS", "LAND_SPEED"])?,
        })
    };
    px4().or_else(apm)
}

pub fn return_altitude(backend: &dyn Backend) -> Option<f64> {
    parameter_si(
        backend,
        &[
            PX4_RETURN_ALTITUDE,
            COPTER_RETURN_ALTITUDE,
            PLANE_RETURN_ALTITUDE,
        ]
        .concat(),
    )
    .filter(|metres| *metres > 0.0)
}

fn parameter(backend: &dyn Backend, name: &str) -> Option<Value> {
    let fact = backend.value(&format!("vehicle.parameterManager.getParameter(-1,{name})"));
    fact.get("name")
        .and_then(Value::as_str)
        .is_some_and(|n| !n.is_empty())
        .then_some(fact)
}

fn parameter_value(backend: &dyn Backend, name: &str, field: &str) -> Option<f64> {
    parameter(backend, name)?.get(field).and_then(Value::as_f64)
}

pub(crate) fn parameter_si(backend: &dyn Backend, names: &[&str]) -> Option<f64> {
    names.iter().find_map(|name| {
        let fact = parameter(backend, name)?;
        let per_base = crate::pilotunits::raw_per_base(
            fact.get("rawUnits")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        )?;
        Some(fact.get("rawValue")?.as_f64()? / per_base)
    })
}

fn row(label: &str, value: String, severity: i64) -> Value {
    json!({ "label": label, "value": value, "severity": severity, "reported": true })
}

fn unreported(label: &str) -> Value {
    json!({ "label": label, "value": UNREPORTED_TEXT, "severity": SECONDARY, "reported": false })
}

fn with_hint(mut row: Value, hint: &str) -> Value {
    row["hint"] = json!(hint);
    row
}

pub fn consumed_row(consumed: f64, capacity: Option<f64>) -> Value {
    let used = mah_text(consumed);
    match capacity.filter(|c| *c > 0.0) {
        Some(c) if consumed > c => with_hint(
            row(
                "Consumed",
                format!("{used} of {} mAh", mah_text(c)),
                CAUTION,
            ),
            OVER_CAPACITY_HINT,
        ),
        Some(c) => row(
            "Consumed",
            format!("{used} of {} mAh", mah_text(c)),
            SECONDARY,
        ),
        None => row("Consumed", format!("{used} mAh"), SECONDARY),
    }
}

pub fn popup_rows(pack: &PopupPack) -> Vec<Value> {
    let level = severity(pack.charge_state);
    let coloured = if level > 0 { level } else { SECONDARY };
    [
        Some(
            pack.time_remaining
                .map(duration_text)
                .filter(|t| !t.is_empty())
                .map_or_else(
                    || unreported("Time left"),
                    |t| row("Time left", t, coloured),
                ),
        ),
        Some(pack.percent.map_or_else(
            || with_hint(unreported("Charge"), CHARGE_HINT),
            |p| row("Charge", format!("{}%", p.round() as i64), coloured),
        )),
        Some(pack.voltage.clone().map_or_else(
            || unreported("Voltage"),
            |v| match &pack.voltage_problem {
                Some(problem) => with_hint(row("Voltage", format!("{v} V"), CAUTION), problem),
                None => row("Voltage", format!("{v} V"), SECONDARY),
            },
        )),
        pack.consumed_mah
            .map(|c| consumed_row(c, pack.capacity_mah)),
        pack.temperature
            .clone()
            .map(|t| row("Temperature", t, SECONDARY)),
        pack.function.clone().map(|f| row("Function", f, SECONDARY)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn popup_pack(fact: &dyn Fn(&str) -> Value) -> PopupPack {
    let raw = |name: &str| {
        let f = fact(name);
        f.get("rawValue")
            .or(f.get("value"))
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
    let spelled = |name: &str| {
        raw(name).and_then(|_| {
            fact(name)
                .get("valueString")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
    };
    let function = raw("function")
        .filter(|f| *f != FUNCTION_UNKNOWN && *f != FUNCTION_ALL)
        .and_then(|_| {
            fact("function")
                .get("enumOrValueString")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    PopupPack {
        charge_state: fact("chargeState")
            .get("value")
            .and_then(Value::as_i64)
            .unwrap_or(CHARGE_UNDEFINED),
        charge_label: fact("chargeState")
            .get("enumOrValueString")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        percent: raw("percentRemaining"),
        time_remaining: raw("timeRemaining"),
        voltage: spelled("voltage"),
        consumed_mah: raw("mahConsumed"),
        capacity_mah: None,
        temperature: spelled("temperature").map(|t| {
            let units = fact("temperature")
                .get("units")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            format!("{t} {units}").trim_end().to_string()
        }),
        function,
        voltage_problem: None,
    }
}

const COMBINE_PACKS: &str =
    "settings.batteryIndicatorSettings.consolidateMultipleBatteries.rawValue";
const COMBINE_PACKS_UNSET: bool = true;

fn severity_rank(level: &Value) -> usize {
    rank(level.as_str().unwrap_or_default())
}

fn thresholds(backend: &dyn Backend) -> (f64, f64) {
    (
        value_number(&backend.value("settings.batteryIndicatorSettings.threshold1.rawValue"))
            .unwrap_or(80.0),
        value_number(&backend.value("settings.batteryIndicatorSettings.threshold2.rawValue"))
            .unwrap_or(60.0),
    )
}

pub fn indicator_packs(described: &[Value], combine: bool) -> Vec<Value> {
    if !combine || described.len() < 2 {
        return described.to_vec();
    }
    let limiting_percent = |pack: &Value| {
        pack["percent"]
            .as_f64()
            .filter(|p| p.is_finite())
            .unwrap_or(f64::INFINITY)
    };
    described
        .iter()
        .min_by(|a, b| {
            severity_rank(&a["level"])
                .cmp(&severity_rank(&b["level"]))
                .then(limiting_percent(a).total_cmp(&limiting_percent(b)))
        })
        .and_then(Value::as_object)
        .map(|limiting| {
            let combined = [
                (String::from("indicatorLabel"), Value::Null),
                (String::from("packCount"), json!(described.len())),
            ];
            vec![Value::Object(
                limiting.clone().into_iter().chain(combined).collect(),
            )]
        })
        .unwrap_or_default()
}

pub fn battery_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let readers: Vec<_> = (0..pack_count(backend))
        .map(|index| remembered(backend, index))
        .collect();
    let packs: Vec<Pack> = readers.iter().map(|read| pack(read)).collect();
    let stages: Vec<Vec<FailsafeStage>> = (0..packs.len())
        .map(|index| apm_stages(backend, index))
        .collect();
    let problems: Vec<Option<String>> = packs
        .iter()
        .enumerate()
        .map(|(index, p)| voltage_problem(p.voltage, set_for_volts(backend, index, &stages[index])))
        .collect();
    let popup_packs: Vec<PopupPack> = readers
        .iter()
        .enumerate()
        .map(|(index, read)| PopupPack {
            capacity_mah: capacity_mah(backend, index),
            voltage_problem: problems[index].clone(),
            ..popup_pack(read)
        })
        .collect();
    let (threshold1, threshold2) = thresholds(backend);
    let value_display =
        value_number(&backend.value("settings.batteryIndicatorSettings.valueDisplay.rawValue"))
            .map(|v| v as i64)
            .unwrap_or(0);
    let numbered = packs.len() > 1;
    let described: Vec<Value> = packs
        .iter()
        .enumerate()
        .map(|(index, p)| {
            json!({
                "index": index,
                "voltage": p.voltage,
                "current": p.current,
                "percent": p.percent,
                "chargeState": p.charge_state,
                "chargeLabel": p.charge_label,
                "level": worse(
                    level(p.charge_state, p.percent, threshold1, threshold2),
                    problems[index].as_ref().map_or("normal", |_| "caution"),
                ),
                "voltageProblem": problems[index],
                "text": text(p),
                "secondaryText": secondary_text(p),
                "indicatorLabel": numbered.then(|| format!("B{}", index + 1)),
                "indicatorLines": indicator_lines(p, value_display),
                "voltageText": p.voltage_text,
                "currentText": p.current_text,
                "percentText": p.percent_text,
                "facts": detail_facts(&readers[index]),
                "rows": popup_rows(&popup_packs[index]),
            })
        })
        .collect();
    let worst = ["critical", "warning", "caution", "normal"]
        .into_iter()
        .find(|l| described.iter().any(|p| p["level"] == *l))
        .unwrap_or("normal");
    json!({
        "kind": "object",
        "class": "Battery",
        "available": !described.is_empty(),
        "level": worst,
        "text": described.first().map(|p| p["text"].clone()).unwrap_or(Value::String(String::new())),
        "indicatorPacks": indicator_packs(&described, backend.value(COMBINE_PACKS).get("value").and_then(Value::as_bool).unwrap_or(COMBINE_PACKS_UNSET)),
        "packs": described,
        "headline": headline(&popup_packs).map(|shown| limiting_pack_extras(backend, shown, &popup_packs, &packs, &described, &stages)),
    })
}

fn low_charge_text(percent: Option<f64>) -> String {
    percent.map_or_else(
        || "Battery low.".to_string(),
        |p| format!("Battery at {}%.", p.round() as i64),
    )
}

pub fn lowest_charge(
    packs: &[(i64, Option<f64>)],
    threshold1: f64,
    threshold2: f64,
) -> Option<String> {
    let low = rank("warning");
    packs
        .iter()
        .map(|(state, percent)| {
            (
                rank(level(*state, *percent, threshold1, threshold2)),
                *percent,
            )
        })
        .filter(|(severity, _)| *severity <= low)
        .min_by(|(a, pa), (b, pb)| {
            a.cmp(b).then(
                pa.unwrap_or(f64::INFINITY)
                    .total_cmp(&pb.unwrap_or(f64::INFINITY)),
            )
        })
        .map(|(_, percent)| low_charge_text(percent))
}

fn charge_of(backend: &dyn Backend, index: usize) -> (i64, Option<f64>) {
    let fact = |name: &str| backend.value(&pack_fact_path(index, name));
    (
        fact("chargeState")
            .get("value")
            .and_then(Value::as_i64)
            .unwrap_or(CHARGE_UNDEFINED),
        value_number(&fact("percentRemaining")),
    )
}

pub fn low_charge(backend: &dyn Backend) -> Option<String> {
    let (threshold1, threshold2) = thresholds(backend);
    let packs: Vec<(i64, Option<f64>)> = (0..pack_count(backend))
        .map(|index| charge_of(backend, index))
        .collect();
    lowest_charge(&packs, threshold1, threshold2)
}

pub fn failsafe_reserve(action: Option<i64>, critical: Option<f64>) -> f64 {
    match action {
        Some(PX4_LOW_BATTERY_RETURN | PX4_LOW_BATTERY_LAND | PX4_LOW_BATTERY_RETURN_THEN_LAND) => {
            critical.unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

pub fn must_return(percent: Option<f64>, return_at: Option<f64>, flying: bool) -> bool {
    flying
        && percent
            .zip(return_at)
            .is_some_and(|(percent, at)| percent <= at)
}

pub fn drain_per_minute(amps: Option<f64>, capacity_mah: Option<f64>) -> Option<f64> {
    amps.filter(|a| *a > 0.0)
        .zip(capacity_mah.filter(|c| *c > 0.0))
        .map(|(amps, mah)| amps * 1000.0 / mah * 100.0 / 60.0)
}

fn capacity_mah(backend: &dyn Backend, index: usize) -> Option<f64> {
    [
        format!("{}_CAPACITY", apm_prefix(index)),
        format!("BAT{}_CAPACITY", index + 1),
    ]
    .iter()
    .find_map(|name| parameter_value(backend, name, "value"))
    .filter(|mah| *mah > 0.0)
}

fn limiting_pack_extras(
    backend: &dyn Backend,
    mut shown: Value,
    popup_packs: &[PopupPack],
    packs: &[Pack],
    described: &[Value],
    stages: &[Vec<FailsafeStage>],
) -> Value {
    let index = shown["index"].as_u64().unwrap_or(0) as usize;
    let limiting = &popup_packs[index];
    let action = parameter_value(backend, "COM_LOW_BAT_ACT", "rawValue").map(|v| v as i64);
    let critical = parameter_value(backend, "BAT_CRIT_THR", "value");
    let reserve = failsafe_reserve(action, critical);
    let return_at = limiting
        .percent
        .zip(limiting.time_remaining)
        .zip(return_trip(backend).and_then(return_seconds))
        .and_then(|((percent, left), trip)| return_charge(percent, left, reserve, trip));
    let flying = crate::read::flag(&backend.value_fields("vehicle", "flying"), "flying");
    shown["level"] = described
        .get(index)
        .map_or(Value::Null, |pack| pack["level"].clone());
    shown["margin"] = margin_text(
        action,
        parameter_value(backend, "BAT_LOW_THR", "value"),
        critical,
        limiting.percent,
        limiting.time_remaining,
    )
    .or_else(|| {
        apm_margin_text(
            stages.get(index).map_or(&[][..], Vec::as_slice),
            packs
                .get(index)
                .and_then(|pack| pack.voltage)
                .filter(|_| limiting.voltage_problem.is_none()),
            limiting
                .capacity_mah
                .zip(limiting.consumed_mah)
                .map(|(capacity, used)| capacity - used),
        )
    })
    .map_or(Value::Null, Value::String);
    shown["percent"] = limiting.percent.map_or(Value::Null, |p| json!(p));
    shown["secondsLeft"] = limiting.time_remaining.map_or(Value::Null, |s| json!(s));
    shown["timeLeft"] = limiting
        .time_remaining
        .map(duration_text)
        .filter(|t| !t.is_empty())
        .map_or(Value::Null, Value::String);
    shown["reserve"] = json!(reserve);
    shown["returnAt"] = return_at.map_or(Value::Null, |at| json!(at));
    shown["returnNow"] = json!(must_return(limiting.percent, return_at, flying));
    shown["drainPerMinute"] = drain_per_minute(
        packs.get(index).and_then(|pack| pack.current),
        limiting.capacity_mah,
    )
    .map_or(Value::Null, |d| json!(d));
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_readiness_line_names_the_lowest_pack_with_its_own_percent() {
        assert_eq!(
            lowest_charge(&[(CHARGE_OK, Some(0.2))], 80.0, 60.0),
            Some("Battery at 0%.".to_string()),
            "OK at 0% is the SITL case the readiness line missed"
        );
        assert_eq!(
            lowest_charge(&[(3, None)], 80.0, 60.0),
            Some("Battery low.".to_string()),
            "a pack with only a charge state still reads as low"
        );
        assert_eq!(
            lowest_charge(&[(CHARGE_OK, Some(70.0))], 80.0, 60.0),
            None,
            "the yellow band is not a reason to hold a takeoff"
        );
        assert_eq!(
            lowest_charge(
                &[
                    (CHARGE_OK, Some(55.0)),
                    (CHARGE_LOW, Some(80.0)),
                    (CHARGE_OK, Some(40.0))
                ],
                80.0,
                60.0
            ),
            Some("Battery at 40%.".to_string()),
            "the worst pack is picked once, so the percent named is that pack's own"
        );
        assert_eq!(
            lowest_charge(&[(3, Some(90.0)), (CHARGE_OK, Some(10.0))], 80.0, 60.0),
            Some("Battery at 90%.".to_string()),
            "a critical charge state outranks a low percent elsewhere"
        );
        assert_eq!(lowest_charge(&[], 80.0, 60.0), None);
    }

    #[test]
    fn a_render_reads_each_battery_fact_once_however_many_rows_spell_it() {
        struct Counted(std::cell::RefCell<Vec<String>>);
        impl Backend for Counted {
            fn get(&self, path: &str) -> String {
                self.0.borrow_mut().push(path.to_string());
                match path {
                    "vehicle.batteries.count" => json!({ "kind": "value", "value": 2 }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                self.get(p)
            }
            fn set(&self, _p: &str, _v: &str) -> String {
                String::new()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                String::new()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let counted = Counted(std::cell::RefCell::default());
        battery_view(&counted, &[]);
        let read = counted.0.borrow();
        let repeated: Vec<&String> = read
            .iter()
            .filter(|path| read.iter().filter(|other| other == path).count() > 1)
            .collect();
        assert!(
            repeated.is_empty(),
            "read more than once in one render: {repeated:?}"
        );
    }

    fn popup(state: i64, label: &str, percent: Option<f64>, left: Option<f64>) -> PopupPack {
        PopupPack {
            charge_state: state,
            charge_label: label.into(),
            percent,
            time_remaining: left,
            voltage: Some("15.80".into()),
            consumed_mah: None,
            capacity_mah: None,
            temperature: None,
            function: None,
            voltage_problem: None,
        }
    }

    #[test]
    fn without_a_percent_the_headline_never_reads_the_charge_state_as_a_reading() {
        let volts = headline(&[popup(1, "Ok", None, None)]).unwrap();
        assert_eq!(
            (volts["text"].as_str(), volts["severity"].as_i64()),
            (Some("15.80 V"), Some(0)),
            "ArduPilot reports Ok at any charge until a failsafe trips, so the voltage leads"
        );
        let blind = headline(&[PopupPack {
            voltage: None,
            ..popup(1, "Ok", None, None)
        }])
        .unwrap();
        assert_eq!(
            (blind["text"].as_str(), blind["severity"].as_i64()),
            (Some("Charge unknown"), Some(1)),
            "with nothing measured the headline says so, in the caution colour"
        );
        let critical = headline(&[PopupPack {
            voltage: None,
            ..popup(3, "Critical", None, None)
        }])
        .unwrap();
        assert_eq!(
            (critical["text"].as_str(), critical["severity"].as_i64()),
            (Some("Critical"), Some(2)),
            "an alarming state is still worth leading with"
        );
    }

    #[test]
    fn the_core_readings_stay_listed_and_say_when_the_aircraft_sends_none() {
        let rows = popup_rows(&PopupPack {
            voltage: None,
            ..popup(1, "Ok", None, None)
        });
        let shown: Vec<(&str, &str, bool)> = rows
            .iter()
            .map(|r| {
                (
                    r["label"].as_str().unwrap(),
                    r["value"].as_str().unwrap(),
                    r["reported"].as_bool().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            shown,
            vec![
                ("Time left", "Not reported", false),
                ("Charge", "Not reported", false),
                ("Voltage", "Not reported", false),
            ]
        );
        assert_eq!(
            rows[1]["hint"], CHARGE_HINT,
            "the charge row says where to fix it"
        );
        assert!(rows[0].get("hint").is_none());
    }

    #[test]
    fn consumed_reads_against_the_pack_and_flags_more_than_it_holds() {
        let row = |used: f64, capacity: Option<f64>| {
            let shown = consumed_row(used, capacity);
            (
                shown["value"].as_str().unwrap().to_string(),
                shown["severity"].as_i64(),
                shown["hint"].as_str().map(str::to_string),
            )
        };
        assert_eq!(
            row(3200.4, Some(5000.0)),
            ("3,200 of 5,000 mAh".to_string(), Some(-1), None)
        );
        assert_eq!(
            row(501833.0, Some(5000.0)),
            (
                "501,833 of 5,000 mAh".to_string(),
                Some(1),
                Some(OVER_CAPACITY_HINT.to_string())
            ),
            "more than the pack holds is a sensor to check, not a reading to trust"
        );
        assert_eq!(
            row(501833.0, None),
            ("501,833 mAh".to_string(), Some(-1), None)
        );
        assert_eq!(
            row(800.0, Some(-1.0)).0,
            "800 mAh",
            "PX4's -1 capacity is unset"
        );
    }

    fn stage(action: i64, label: &str, volts: Option<f64>, mah: Option<f64>) -> FailsafeStage {
        FailsafeStage {
            action,
            label: label.into(),
            volts,
            mah,
        }
    }

    #[test]
    fn the_ardupilot_margin_names_the_first_failsafe_that_takes_over() {
        assert_eq!(
            apm_margin_text(&[stage(2, "RTL", Some(10.5), None)], Some(11.8), None).as_deref(),
            Some("RTL at 10.5 V  \u{00b7}  1.3 V to go")
        );
        assert_eq!(
            apm_margin_text(
                &[stage(2, "RTL", Some(10.5), Some(1000.0))],
                Some(11.8),
                Some(3200.0)
            )
            .as_deref(),
            Some("RTL at 10.5 V or 1,000 mAh left  \u{00b7}  2,200 mAh to go"),
            "consumption is steadier than a voltage that sags under load"
        );
        assert_eq!(
            apm_margin_text(
                &[
                    stage(0, "None", Some(10.5), None),
                    stage(1, "Land", Some(9.9), None)
                ],
                Some(10.2),
                None
            )
            .as_deref(),
            Some("Land at 9.9 V  \u{00b7}  0.3 V to go"),
            "a warn-only low stage gives way to the critical stage that lands"
        );
        assert_eq!(
            apm_margin_text(&[stage(0, "None", Some(10.5), None)], Some(10.4), None).as_deref(),
            Some("Warns at 10.5 V  \u{00b7}  reached")
        );
        assert_eq!(
            apm_margin_text(&[stage(2, "RTL", None, None)], Some(11.8), None),
            None,
            "BATT_LOW_VOLT and BATT_LOW_MAH at 0 disable the failsafe"
        );
        assert_eq!(apm_margin_text(&[], Some(11.8), None), None);
    }

    #[test]
    fn the_ardupilot_failsafe_parameters_are_read_per_pack() {
        let apm = Params(vec![
            ("BATT2_FS_LOW_ACT", 2.0),
            ("BATT2_LOW_VOLT", 10.5),
            ("BATT2_LOW_MAH", 0.0),
            ("BATT2_FS_CRT_ACT", 1.0),
            ("BATT2_CRT_VOLT", 9.9),
        ]);
        assert_eq!(
            apm_stages(&apm, 1)
                .iter()
                .map(|s| (s.action, s.volts, s.mah))
                .collect::<Vec<_>>(),
            vec![(2, Some(10.5), None), (1, Some(9.9), None)]
        );
        assert!(
            apm_stages(&apm, 0).is_empty(),
            "a PX4 vehicle has no BATT_ failsafes"
        );
    }

    #[test]
    fn durations_read_like_battery_indicator() {
        assert_eq!(
            (
                duration_text(42.4),
                duration_text(125.0),
                duration_text(3725.0),
                duration_text(-1.0)
            ),
            (
                "42 sec".to_string(),
                "2:05".to_string(),
                "1:02:05".to_string(),
                String::new()
            )
        );
    }

    #[test]
    fn the_headline_follows_the_worst_pack_like_battery_indicator() {
        let calm = headline(&[popup(1, "Ok", Some(72.6), Some(600.0))]).unwrap();
        assert_eq!(
            (
                calm["text"].as_str(),
                calm["detail"].as_str(),
                calm["severity"].as_i64()
            ),
            (Some("73%"), Some("10:00 left"), Some(0)),
            "a healthy pack leads with its charge"
        );
        let mixed = headline(&[
            popup(1, "Ok", Some(80.0), None),
            popup(3, "Critical", Some(12.0), Some(90.0)),
        ])
        .unwrap();
        assert_eq!(
            (
                mixed["text"].as_str(),
                mixed["detail"].as_str(),
                mixed["severity"].as_i64()
            ),
            (
                Some("Critical"),
                Some("1:30 left  \u{00b7}  12%  \u{00b7}  lowest of 2"),
                Some(2)
            ),
            "an alarming pack leads with its state and adds its charge"
        );
        assert!(headline(&[]).is_none());
    }

    #[test]
    fn two_healthy_packs_lead_with_the_lower_one_and_say_so() {
        let both = headline(&[
            popup(1, "Ok", Some(90.0), Some(810.0)),
            popup(1, "Ok", Some(79.0), Some(711.0)),
        ])
        .unwrap();
        assert_eq!(
            (
                both["text"].as_str(),
                both["detail"].as_str(),
                both["index"].as_u64()
            ),
            (
                Some("79%"),
                Some("11:51 left  \u{00b7}  lowest of 2"),
                Some(1)
            )
        );
    }

    #[test]
    fn the_turn_back_point_is_the_charge_the_flight_home_costs_on_top_of_the_reserve() {
        let trip = ReturnTrip {
            distance_m: 600.0,
            altitude_m: 40.0,
            return_altitude_m: 60.0,
            cruise_mps: 5.0,
            climb_mps: 2.0,
            descent_mps: 1.0,
        };
        assert_eq!(
            return_seconds(trip),
            Some(10.0 + 120.0 + 60.0),
            "climb to the return altitude, fly home, then descend at landing speed the whole way"
        );
        assert_eq!(
            return_seconds(ReturnTrip {
                altitude_m: 90.0,
                ..trip
            }),
            Some(0.0 + 120.0 + 90.0),
            "already above the return altitude it comes home where it is"
        );
        assert_eq!(
            return_seconds(ReturnTrip {
                cruise_mps: 0.0,
                ..trip
            }),
            None,
            "an unknown speed promises nothing"
        );
        assert_eq!(
            return_charge(60.0, 600.0, 7.0, 190.0),
            Some(7.0 + 19.0),
            "190 s at 0.1 %/s costs 19 % on top of the 7 % the aircraft lands at"
        );
        assert_eq!(return_charge(60.0, 0.0, 7.0, 190.0), None);
    }

    struct Params(Vec<(&'static str, f64)>);
    impl Backend for Params {
        fn get(&self, path: &str) -> String {
            let name = path
                .trim_end_matches(".rawValue")
                .trim_start_matches("vehicle.parameterManager.getParameter(-1,")
                .trim_end_matches(')');
            let units = match name {
                "RTL_ALT" | "ALT_HOLD_RTL" => "cm",
                "RTL_SPEED" | "WPNAV_SPEED" | "WPNAV_SPEED_UP" | "LAND_SPEED" => "cm/s",
                "RTL_RETURN_ALT" | "RTL_ALT_M" => "m",
                _ => "m/s",
            };
            let absent = match path.starts_with("vehicle.parameterManager.") {
                true => {
                    json!({ "kind": "fact", "name": "", "value": 0, "rawValue": 0, "rawUnits": "" })
                }
                false => json!({ "kind": "null" }),
            };
            self.0.iter().find(|(n, _)| *n == name).map_or(absent, |(n, v)| json!({ "kind": "fact", "name": n, "value": v, "rawValue": v, "rawUnits": units })).to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            self.get(p)
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_trip_home_reads_each_firmware_in_its_own_units() {
        let place = [
            ("vehicle.distanceToHome", 600.0),
            ("vehicle.altitudeRelative", 40.0),
        ];
        let px4 = Params(
            place
                .into_iter()
                .chain([
                    ("RTL_RETURN_ALT", 60.0),
                    ("MPC_XY_CRUISE", 5.0),
                    ("MPC_Z_V_AUTO_UP", 2.0),
                    ("MPC_LAND_SPEED", 1.0),
                ])
                .collect(),
        );
        assert_eq!(return_trip(&px4).and_then(return_seconds), Some(190.0));
        let apm = Params(
            place
                .into_iter()
                .chain([
                    ("RTL_ALT", 6000.0),
                    ("RTL_SPEED", 0.0),
                    ("WPNAV_SPEED", 500.0),
                    ("WPNAV_SPEED_UP", 200.0),
                    ("LAND_SPEED", 100.0),
                ])
                .collect(),
        );
        assert_eq!(
            return_trip(&apm).and_then(return_seconds),
            Some(190.0),
            "ArduPilot speaks centimetres, and RTL_SPEED 0 means fly home at WPNAV_SPEED; the PX4 names it lacks read as absent, not 0"
        );
        let copter_4_7 = Params(
            place
                .into_iter()
                .chain([
                    ("RTL_ALT_M", 60.0),
                    ("RTL_SPEED_MS", 0.0),
                    ("WP_SPD", 5.0),
                    ("WP_SPD_UP", 2.0),
                    ("LAND_SPD_MS", 1.0),
                ])
                .collect(),
        );
        assert_eq!(
            return_trip(&copter_4_7).and_then(return_seconds),
            Some(190.0),
            "ArduCopter 4.7 renamed these into metres"
        );
        let unplaced = Params(vec![
            ("vehicle.distanceToHome", 600.0),
            ("RTL_RETURN_ALT", 60.0),
            ("MPC_XY_CRUISE", 5.0),
            ("MPC_Z_V_AUTO_UP", 2.0),
            ("MPC_LAND_SPEED", 1.0),
        ]);
        assert_eq!(
            return_trip(&unplaced),
            None,
            "an unknown height is not guessed as the ground"
        );
    }

    #[test]
    fn the_return_altitude_is_read_under_each_firmwares_name_and_unit() {
        let read = |name: &'static str, value: f64| return_altitude(&Params(vec![(name, value)]));
        assert_eq!(read("RTL_RETURN_ALT", 60.0), Some(60.0), "PX4");
        assert_eq!(read("RTL_ALT_M", 60.0), Some(60.0), "ArduCopter 4.7");
        assert_eq!(
            read("RTL_ALT", 6000.0),
            Some(60.0),
            "older ArduCopter, in centimetres"
        );
        assert_eq!(
            read("ALT_HOLD_RTL", 10000.0),
            Some(100.0),
            "ArduPlane, in centimetres"
        );
        assert_eq!(
            read("RTL_ALT", 0.0),
            None,
            "0 returns at the current altitude, so there is no return altitude to show"
        );
        assert_eq!(return_altitude(&Params(vec![])), None);
        assert_eq!(
            ReturnTrip {
                distance_m: 600.0,
                altitude_m: 40.0,
                return_altitude_m: -1.0,
                cruise_mps: 5.0,
                climb_mps: 2.0,
                descent_mps: 1.0,
            }
            .flown_at_m(),
            40.0,
            "a return altitude below the aircraft never takes it lower"
        );
    }

    #[test]
    fn the_turn_back_warning_is_for_the_air_and_the_reserve_follows_the_failsafe() {
        assert!(must_return(Some(20.0), Some(26.0), true));
        assert!(
            !must_return(Some(20.0), Some(26.0), false),
            "on the pad a low pack is not a reason to shout return now"
        );
        assert!(!must_return(Some(30.0), Some(26.0), true));
        assert_eq!(failsafe_reserve(Some(3), Some(7.0)), 7.0);
        assert_eq!(
            failsafe_reserve(Some(0), Some(7.0)),
            0.0,
            "a warn-only failsafe never lands, so there is no reserve to keep"
        );
        assert_eq!(failsafe_reserve(None, None), 0.0);
    }

    #[test]
    fn the_drain_is_the_share_of_the_pack_a_minute_at_this_current_costs() {
        assert!(
            (drain_per_minute(Some(36.0), Some(3300.0)).unwrap() - 18.18).abs() < 0.01,
            "36 A on a 3.3 Ah pack empties 18% a minute"
        );
        assert_eq!(
            drain_per_minute(Some(0.0), Some(3300.0)),
            None,
            "no draw, nothing to show"
        );
        assert_eq!(
            drain_per_minute(Some(-2.0), Some(3300.0)),
            None,
            "a charging pack is not drained"
        );
        assert_eq!(
            drain_per_minute(Some(10.0), Some(0.0)),
            None,
            "an unset capacity gives no scale"
        );
        assert_eq!(drain_per_minute(Some(10.0), None), None);
    }

    #[test]
    fn the_margin_names_the_failsafe_and_the_time_before_it() {
        assert_eq!(
            margin_text(Some(3), Some(15.0), Some(7.0), Some(79.0), Some(711.0)).as_deref(),
            Some("Returns home at 7%  \u{00b7}  about 11 min to go")
        );
        assert_eq!(
            margin_text(Some(2), Some(15.0), Some(7.0), Some(50.0), None).as_deref(),
            Some("Lands at 7%  \u{00b7}  43 points to go")
        );
        assert_eq!(
            margin_text(Some(2), Some(15.0), Some(7.0), Some(50.0), Some(9.0)).as_deref(),
            Some("Lands at 7%  \u{00b7}  under a minute to go"),
            "nine seconds left is not a minute"
        );
        assert_eq!(
            margin_text(Some(0), Some(15.0), Some(7.0), Some(12.0), Some(300.0)).as_deref(),
            Some("Warns at 15%  \u{00b7}  reached")
        );
        assert_eq!(
            margin_text(None, Some(15.0), Some(7.0), Some(79.0), Some(711.0)),
            None,
            "a firmware without these parameters says nothing rather than guess"
        );
        assert_eq!(
            margin_text(Some(3), None, None, Some(79.0), Some(711.0)),
            None
        );
    }

    #[test]
    fn pack_rows_follow_the_popup_order_and_colour_only_time_and_charge() {
        let rows = popup_rows(&popup(2, "Low", Some(30.0), Some(65.0)));
        let labels: Vec<&str> = rows.iter().filter_map(|r| r["label"].as_str()).collect();
        assert_eq!(labels, vec!["Time left", "Charge", "Voltage"]);
        assert_eq!(
            (
                rows[0]["severity"].as_i64(),
                rows[2]["severity"].as_i64(),
                rows[2]["value"].as_str()
            ),
            (Some(1), Some(-1), Some("15.80 V"))
        );
        assert_eq!(
            popup_rows(&popup(1, "Ok", Some(80.0), None))[0]["severity"].as_i64(),
            Some(-1),
            "a healthy pack's charge is grey, as _packColor defaults to colorGrey"
        );
    }

    #[test]
    fn a_pack_temperature_reads_in_the_unit_its_fact_was_cooked_in() {
        let fahrenheit = |name: &str| match name {
            "temperature" => {
                json!({ "kind": "fact", "name": "temperature", "rawValue": 21.0, "value": 69.8, "valueString": "70", "units": "\u{b0}F" })
            }
            _ => Value::Null,
        };
        let rows = popup_rows(&popup_pack(&fahrenheit));
        assert_eq!(
            rows.iter()
                .find(|r| r["label"] == "Temperature")
                .and_then(|r| r["value"].as_str()),
            Some("70 \u{b0}F"),
            "a pack cooked to Fahrenheit says so instead of labelling 70 F as 70 C"
        );
    }

    #[test]
    fn severity_ranks_charge_states_like_severity_of() {
        assert_eq!((4..=6).map(severity).collect::<Vec<_>>(), vec![3, 3, 3]);
        assert_eq!(
            (severity(3), severity(2), severity(7), severity(0)),
            (2, 1, 0, 0),
            "charging is not alarming"
        );
        let tie = headline(&[
            popup(2, "Low", Some(40.0), None),
            popup(2, "Low", Some(20.0), None),
        ])
        .unwrap();
        assert_eq!(
            (tie["detail"].as_str(), tie["index"].as_u64()),
            (Some("20%  \u{00b7}  lowest of 2"), Some(1)),
            "an equal severity leads with the lower charge, as the Fly bar ring does"
        );
    }

    fn pack_facts(percent: Option<f64>, state: i64, label: &str) -> Vec<(String, Value)> {
        [
            Some(("voltage".to_string(), json!({ "kind": "fact", "name": "voltage", "value": 15.8, "valueString": "15.80", "units": "V" }))),
            Some(("current".to_string(), json!({ "kind": "fact", "name": "current", "value": 12.5, "valueString": "12.50", "units": "A" }))),
            Some(("chargeState".to_string(), json!({ "kind": "fact", "name": "chargeState", "value": state, "enumOrValueString": label }))),
            percent.map(|p| ("percentRemaining".to_string(), json!({ "kind": "fact", "name": "percentRemaining", "value": p, "valueString": format!("{p:.0}"), "units": "%" }))),
            Some(("instantPower".to_string(), json!({ "kind": "fact", "name": "instantPower", "value": 197.5, "valueString": "197.50", "units": "W" }))),
            Some(("mahConsumed".to_string(), json!({ "kind": "fact", "name": "mahConsumed", "value": 340.0, "valueString": "340", "units": "mAh" }))),
            Some(("temperature".to_string(), json!({ "kind": "fact", "name": "temperature", "value": 0.0, "valueString": "", "units": "\u{00b0}C" }))),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn detail_facts_of(facts: &[(String, Value)]) -> Value {
        struct One(Vec<(String, Value)>);
        impl Backend for One {
            fn get(&self, path: &str) -> String {
                let name = path.rsplit('.').next().unwrap_or_default();
                self.0
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, f)| f.clone())
                    .unwrap_or(json!({ "kind": "null" }))
                    .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                self.get(p)
            }
            fn set(&self, _p: &str, _v: &str) -> String {
                String::new()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                String::new()
            }
            fn watch(&self, _p: &[String]) {}
        }
        detail_facts(&remembered(&One(facts.to_vec()), 0))
    }

    fn pack_of(facts: &[(String, Value)]) -> Pack {
        pack(&|name: &str| {
            facts
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, f)| f.clone())
                .unwrap_or(Value::Null)
        })
    }

    #[test]
    fn details_carry_the_value_for_total_draw_and_a_function_qgc_names() {
        let base = pack_facts(Some(50.0), 1, "OK");
        let shown = detail_facts_of(&base);
        assert_eq!(
            shown
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["name"] == "instantPower")
                .unwrap()["value"],
            197.5,
            "BatteryIndicator sums instantPower, so the number has to travel"
        );
        let propulsion = base.iter().cloned().chain(std::iter::once(("function".to_string(), json!({ "kind": "fact", "name": "function", "value": 2, "enumOrValueString": "Propulsion" })))).collect::<Vec<_>>();
        assert_eq!(
            detail_facts_of(&propulsion)[0],
            json!({ "name": "function", "valueString": "Propulsion", "value": 2, "units": "" })
        );
        let all = base.iter().cloned().chain(std::iter::once(("function".to_string(), json!({ "kind": "fact", "name": "function", "value": 1, "enumOrValueString": "All" })))).collect::<Vec<_>>();
        assert!(
            detail_facts_of(&all)
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["name"] != "function"),
            "UNKNOWN and ALL are not shown"
        );
    }

    #[test]
    fn the_indicator_thresholds_apply_whatever_charge_state_the_vehicle_reports() {
        assert_eq!(
            level(CHARGE_OK, Some(0.0), 80.0, 60.0),
            "warning",
            "ArduPilot reports OK until a low-battery failsafe trips, and SITL drained to 0% by consumption never trips one; QGC's rule drew that as a normal pack"
        );
        assert_eq!(level(CHARGE_OK, Some(70.0), 80.0, 60.0), "caution");
        assert_eq!(level(CHARGE_OK, Some(95.0), 80.0, 60.0), "normal");
        assert_eq!(level(CHARGE_OK, None, 80.0, 60.0), "normal");
        assert_eq!(
            level(7, Some(10.0), 80.0, 60.0),
            "warning",
            "a pack on charge is still nearly empty"
        );
        assert_eq!(level(CHARGE_UNDEFINED, Some(85.0), 80.0, 60.0), "normal");
        assert_eq!(level(CHARGE_UNDEFINED, Some(70.0), 80.0, 60.0), "caution");
        assert_eq!(level(CHARGE_UNDEFINED, Some(55.0), 80.0, 60.0), "warning");
        assert_eq!(level(CHARGE_UNDEFINED, None, 80.0, 60.0), "normal");
        assert_eq!(level(CHARGE_LOW, Some(90.0), 80.0, 60.0), "warning");
        assert_eq!(
            level(3, Some(90.0), 80.0, 60.0),
            "critical",
            "a charge state can only make the reading worse than the percent says"
        );
        assert_eq!(level(6, None, 80.0, 60.0), "critical");
        assert_eq!(level(7, None, 80.0, 60.0), "normal");
    }

    #[test]
    fn nearly_full_reads_as_full_and_a_missing_percent_falls_back_to_voltage_then_charge_state() {
        let packs: Vec<Pack> = [
            pack_facts(Some(99.2), CHARGE_OK, "OK"),
            pack_facts(Some(72.0), CHARGE_UNDEFINED, "Undefined"),
            pack_facts(None, CHARGE_LOW, "Low"),
        ]
        .iter()
        .map(|f| pack_of(f))
        .collect();
        assert_eq!(text(&packs[0]), "100%");
        assert_eq!(text(&packs[1]), "72%");
        assert_eq!(text(&packs[2]), "15.80V");
        assert_eq!(packs[0].voltage_text, "15.80V");
        let bare = Pack {
            voltage: None,
            current: None,
            percent: None,
            charge_state: CHARGE_LOW,
            charge_label: "Low".into(),
            percent_text: String::new(),
            voltage_text: String::new(),
            current_text: String::new(),
            time_remaining_text: None,
        };
        assert_eq!(text(&bare), "Low");
        assert_eq!(secondary_text(&bare), "Low");
        let empty = Pack {
            charge_state: CHARGE_UNDEFINED,
            charge_label: String::new(),
            ..bare
        };
        assert_eq!(text(&empty), "n/a");
        let calm = Pack {
            charge_state: CHARGE_OK,
            charge_label: "Ok".into(),
            ..empty
        };
        assert_eq!(
            (text(&calm), secondary_text(&calm)),
            ("n/a".to_string(), "n/a".to_string()),
            "ArduPilot reports Ok at any charge until a failsafe trips, so the chip never shows it as a reading"
        );
    }

    #[test]
    fn indicator_lines_follow_value_display_like_battery_indicator() {
        let pack = Pack {
            voltage: Some(15.8),
            current: None,
            percent: Some(42.0),
            charge_state: CHARGE_OK,
            charge_label: "OK".into(),
            percent_text: "42%".into(),
            voltage_text: "15.80V".into(),
            current_text: String::new(),
            time_remaining_text: Some("00:12:00".into()),
        };
        assert_eq!(indicator_lines(&pack, 0), ["42%"]);
        assert_eq!(indicator_lines(&pack, 1), ["00:12:00"]);
        assert_eq!(indicator_lines(&pack, 2), ["42%", "00:12:00"]);
    }

    #[test]
    fn the_view_reports_the_worst_pack() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, path: &str) -> String {
                let packs = [
                    pack_facts(Some(90.0), CHARGE_OK, "OK"),
                    pack_facts(Some(50.0), CHARGE_UNDEFINED, "Undefined"),
                ];
                let fact = path
                    .strip_prefix("vehicle.batteries.")
                    .and_then(|rest| rest.split_once('.'))
                    .and_then(|(index, name)| {
                        packs
                            .get(index.parse::<usize>().ok()?)?
                            .iter()
                            .find(|(n, _)| n == name)
                            .map(|(_, f)| f.clone())
                    });
                match (path, fact) {
                    (_, Some(fact)) => fact,
                    ("vehicle.batteries.count", _) => json!({ "kind": "value", "value": 2 }),
                    ("settings.batteryIndicatorSettings.threshold1.rawValue", _) => {
                        json!({ "kind": "value", "value": 80 })
                    }
                    ("settings.batteryIndicatorSettings.threshold2.rawValue", _) => {
                        json!({ "kind": "value", "value": 60 })
                    }
                    ("settings.batteryIndicatorSettings.valueDisplay.rawValue", _) => {
                        json!({ "kind": "value", "value": 2 })
                    }
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                self.get(p)
            }
            fn set(&self, _p: &str, _v: &str) -> String {
                String::new()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                String::new()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let view = battery_view(&Fake, &[]);
        assert!(
            deps().len() >= 4 + 12,
            "after a read the packs the vehicle reported are watched"
        );
        assert!(deps().contains(&"vehicle.batteries.1.chargeState".to_string()));
        assert_eq!(view["available"], true);
        assert_eq!(view["level"], "warning");
        assert_eq!(view["text"], "90%");
        assert_eq!(view["packs"][1]["level"], "warning");
        assert_eq!(view["packs"][0]["secondaryText"], "15.80V");
        assert_eq!(
            view["packs"][0]["currentText"], "12.50A",
            "the head was formatting this itself as %.2f A, which is locale-independent and prints a full stop where the Fact prints whatever the operator's locale does"
        );
        assert_eq!(
            (
                view["packs"][0]["indicatorLabel"].clone(),
                view["packs"][1]["indicatorLabel"].clone()
            ),
            (json!("B1"), json!("B2"))
        );
        assert_eq!(view["packs"][0]["indicatorLines"], json!(["90%", "15.80V"]));
        assert_eq!(
            view["packs"][0]["percentText"], "90%",
            "and this was on the struct already and simply never served, so a head had nothing to read and spelled its own"
        );
        assert_eq!(
            view["indicatorPacks"].as_array().map(Vec::len),
            Some(1),
            "two packs show as one indicator unless the pilot turns combining off"
        );
        assert_eq!(view["indicatorPacks"][0]["level"], "warning");
        assert_eq!(view["indicatorPacks"][0]["packCount"], 2);
    }

    #[test]
    fn the_combined_indicator_shows_the_pack_that_limits_the_flight() {
        let pack = |level: &str, percent: f64| json!({ "level": level, "percent": percent, "indicatorLabel": "B" });
        let packs = [
            pack("normal", 79.0),
            pack("normal", 57.0),
            pack("normal", 91.0),
        ];
        let shown = indicator_packs(&packs, true);
        assert_eq!(
            (
                shown.len(),
                shown[0]["percent"].as_f64(),
                shown[0]["packCount"].clone(),
                shown[0]["indicatorLabel"].clone()
            ),
            (1, Some(57.0), json!(3), Value::Null)
        );
        let warned = indicator_packs(&[pack("normal", 40.0), pack("warning", 60.0)], true);
        assert_eq!(
            warned[0]["level"], "warning",
            "a pack the vehicle flags outranks a lower charge it does not"
        );
        assert_eq!(indicator_packs(&packs, false).len(), 3);
        assert_eq!(indicator_packs(&packs[..1], true)[0]["indicatorLabel"], "B");
    }

    #[test]
    fn a_pack_carries_the_facts_the_detail_panel_names() {
        let facts = pack_facts(Some(90.0), CHARGE_OK, "OK");
        let view = detail_facts_of(&facts);
        let named: Vec<&str> = view
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            named,
            ["voltage", "current", "instantPower", "mahConsumed"],
            "the head reads a list, not scalars, and a fact the vehicle never spelled is absent rather than blank"
        );
        assert_eq!(view[2]["valueString"], "197.50");
        assert_eq!(
            view[2]["units"], "W",
            "raw, because the head runs its own Units.display over it"
        );
        assert!(
            PACK_FACTS.contains(&"temperature"),
            "watched even when unspelled, or the view never recomputes when it starts arriving"
        );
    }

    #[test]
    fn without_packs_nothing_is_available() {
        struct Empty;
        impl Backend for Empty {
            fn get(&self, _p: &str) -> String {
                json!({ "kind": "null" }).to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                self.get("")
            }
            fn set(&self, _p: &str, _v: &str) -> String {
                String::new()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                String::new()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let view = battery_view(&Empty, &[]);
        assert_eq!(view["available"], false);
        assert_eq!(view["level"], "normal");
    }

    #[test]
    fn a_pack_voltage_no_battery_could_give_is_named_against_what_the_pack_is_set_up_for() {
        assert_eq!(
            voltage_problem(Some(351.6), Some(43.5)),
            Some(format!(
                "No pack set up for 43.5 V reads that high. {CALIBRATE_VOLTAGE}"
            )),
            "BATT_VOLT_MULT 2631 made a 12S pack armed at 43.5 V read 351.6 V, and the low-voltage failsafe could never trip"
        );
        assert_eq!(
            voltage_problem(Some(52.2), Some(43.5)),
            None,
            "a full high-voltage 12S pack is real"
        );
        assert_eq!(
            voltage_problem(Some(50.4), Some(30.0)),
            None,
            "a Li-ion pack whose critical failsafe sits at 2.5 V a cell is real when full"
        );
        assert_eq!(
            voltage_problem(Some(100.0), Some(43.5)),
            Some(format!(
                "No pack set up for 43.5 V reads that high. {CALIBRATE_VOLTAGE}"
            )),
            "a multiplier twice too high is caught too"
        );
        assert_eq!(
            voltage_problem(Some(351.6), None),
            Some(format!(
                "More than any drone pack holds. {CALIBRATE_VOLTAGE}"
            )),
            "with no voltage set up the reading is held against the largest packs that fly"
        );
        assert_eq!(
            voltage_problem(Some(100.0), None),
            None,
            "a 24S pack is real"
        );
        assert_eq!(voltage_problem(None, Some(43.5)), None);
    }

    #[test]
    fn a_pack_is_set_up_for_its_arming_voltage_and_the_voltage_failsafes_that_act() {
        let apm = Params(vec![
            ("BATT_ARM_VOLT", 43.5),
            ("BATT_FS_LOW_ACT", 2.0),
            ("BATT_LOW_VOLT", 40.0),
            ("BATT_FS_CRT_ACT", 1.0),
            ("BATT_CRT_VOLT", 37.0),
        ]);
        assert_eq!(set_for_volts(&apm, 0, &apm_stages(&apm, 0)), Some(43.5));
        let warn_only = Params(vec![("BATT_FS_LOW_ACT", 0.0), ("BATT_LOW_VOLT", 10.5)]);
        assert_eq!(
            set_for_volts(&warn_only, 0, &apm_stages(&warn_only, 0)),
            None,
            "ArduPilot's 10.5 V default stays on every pack that never set a failsafe, so a warn-only stage says nothing about the pack"
        );
        let second = Params(vec![("BATT2_FS_LOW_ACT", 2.0), ("BATT2_LOW_VOLT", 21.0)]);
        assert_eq!(
            set_for_volts(&second, 1, &apm_stages(&second, 1)),
            Some(21.0)
        );
        assert_eq!(set_for_volts(&second, 0, &apm_stages(&second, 0)), None);
    }

    #[test]
    fn a_pack_reading_an_impossible_voltage_turns_caution_and_its_voltage_row_says_how_to_fix_it() {
        struct Miscalibrated;
        impl Backend for Miscalibrated {
            fn get(&self, path: &str) -> String {
                let volts = json!({ "kind": "fact", "name": "voltage", "value": 351.6, "valueString": "351.60", "units": "V" });
                let facts: Vec<(String, Value)> = pack_facts(Some(98.0), CHARGE_OK, "OK")
                    .into_iter()
                    .map(|(name, fact)| match name.as_str() {
                        "voltage" => (name, volts.clone()),
                        _ => (name, fact),
                    })
                    .collect();
                let fact = path
                    .strip_prefix("vehicle.batteries.0.")
                    .and_then(|name| facts.iter().find(|(n, _)| n == name))
                    .map(|(_, f)| f.to_string());
                match (path, fact) {
                    (_, Some(fact)) => fact,
                    ("vehicle.batteries.count", _) => {
                        json!({ "kind": "value", "value": 1 }).to_string()
                    }
                    (parameter, _) if parameter.starts_with("vehicle.parameterManager.") => {
                        Params(vec![
                            ("BATT_ARM_VOLT", 43.5),
                            ("BATT_FS_LOW_ACT", 2.0),
                            ("BATT_LOW_VOLT", 40.0),
                        ])
                        .get(parameter)
                    }
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                self.get(p)
            }
            fn set(&self, _p: &str, _v: &str) -> String {
                String::new()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                String::new()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let view = battery_view(&Miscalibrated, &[]);
        let problem = format!("No pack set up for 43.5 V reads that high. {CALIBRATE_VOLTAGE}");
        assert_eq!(
            view["packs"][0]["level"], "caution",
            "98% is no comfort when the voltage the failsafes read is impossible"
        );
        assert_eq!(view["packs"][0]["voltageProblem"], problem);
        assert_eq!(
            view["headline"]["margin"], "Failsafe at 40.0 V",
            "the gap to the failsafe is not counted from a voltage that can't be real; it said 311.6 V to go"
        );
        let voltage = view["packs"][0]["rows"]
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["label"] == "Voltage"))
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            (
                voltage["value"].clone(),
                voltage["severity"].clone(),
                voltage["hint"].clone()
            ),
            (json!("351.60 V"), json!(CAUTION), json!(problem))
        );
        assert_eq!(
            unreal_voltage(&Miscalibrated),
            Some("Battery reads 351.6 V, which can't be real. Calibrate its voltage sensor in Power setup.".to_string())
        );
    }
}
