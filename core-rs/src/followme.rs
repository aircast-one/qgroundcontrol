use serde_json::{Value, json};

use crate::read::{flag, integer, object, text, value_number};
use crate::router::Backend;

pub const SETTING: &str = "settings.appSettings.followTarget.rawValue";
pub const GCS_POSITION: &str = "positionManager.gcsPosition";
pub const GCS_HEADING: &str = "positionManager.gcsHeading";
pub const GCS_HORIZONTAL_ACCURACY: &str = "positionManager.gcsPositionHorizontalAccuracy";
pub const GCS_TIMESTAMP: &str = "positionManager.gcsPositionTimestamp";

pub const DEPS: &[&str] = &[SETTING, "vehicles.vehicles.count", "vehicle.id", "vehicle.flightMode", "vehicle.homePosition", GCS_POSITION, GCS_HEADING, GCS_HORIZONTAL_ACCURACY, GCS_TIMESTAMP];

const FIELDS: &str = "id,flightMode,followFlightMode,apmFirmware,homePosition";

pub const MOTION_INTERVAL_MS: u64 = 250;
pub const ALLOWED_FIX_AGE_MS: u64 = 5000;

pub const ESTIMATION_POSITION: u8 = 1 << 0;
pub const ESTIMATION_VELOCITY: u8 = 1 << 1;
pub const ESTIMATION_HEADING: u8 = 1 << 4;

pub const HEADING_UNKNOWN_CDEG: u16 = u16::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Never,
    Always,
    FollowMe,
}

impl Mode {
    pub fn token(self) -> &'static str {
        match self {
            Mode::Never => "never",
            Mode::Always => "always",
            Mode::FollowMe => "followMe",
        }
    }

    pub fn from_setting(value: Option<f64>) -> Option<Mode> {
        match value? as i64 {
            0 => Some(Mode::Never),
            1 => Some(Mode::Always),
            2 => Some(Mode::FollowMe),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Fix {
    pub valid: bool,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_amsl_m: Option<f64>,
    pub heading_deg: Option<f64>,
    pub ground_speed_m_s: Option<f64>,
    pub vertical_speed_down_m_s: Option<f64>,
    pub horizontal_accuracy_m: Option<f64>,
    pub vertical_accuracy_m: Option<f64>,
    pub age_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Report {
    pub latitude: f64,
    pub longitude: f64,
    pub latitude_deg_e7: i32,
    pub longitude_deg_e7: i32,
    pub altitude_amsl_m: Option<f64>,
    pub heading_deg: Option<f64>,
    pub velocity_north_m_s: Option<f64>,
    pub velocity_east_m_s: Option<f64>,
    pub velocity_down_m_s: Option<f64>,
    pub position_std_dev_horizontal_m: Option<f64>,
    pub position_std_dev_vertical_m: Option<f64>,
    pub estimation: u8,
}

impl Report {
    pub fn estimation_tokens(&self) -> Vec<&'static str> {
        [(ESTIMATION_POSITION, "position"), (ESTIMATION_VELOCITY, "velocity"), (ESTIMATION_HEADING, "heading")]
            .into_iter()
            .filter(|(bit, _)| self.estimation & bit != 0)
            .map(|(_, token)| token)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Target {
    pub id: i64,
    pub flight_mode: String,
    pub follow_flight_mode: String,
    pub apm: bool,
    pub home_altitude_amsl_m: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    FollowTarget,
    GlobalPositionInt,
}

impl Stream {
    pub fn token(self) -> &'static str {
        match self {
            Stream::FollowTarget => "followTarget",
            Stream::GlobalPositionInt => "globalPositionInt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    ModeUnknown,
    ModeNever,
    FollowModeUnsupported,
    NotInFollowMode,
    HomePositionUnknown,
}

impl Refusal {
    pub fn token(self) -> &'static str {
        match self {
            Refusal::ModeUnknown => "modeUnknown",
            Refusal::ModeNever => "modeNever",
            Refusal::FollowModeUnsupported => "followModeUnsupported",
            Refusal::NotInFollowMode => "notInFollowMode",
            Refusal::HomePositionUnknown => "homePositionUnknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    ModeUnknown,
    ModeNever,
    NoVehicles,
    NoVehicleInFollowMode,
    NoFix,
    FixInvalid,
    FixStale,
    FixUnusable,
    AllVehiclesRefused,
}

impl Reason {
    pub fn token(self) -> &'static str {
        match self {
            Reason::ModeUnknown => "modeUnknown",
            Reason::ModeNever => "modeNever",
            Reason::NoVehicles => "noVehicles",
            Reason::NoVehicleInFollowMode => "noVehicleInFollowMode",
            Reason::NoFix => "noFix",
            Reason::FixInvalid => "fixInvalid",
            Reason::FixStale => "fixStale",
            Reason::FixUnusable => "fixUnusable",
            Reason::AllVehiclesRefused => "allVehiclesRefused",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalPosition {
    pub latitude_deg_e7: i32,
    pub longitude_deg_e7: i32,
    pub altitude_amsl_mm: i32,
    pub relative_altitude_mm: i32,
    pub velocity_north_cm_s: i16,
    pub velocity_east_cm_s: i16,
    pub velocity_down_cm_s: i16,
    pub heading_cdeg: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FollowTargetWire {
    pub latitude_deg_e7: i32,
    pub longitude_deg_e7: i32,
    pub altitude_amsl_m: f32,
    pub velocity_m_s: [f32; 3],
    pub position_cov: [f32; 3],
    pub estimation: u8,
}

pub fn wrap_longitude(degrees: f64) -> f64 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

pub fn wrap_heading(degrees: f64) -> f64 {
    degrees.rem_euclid(360.0)
}

fn finite(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite())
}

pub fn motion_report(fix: &Fix) -> Option<Report> {
    if !fix.valid || fix.age_ms > ALLOWED_FIX_AGE_MS {
        return None;
    }
    let latitude = finite(Some(fix.latitude)).filter(|v| (-90.0..=90.0).contains(v))?;
    let longitude = wrap_longitude(finite(Some(fix.longitude))?);
    let heading = finite(fix.heading_deg).map(wrap_heading);
    let speed = finite(fix.ground_speed_m_s);
    let velocity = heading.zip(speed).map(|(heading, speed)| (heading.to_radians().cos() * speed, heading.to_radians().sin() * speed));
    let estimation = ESTIMATION_POSITION | heading.map_or(0, |_| ESTIMATION_HEADING) | velocity.map_or(0, |_| ESTIMATION_VELOCITY);
    Some(Report {
        latitude,
        longitude,
        latitude_deg_e7: (latitude * 1.0e7) as i32,
        longitude_deg_e7: (longitude * 1.0e7) as i32,
        altitude_amsl_m: finite(fix.altitude_amsl_m),
        heading_deg: heading,
        velocity_north_m_s: velocity.map(|(north, _)| north),
        velocity_east_m_s: velocity.map(|(_, east)| east),
        velocity_down_m_s: finite(fix.vertical_speed_down_m_s),
        position_std_dev_horizontal_m: finite(fix.horizontal_accuracy_m),
        position_std_dev_vertical_m: finite(fix.vertical_accuracy_m),
        estimation,
    })
}

pub fn in_follow_mode(target: &Target) -> bool {
    !target.follow_flight_mode.is_empty() && target.flight_mode == target.follow_flight_mode
}

pub fn receives(mode: Mode, target: &Target) -> bool {
    match mode {
        Mode::Never => false,
        Mode::Always => true,
        Mode::FollowMe => in_follow_mode(target),
    }
}

pub fn enabled(mode: Option<Mode>, fleet: &[Target]) -> bool {
    mode.is_some_and(|mode| fleet.iter().any(|target| receives(mode, target)))
}

pub fn stream(mode: Option<Mode>, target: &Target) -> Result<Stream, Refusal> {
    match mode {
        None => Err(Refusal::ModeUnknown),
        Some(Mode::Never) => Err(Refusal::ModeNever),
        Some(Mode::FollowMe) if target.follow_flight_mode.is_empty() => Err(Refusal::FollowModeUnsupported),
        Some(Mode::FollowMe) if !in_follow_mode(target) => Err(Refusal::NotInFollowMode),
        Some(_) => match (target.apm, finite(target.home_altitude_amsl_m)) {
            (false, _) => Ok(Stream::FollowTarget),
            (true, Some(_)) => Ok(Stream::GlobalPositionInt),
            (true, None) => Err(Refusal::HomePositionUnknown),
        },
    }
}

fn centimetres(value: Option<f64>) -> i16 {
    (value.unwrap_or(0.0) * 100.0).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16
}

fn metres(value: Option<f64>) -> f32 {
    value.map_or(f32::NAN, |v| v as f32)
}

pub fn global_position(report: &Report, home_altitude_amsl_m: f64) -> GlobalPosition {
    GlobalPosition {
        latitude_deg_e7: report.latitude_deg_e7,
        longitude_deg_e7: report.longitude_deg_e7,
        altitude_amsl_mm: (home_altitude_amsl_m * 1000.0).round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
        relative_altitude_mm: 0,
        velocity_north_cm_s: centimetres(report.velocity_north_m_s),
        velocity_east_cm_s: centimetres(report.velocity_east_m_s),
        velocity_down_cm_s: centimetres(report.velocity_down_m_s),
        heading_cdeg: report.heading_deg.map_or(HEADING_UNKNOWN_CDEG, |heading| ((wrap_heading(heading) * 100.0).round() as u16) % 36000),
    }
}

pub fn follow_target(report: &Report) -> FollowTargetWire {
    FollowTargetWire {
        latitude_deg_e7: report.latitude_deg_e7,
        longitude_deg_e7: report.longitude_deg_e7,
        altitude_amsl_m: metres(report.altitude_amsl_m),
        velocity_m_s: [metres(report.velocity_north_m_s), metres(report.velocity_east_m_s), metres(report.velocity_down_m_s)],
        position_cov: [metres(report.position_std_dev_horizontal_m), metres(report.position_std_dev_horizontal_m), metres(report.position_std_dev_vertical_m)],
        estimation: report.estimation,
    }
}

fn report_json(report: &Report) -> Value {
    json!({
        "latitude": report.latitude,
        "longitude": report.longitude,
        "latitudeDegE7": report.latitude_deg_e7,
        "longitudeDegE7": report.longitude_deg_e7,
        "altitudeAmsl": report.altitude_amsl_m,
        "heading": report.heading_deg,
        "velocityNorth": report.velocity_north_m_s,
        "velocityEast": report.velocity_east_m_s,
        "velocityDown": report.velocity_down_m_s,
        "positionStdDevHorizontal": report.position_std_dev_horizontal_m,
        "positionStdDevVertical": report.position_std_dev_vertical_m,
        "estimation": report.estimation_tokens(),
    })
}

fn sent_altitude(stream: Option<Stream>, target: &Target, report: Option<&Report>) -> Option<f64> {
    let report = report?;
    match stream? {
        Stream::FollowTarget => report.altitude_amsl_m,
        Stream::GlobalPositionInt => finite(target.home_altitude_amsl_m),
    }
}

fn velocity_zeroed(stream: Option<Stream>, report: Option<&Report>) -> Option<bool> {
    let report = report?;
    Some(stream? == Stream::GlobalPositionInt && report.velocity_north_m_s.is_none())
}

fn reason(mode: Option<Mode>, fleet: &[Target], fix: Option<&Fix>, report: Option<&Report>, plans: &[Result<Stream, Refusal>]) -> Option<Reason> {
    let mode = match mode {
        None => return Some(Reason::ModeUnknown),
        Some(Mode::Never) => return Some(Reason::ModeNever),
        Some(mode) => mode,
    };
    if fleet.is_empty() {
        return Some(Reason::NoVehicles);
    }
    if !fleet.iter().any(|target| receives(mode, target)) {
        return Some(Reason::NoVehicleInFollowMode);
    }
    match fix {
        None => Some(Reason::NoFix),
        Some(fix) if !fix.valid => Some(Reason::FixInvalid),
        Some(fix) if fix.age_ms > ALLOWED_FIX_AGE_MS => Some(Reason::FixStale),
        Some(_) if report.is_none() => Some(Reason::FixUnusable),
        _ => plans.iter().all(Result::is_err).then_some(Reason::AllVehiclesRefused),
    }
}

pub fn snapshot(setting: Option<f64>, fleet: &[Target], fix: Option<&Fix>) -> Value {
    let mode = Mode::from_setting(setting);
    let report = fix.and_then(motion_report);
    let plans: Vec<Result<Stream, Refusal>> = fleet.iter().map(|target| stream(mode, target)).collect();
    let refused = reason(mode, fleet, fix, report.as_ref(), &plans);
    let listed: Vec<Value> = fleet
        .iter()
        .zip(&plans)
        .map(|(target, plan)| {
            let stream = plan.ok();
            json!({
                "id": target.id,
                "following": plan.is_ok(),
                "flightMode": target.flight_mode,
                "followFlightMode": target.follow_flight_mode,
                "stream": stream.map(Stream::token),
                "refusal": plan.err().map(Refusal::token),
                "homeAltitudeAmsl": target.home_altitude_amsl_m,
                "sentAltitudeAmsl": sent_altitude(stream, target, report.as_ref()),
                "sentVelocityZeroed": velocity_zeroed(stream, report.as_ref()),
            })
        })
        .collect();
    json!({
        "kind": "object",
        "class": "FollowMe",
        "mode": mode.map(Mode::token),
        "modeRaw": setting,
        "intervalMs": MOTION_INTERVAL_MS,
        "allowedFixAgeMs": ALLOWED_FIX_AGE_MS,
        "enabled": enabled(mode, fleet),
        "wouldSend": refused.is_none(),
        "reason": refused.map(Reason::token),
        "fixValid": fix.map(|fix| fix.valid),
        "fixAgeMs": fix.map(|fix| fix.age_ms),
        "fixFresh": fix.map(|fix| fix.age_ms <= ALLOWED_FIX_AGE_MS),
        "report": report.as_ref().map(report_json),
        "count": listed.len(),
        "vehicles": listed,
        "units": { "altitude": "m", "velocity": "m/s", "heading": "deg", "accuracy": "m", "interval": "ms" },
    })
}

fn home_altitude(vehicle: &Value) -> Option<f64> {
    let home = vehicle.get("homePosition")?;
    flag(home, "valid").then(|| finite(home.get("altitude").and_then(Value::as_f64))).flatten()
}

fn target_of(read: &Value) -> Option<Target> {
    let identified = read.get("kind").and_then(Value::as_str) == Some("object");
    Some(Target {
        id: integer(read, "id").filter(|_| identified)?,
        flight_mode: text(read, "flightMode"),
        follow_flight_mode: text(read, "followFlightMode"),
        apm: flag(read, "apmFirmware"),
        home_altitude_amsl_m: home_altitude(read),
    })
}

pub fn gcs_fix(backend: &dyn Backend, wall_ms: u64) -> Option<Fix> {
    let stamp = value_number(&backend.get(GCS_TIMESTAMP)).filter(|t| *t > 0.0)? as u64;
    let position = object(&backend.get(GCS_POSITION));
    let coordinate = position.get("value").filter(|v| v.is_object()).unwrap_or(&position).clone();
    let read = |key: &str| finite(coordinate.get(key).and_then(Value::as_f64));
    Some(Fix {
        valid: flag(&coordinate, "valid"),
        latitude: read("latitude").unwrap_or(f64::NAN),
        longitude: read("longitude").unwrap_or(f64::NAN),
        altitude_amsl_m: read("altitude"),
        heading_deg: value_number(&backend.get(GCS_HEADING)),
        ground_speed_m_s: None,
        vertical_speed_down_m_s: None,
        horizontal_accuracy_m: value_number(&backend.get(GCS_HORIZONTAL_ACCURACY)),
        vertical_accuracy_m: None,
        age_ms: wall_ms.saturating_sub(stamp),
    })
}

pub fn fleet_of(backend: &dyn Backend) -> Vec<Target> {
    let count = integer(&object(&backend.get("vehicles.vehicles.count")), "value").unwrap_or(0).max(0);
    (0..count).filter_map(|index| target_of(&object(&backend.get_fields(&format!("vehicles.vehicles.{index}"), FIELDS)))).collect()
}

pub fn follow_me_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let fix = gcs_fix(backend, crate::hub::now_us() / 1000);
    snapshot(value_number(&backend.get(SETTING)), &fleet_of(backend), fix.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix() -> Fix {
        Fix { valid: true, latitude: 47.5, longitude: 8.5, altitude_amsl_m: Some(400.0), heading_deg: Some(90.0), ground_speed_m_s: Some(4.0), ..Default::default() }
    }

    fn follower(mode: &str, apm: bool) -> Target {
        Target { id: 1, flight_mode: mode.to_string(), follow_flight_mode: "Follow Me".to_string(), apm, home_altitude_amsl_m: Some(500.0) }
    }

    const FOLLOW_ME: Option<f64> = Some(2.0);
    const ALWAYS: Option<f64> = Some(1.0);
    const NEVER: Option<f64> = Some(0.0);

    #[derive(Default)]
    struct Fake {
        setting: Value,
        fleet: Vec<Value>,
        count: Option<usize>,
        position: Value,
        heading: Value,
        accuracy: Value,
        stamp: Value,
    }

    fn wall_ms() -> u64 {
        crate::hub::now_us() / 1000
    }

    fn placed(setting: Value, fleet: Vec<Value>, age_ms: u64) -> Fake {
        Fake {
            setting,
            fleet,
            position: json!({ "valid": true, "latitude": 47.5, "longitude": 8.5, "altitude": 400.0 }),
            heading: json!(90.0),
            accuracy: json!(2.5),
            stamp: json!(wall_ms().saturating_sub(age_ms)),
            ..Default::default()
        }
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            let value = match path {
                SETTING => self.setting.clone(),
                "vehicles.vehicles.count" => json!(self.count.unwrap_or(self.fleet.len())),
                GCS_POSITION => self.position.clone(),
                GCS_HEADING => self.heading.clone(),
                GCS_HORIZONTAL_ACCURACY => self.accuracy.clone(),
                GCS_TIMESTAMP => self.stamp.clone(),
                _ => Value::Null,
            };
            json!({ "kind": "value", "value": value }).to_string()
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            let index = path.strip_prefix("vehicles.vehicles.").and_then(|i| i.parse::<usize>().ok());
            index.and_then(|i| self.fleet.get(i)).map(Value::to_string).unwrap_or_else(|| json!({ "kind": "null" }).to_string())
        }
        fn set(&self, _path: &str, _value: &str) -> String { String::new() }
        fn invoke(&self, _path: &str, _args: &str) -> String { String::new() }
        fn watch(&self, _paths: &[String]) {}
    }

    fn px4(id: i64, mode: &str) -> Value {
        json!({ "kind": "object", "id": id, "flightMode": mode, "followFlightMode": "Follow Me", "apmFirmware": false })
    }

    fn apm(id: i64, mode: &str, home: bool) -> Value {
        json!({ "kind": "object", "id": id, "flightMode": mode, "followFlightMode": "Follow Me", "apmFirmware": true, "homePosition": { "valid": home, "altitude": 500.0 } })
    }

    #[test]
    fn an_unknown_follow_setting_never_starts_the_stream() {
        assert_eq!(Mode::from_setting(Some(0.0)), Some(Mode::Never));
        assert_eq!(Mode::from_setting(Some(1.0)), Some(Mode::Always));
        assert_eq!(Mode::from_setting(Some(2.0)), Some(Mode::FollowMe));
        assert_eq!(Mode::from_setting(None), None, "a setting the core could not read is not a setting of never mind always, so nothing is sent");
        assert_eq!(Mode::from_setting(Some(7.0)), None, "a value outside the three modes decodes to no mode rather than to the one that streams");
        assert!(!enabled(None, &[follower("Follow Me", false)]), "an unknown mode withholds the stream");
        assert!(!enabled(Some(Mode::Never), &[follower("Follow Me", false)]));
        assert!(enabled(Some(Mode::Always), &[follower("Hold", false)]), "always means always, whatever the vehicle is doing");
        assert!(!enabled(Some(Mode::Always), &[]), "with no vehicle there is nobody to send to");
    }

    #[test]
    fn a_corrupt_setting_reads_differently_from_a_missing_one() {
        let fleet = vec![px4(1, "Follow Me")];
        let corrupt = follow_me_view(&placed(json!(7), fleet.clone(), 0), &[]);
        let missing = follow_me_view(&placed(Value::Null, fleet, 0), &[]);
        assert_eq!((corrupt["mode"].as_str(), corrupt["reason"].as_str()), (None, Some("modeUnknown")));
        assert_eq!(corrupt["modeRaw"], json!(7.0), "a setting outside the three modes is handed on raw, because a corrupt setting needs a reset and a missing one does not");
        assert_eq!(missing["modeRaw"], Value::Null, "and a setting the core could not read at all has no raw value to show");
    }

    #[test]
    fn follow_me_mode_needs_a_vehicle_that_names_its_own_follow_mode() {
        assert!(receives(Mode::FollowMe, &follower("Follow Me", false)));
        assert!(!receives(Mode::FollowMe, &follower("Hold", false)));
        let nameless = Target { follow_flight_mode: String::new(), flight_mode: String::new(), ..follower("", false) };
        assert!(!in_follow_mode(&nameless), "a vehicle that reported no flight mode and no follow mode matches itself by string equality, and that empty match must not grant the stream");
        assert!(enabled(Some(Mode::FollowMe), &[follower("Hold", false), follower("Follow Me", false)]), "one vehicle in follow mode is enough to run the timer");
        assert!(!enabled(Some(Mode::FollowMe), &[follower("Hold", false)]), "and when it leaves follow mode the timer stops, because the decision is recomputed and never latched");
    }

    #[test]
    fn the_report_wraps_its_angles_and_keeps_absence_absent() {
        let report = motion_report(&fix()).unwrap();
        assert_eq!((report.latitude_deg_e7, report.longitude_deg_e7), (475000000, 85000000));
        assert_eq!((report.latitude, report.longitude), (47.5, 8.5), "the marker on the map is drawn from degrees, so the core publishes the degrees it already had rather than making every head divide by ten million");
        assert_eq!(report.estimation, 19, "the MAVLink est_capabilities bits are position, velocity and heading at 1, 2 and 16");
        assert_eq!(report.estimation_tokens(), vec!["position", "velocity", "heading"]);
        assert!(report.velocity_north_m_s.unwrap().abs() < 1.0e-9 && (report.velocity_east_m_s.unwrap() - 4.0).abs() < 1.0e-9, "a heading of ninety degrees puts the whole ground speed on the east axis");
        assert_eq!(report.position_std_dev_horizontal_m, None, "an accuracy the position source never reported is unknown, not perfect");
        assert_eq!(report.velocity_down_m_s, None, "and a vertical speed that was never reported is unknown, not still");
        let wrapped = motion_report(&Fix { heading_deg: Some(370.0), longitude: 190.0, ..fix() }).unwrap();
        assert_eq!(wrapped.heading_deg, Some(10.0), "a bearing wraps at three hundred and sixty");
        assert_eq!(wrapped.longitude_deg_e7, -1700000000, "longitude wraps too, and the antimeridian is where a follow target crosses it");
        assert_eq!(wrapped.longitude, -170.0, "the degrees a head draws wrap with the wire integer, never one of each");
        let still = motion_report(&Fix { heading_deg: None, ..fix() }).unwrap();
        assert_eq!((still.velocity_north_m_s, still.velocity_east_m_s), (None, None), "a ground speed without a direction is no velocity at all");
        assert_eq!(still.estimation & ESTIMATION_VELOCITY, 0, "and the estimation bits must not claim a velocity the report does not carry");
        assert_eq!(motion_report(&Fix { latitude: 91.0, ..fix() }), None, "a latitude off the globe is not a position");
        assert_eq!(motion_report(&Fix { valid: false, ..fix() }), None);
    }

    #[test]
    fn a_stale_fix_stops_the_report_and_a_fresh_one_starts_it_again() {
        assert_eq!(ALLOWED_FIX_AGE_MS, 5000, "the age gate is the remote ID gate, because both age the same GCS fix");
        assert_eq!(MOTION_INTERVAL_MS, 250, "the motion rate is FollowMe.h's fixed quarter second, never the interval the position device claims");
        assert!(motion_report(&Fix { age_ms: 5000, ..fix() }).is_some());
        assert_eq!(motion_report(&Fix { age_ms: 5001, ..fix() }), None, "a position this old is a ghost the vehicle would still chase");
        assert!(motion_report(&Fix { age_ms: 0, ..fix() }).is_some(), "the age guard clears the moment the device fixes again, so it is a gate and not a latch");
        let view = snapshot(ALWAYS, &[follower("Hold", false)], Some(&Fix { age_ms: 5001, ..fix() }));
        assert_eq!((view["enabled"].as_bool(), view["wouldSend"].as_bool(), view["reason"].as_str()), (Some(true), Some(false), Some("fixStale")), "the timer keeps running while the fix is stale, it just sends nothing, and it says why");
        assert_eq!((&view["report"], view["fixFresh"].as_bool()), (&Value::Null, Some(false)));
        let never_fixed = snapshot(ALWAYS, &[follower("Hold", false)], None);
        assert_eq!((&never_fixed["fixAgeMs"], &never_fixed["fixValid"]), (&Value::Null, &Value::Null), "a fix the position manager never produced has no age and no validity, rather than an age of zero");
    }

    #[test]
    fn every_dark_panel_names_the_one_thing_the_operator_must_fix() {
        let following = vec![follower("Follow Me", false)];
        let held = vec![follower("Hold", false)];
        let reason = |setting, fleet: &[Target], fix: Option<&Fix>| snapshot(setting, fleet, fix)["reason"].as_str().map(str::to_string);
        assert_eq!(reason(None, &following, Some(&fix())).as_deref(), Some("modeUnknown"));
        assert_eq!(reason(NEVER, &following, Some(&fix())).as_deref(), Some("modeNever"));
        assert_eq!(reason(ALWAYS, &[], Some(&fix())).as_deref(), Some("noVehicles"));
        assert_eq!(reason(FOLLOW_ME, &held, Some(&fix())).as_deref(), Some("noVehicleInFollowMode"));
        assert_eq!(reason(ALWAYS, &held, None).as_deref(), Some("noFix"));
        assert_eq!(reason(ALWAYS, &held, Some(&Fix { valid: false, ..fix() })).as_deref(), Some("fixInvalid"));
        assert_eq!(reason(ALWAYS, &held, Some(&Fix { age_ms: ALLOWED_FIX_AGE_MS + 1, ..fix() })).as_deref(), Some("fixStale"));
        assert_eq!(reason(ALWAYS, &held, Some(&Fix { latitude: 91.0, ..fix() })).as_deref(), Some("fixUnusable"), "a fix that is valid and fresh but off the globe was invisible before, and looked exactly like a core bug");
        let homeless = vec![Target { home_altitude_amsl_m: None, ..follower("Follow Me", true) }];
        assert_eq!(reason(FOLLOW_ME, &homeless, Some(&fix())).as_deref(), Some("allVehiclesRefused"));
        assert_eq!(reason(FOLLOW_ME, &following, Some(&fix())), None, "the reason is null exactly when the report would go out, so a head never has to rank the causes itself");
        assert_eq!(reason(FOLLOW_ME, &[follower("Hold", false), follower("Follow Me", false)], Some(&Fix { age_ms: ALLOWED_FIX_AGE_MS + 1, ..fix() })).as_deref(), Some("fixStale"), "with a vehicle asking to be followed the stale fix is the cause, and the core states that order rather than leaving heads to disagree on it");
    }

    #[test]
    fn the_snapshot_never_claims_to_have_sent_anything() {
        let view = snapshot(FOLLOW_ME, &[follower("Follow Me", false)], Some(&fix()));
        assert_eq!(view["wouldSend"], json!(true), "nothing in this module touches a link, so the field says what would be sent");
        assert!(view.get("sending").is_none(), "a field called sending would assert an action the core never performs");
        assert!(view.get("headingUnknownCdeg").is_none() && view.get("estimationBits").is_none(), "the protocol's sentinels and raw masks are the send path's business, not a panel's");
        assert!(view["vehicles"][0].get("globalPosition").is_none(), "one position in two representations makes a head choose which to trust");
        let registered = crate::view::lookup("view.followMe").expect("the view is registered, or the port is inert");
        assert_eq!(registered.deps, DEPS);
    }

    #[test]
    fn a_vehicle_that_is_not_followed_says_why_in_its_own_row() {
        let row = |setting, target: Target| snapshot(setting, &[target], Some(&fix()))["vehicles"][0].clone();
        assert_eq!(row(None, follower("Follow Me", false))["refusal"], json!("modeUnknown"));
        assert_eq!(row(NEVER, follower("Follow Me", false))["refusal"], json!("modeNever"));
        assert_eq!(row(FOLLOW_ME, follower("Hold", false))["refusal"], json!("notInFollowMode"), "switching this vehicle to its follow mode takes ten seconds, and the row says so");
        let unsupported = Target { follow_flight_mode: String::new(), ..follower("Hold", false) };
        assert_eq!(row(FOLLOW_ME, unsupported)["refusal"], json!("followModeUnsupported"), "a firmware that names no follow mode can never be followed, and hunting its mode menu is wasted time");
        let homeless = Target { home_altitude_amsl_m: None, ..follower("Follow Me", true) };
        assert_eq!(row(FOLLOW_ME, homeless.clone())["refusal"], json!("homePositionUnknown"));
        assert_eq!(row(FOLLOW_ME, homeless)["following"], json!(false), "a refused vehicle is not being followed");
        assert_eq!(row(FOLLOW_ME, follower("Follow Me", false))["refusal"], Value::Null);
    }

    #[test]
    fn each_vehicle_is_told_the_altitude_its_firmware_actually_carries() {
        let report = motion_report(&fix()).unwrap();
        assert_eq!(stream(Mode::from_setting(FOLLOW_ME), &follower("Follow Me", false)), Ok(Stream::FollowTarget));
        assert_eq!(stream(Mode::from_setting(FOLLOW_ME), &follower("Follow Me", true)), Ok(Stream::GlobalPositionInt));
        let fleet = vec![follower("Follow Me", false), Target { id: 2, ..follower("Follow Me", true) }];
        let view = snapshot(FOLLOW_ME, &fleet, Some(&fix()));
        assert_eq!(view["vehicles"][0]["sentAltitudeAmsl"], json!(400.0), "the follow target message carries the operator's own altitude");
        assert_eq!(view["vehicles"][1]["sentAltitudeAmsl"], json!(500.0), "and ArduPilot's global position carries the vehicle's home altitude, which is the height the drone holds over the operator");
        assert_eq!(view["vehicles"][1]["stream"], json!("globalPositionInt"));
        assert_eq!(global_position(&report, 500.0).altitude_amsl_mm, 500_000, "the metres a head shows and the millimetres on the wire are the same altitude");
        let px4_only = snapshot(FOLLOW_ME, &[follower("Follow Me", false)], Some(&Fix { altitude_amsl_m: None, ..fix() }));
        assert_eq!(px4_only["vehicles"][0]["sentAltitudeAmsl"], Value::Null, "an altitude the fix never carried is not an altitude the vehicle is told");
    }

    #[test]
    fn ardupilot_needs_a_home_altitude_and_the_wire_never_invents_one() {
        let apm_follower = Target { home_altitude_amsl_m: None, ..follower("Follow Me", true) };
        assert_eq!(stream(Some(Mode::FollowMe), &apm_follower), Err(Refusal::HomePositionUnknown), "the global position carries the home altitude, so without one there is nothing honest to send");
        assert_eq!(stream(Some(Mode::FollowMe), &Target { home_altitude_amsl_m: None, ..follower("Follow Me", false) }), Ok(Stream::FollowTarget), "and the follow target message never needed a home position");
        let wire = global_position(&motion_report(&fix()).unwrap(), 500.0);
        assert_eq!((wire.altitude_amsl_mm, wire.relative_altitude_mm), (500_000, 0));
        assert_eq!((wire.velocity_north_cm_s, wire.velocity_east_cm_s), (0, 400));
        assert_eq!(wire.heading_cdeg, 9000);
        assert_eq!(global_position(&motion_report(&Fix { heading_deg: Some(-90.0), ..fix() }).unwrap(), 0.0).heading_cdeg, 27000, "a negative bearing wraps before it is scaled, because the wire field is unsigned");
    }

    #[test]
    fn an_unknown_velocity_and_heading_reach_the_wire_as_unknown_or_as_a_stated_zero() {
        let blind = motion_report(&Fix { heading_deg: None, ..fix() }).unwrap();
        let wire = global_position(&blind, 500.0);
        assert_eq!(wire.heading_cdeg, HEADING_UNKNOWN_CDEG, "an unknown heading goes out as MAVLink's sentinel, never as zero centidegrees, which reads as due north");
        assert_eq!(wire.velocity_north_cm_s, 0, "GLOBAL_POSITION_INT has no unknown velocity, so a zero is all the wire can carry");
        let view = snapshot(FOLLOW_ME, &[follower("Follow Me", true), Target { id: 2, ..follower("Follow Me", false) }], Some(&Fix { heading_deg: None, ..fix() }));
        assert_eq!(view["vehicles"][0]["sentVelocityZeroed"], json!(true), "so the snapshot says the ArduPilot vehicle is being told the operator is standing still, which is what makes it mispredict the lead");
        assert_eq!(view["vehicles"][1]["sentVelocityZeroed"], json!(false), "the follow target message says unknown in its estimation bits instead, so nothing is faked there");
        let follow = follow_target(&blind);
        assert!(follow.velocity_m_s.iter().all(|v| v.is_nan()), "an unknown velocity is not a stationary one, and the C++ ships a zero-initialised struct");
        assert!(follow.position_cov.iter().all(|c| c.is_nan()), "and an unknown covariance is MAVLink's NaN, not the zero that claims a perfect fix");
        assert_eq!(follow.estimation & ESTIMATION_VELOCITY, 0);
        let known = follow_target(&motion_report(&Fix { horizontal_accuracy_m: Some(2.5), vertical_accuracy_m: Some(4.0), ..fix() }).unwrap());
        assert_eq!(known.position_cov, [2.5, 2.5, 4.0], "a horizontal accuracy covers both horizontal axes, as FollowMe.cc fills them");
    }

    #[test]
    fn a_vehicle_read_that_failed_is_not_a_vehicle_to_follow() {
        let raced = Fake { count: Some(3), ..placed(json!(1), vec![px4(1, "Hold")], 0) };
        let view = follow_me_view(&raced, &[]);
        assert_eq!(view["count"], json!(1), "a count that outran the reads must not list a vehicle nothing could read");
        assert_eq!(view["vehicles"][0]["id"], json!(1));
        let phantom = Fake { count: Some(2), ..placed(json!(1), vec![], 0) };
        let dark = follow_me_view(&phantom, &[]);
        assert_eq!((dark["count"].as_u64(), dark["wouldSend"].as_bool(), dark["reason"].as_str()), (Some(0), Some(false), Some("noVehicles")), "an absent read must not decode to a vehicle that is granted the stream under always");
        assert_eq!(dark["enabled"], json!(false));
        assert_eq!(target_of(&json!({ "kind": "null" })), None);
        assert_eq!(target_of(&json!({ "kind": "object", "flightMode": "Hold" })), None, "a vehicle with no id is nothing a head could send to");
        assert_eq!(target_of(&json!({ "kind": "value", "value": null, "id": 5 })), None, "and only an object read is a vehicle, which is the kind track.rs checks before it trusts one");
    }

    #[test]
    fn the_view_reads_the_fix_from_the_position_manager_and_ages_it_from_the_manager_s_own_clock() {
        assert!(DEPS.contains(&GCS_POSITION) && DEPS.contains(&GCS_TIMESTAMP) && DEPS.contains(&GCS_HEADING), "a head watching the view is woken by the fix itself, or its panel freezes at whatever the last unrelated change left behind");
        let backend = placed(json!(2), vec![apm(1, "Follow Me", true), px4(2, "Hold")], 120);
        let view = follow_me_view(&backend, &[]);
        assert_eq!(view["class"], "FollowMe");
        assert_eq!((view["mode"].as_str(), view["enabled"].as_bool(), view["wouldSend"].as_bool()), (Some("followMe"), Some(true), Some(true)));
        assert_eq!((view["fixValid"].as_bool(), view["fixFresh"].as_bool()), (Some(true), Some(true)));
        assert!((120..3000).contains(&view["fixAgeMs"].as_u64().unwrap()), "the age is the delta to the position manager's own timestamp, not the time since some head last pushed a cached position; got {}", view["fixAgeMs"]);
        assert_eq!(view["report"]["latitudeDegE7"], json!(475000000));
        assert_eq!((view["report"]["heading"].as_f64(), view["report"]["positionStdDevHorizontal"].as_f64()), (Some(90.0), Some(2.5)), "heading and accuracy come from the manager's own facts");
        assert_eq!(view["units"]["altitude"], "m", "the head formats the numbers, the core only names their unit");
        assert_eq!((view["vehicles"][0]["following"].as_bool(), view["vehicles"][0]["stream"].as_str()), (Some(true), Some("globalPositionInt")));
        assert_eq!((view["vehicles"][1]["following"].as_bool(), view["vehicles"][1]["refusal"].as_str()), (Some(false), Some("notInFollowMode")));
        let stale = follow_me_view(&placed(json!(2), vec![px4(1, "Follow Me")], ALLOWED_FIX_AGE_MS + 2000), &[]);
        assert_eq!((stale["reason"].as_str(), &stale["report"]), (Some("fixStale"), &Value::Null), "a GPS that dies mid follow ages out on its own clock, so the panel stops reading fresh without anything else happening");
        let dark = follow_me_view(&Fake { setting: json!(2), fleet: vec![px4(1, "Follow Me")], ..Default::default() }, &[]);
        assert_eq!((dark["reason"].as_str(), &dark["fixAgeMs"]), (Some("noFix"), &Value::Null), "a position manager with no fix yet has no age at all");
        assert_eq!(gcs_fix(&placed(json!(1), vec![], 0), wall_ms()).unwrap().ground_speed_m_s, None, "the manager publishes no ground speed today, and unknown stays unknown rather than becoming a standstill");
    }
}
