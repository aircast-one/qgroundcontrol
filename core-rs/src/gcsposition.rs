use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;

pub const MIN_HORIZONTAL_ACCURACY_M: f64 = 100.0;
pub const MIN_VERTICAL_ACCURACY_M: f64 = 10.0;
pub const MIN_DIRECTION_ACCURACY_DEG: f64 = 30.0;
pub const NULL_ISLAND_DEG: f64 = 0.001;
pub const STALE_AFTER_MS: u64 = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct MonotonicMs(u64);

pub fn now() -> MonotonicMs {
    MonotonicMs(crate::hub::now_ms())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Source {
    #[default]
    None,
    Simulated,
    InternalGps,
    Plugin,
    Nmea,
    Log,
    External,
}

pub const SOURCES: &[Source] = &[Source::None, Source::Simulated, Source::InternalGps, Source::Plugin, Source::Nmea, Source::Log, Source::External];

impl Source {
    pub fn token(self) -> &'static str {
        match self {
            Source::None => "none",
            Source::Simulated => "simulated",
            Source::InternalGps => "internalGps",
            Source::Plugin => "plugin",
            Source::Nmea => "nmea",
            Source::Log => "log",
            Source::External => "external",
        }
    }

    pub fn listening(self) -> bool {
        matches!(self, Source::Simulated | Source::InternalGps | Source::Plugin | Source::Nmea)
    }

    pub fn trusts_heading_without_accuracy(self) -> bool {
        self == Source::Plugin
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Access,
    Closed,
    UnknownSource,
    UpdateTimeout,
}

pub const ERROR_NONE: i64 = 3;

impl Error {
    pub fn token(self) -> &'static str {
        match self {
            Error::Access => "accessError",
            Error::Closed => "closedError",
            Error::UnknownSource => "unknownSourceError",
            Error::UpdateTimeout => "updateTimeoutError",
        }
    }

    pub fn from_code(code: i64) -> Option<Error> {
        match code {
            0 => Some(Error::Access),
            1 => Some(Error::Closed),
            2 => Some(Error::UnknownSource),
            4 => Some(Error::UpdateTimeout),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    NoCoordinate,
    LatitudeOutOfRange,
    NullIsland,
    AccuracyUnknown,
    AccuracyTooCoarse,
    VerticalAccuracyTooCoarse,
}

impl Refusal {
    pub fn token(self) -> &'static str {
        match self {
            Refusal::NoCoordinate => "noCoordinate",
            Refusal::LatitudeOutOfRange => "latitudeOutOfRange",
            Refusal::NullIsland => "nullIsland",
            Refusal::AccuracyUnknown => "accuracyUnknown",
            Refusal::AccuracyTooCoarse => "accuracyTooCoarse",
            Refusal::VerticalAccuracyTooCoarse => "verticalAccuracyTooCoarse",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Update {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub horizontal_accuracy_m: Option<f64>,
    pub vertical_accuracy_m: Option<f64>,
    pub direction_deg: Option<f64>,
    pub direction_accuracy_deg: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    Reported,
    Source(Source),
    Error(Option<Error>),
    HorizontalAccuracy(Option<f64>),
    Position { latitude: Option<f64>, longitude: Option<f64>, altitude: Option<f64> },
    Heading(Option<f64>),
}

pub fn present(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite())
}

pub fn wrap_heading(degrees: f64) -> f64 {
    degrees.rem_euclid(360.0)
}

pub fn wrap_longitude(degrees: f64) -> f64 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

fn age_of(stamped_ms: Option<u64>, now: MonotonicMs) -> Option<u64> {
    stamped_ms.filter(|stamped| *stamped <= now.0).map(|stamped| now.0 - stamped)
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GcsPosition {
    pub source: Source,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub heading_deg: Option<f64>,
    pub horizontal_accuracy_m: Option<f64>,
    pub vertical_accuracy_m: Option<f64>,
    pub direction_accuracy_deg: Option<f64>,
    pub stamped_ms: Option<u64>,
    pub last_report_ms: Option<u64>,
    pub refusal: Option<Refusal>,
    pub error: Option<Error>,
    stale_announced: bool,
}

fn diff(before: &GcsPosition, after: &GcsPosition) -> Vec<Out> {
    [
        (before.error != after.error).then_some(Out::Error(after.error)),
        (before.horizontal_accuracy_m != after.horizontal_accuracy_m).then_some(Out::HorizontalAccuracy(after.horizontal_accuracy_m)),
        (before.coordinate() != after.coordinate()).then_some(Out::Position { latitude: after.latitude, longitude: after.longitude, altitude: after.altitude }),
        (before.heading_deg != after.heading_deg).then_some(Out::Heading(after.heading_deg)),
    ]
    .into_iter()
    .flatten()
    .chain([Out::Reported])
    .collect()
}

impl GcsPosition {
    pub fn coordinate(&self) -> (Option<f64>, Option<f64>, Option<f64>) {
        (self.latitude, self.longitude, self.altitude)
    }

    pub fn fixed(&self) -> bool {
        self.latitude.is_some() && self.longitude.is_some()
    }

    pub fn age_ms(&self, now: MonotonicMs) -> Option<u64> {
        age_of(self.stamped_ms, now)
    }

    pub fn last_report_age_ms(&self, now: MonotonicMs) -> Option<u64> {
        age_of(self.last_report_ms, now)
    }

    pub fn stale(&self, now: MonotonicMs) -> bool {
        self.age_ms(now).is_none_or(|age| age > STALE_AFTER_MS)
    }

    pub fn usable(&self, now: MonotonicMs) -> bool {
        self.fixed() && !self.stale(now) && self.horizontal_accuracy_m.is_some_and(|accuracy| accuracy <= MIN_HORIZONTAL_ACCURACY_M)
    }

    pub fn fix(&self, now: MonotonicMs) -> &'static str {
        match (self.source.listening(), self.fixed(), self.stale(now), self.altitude.is_some()) {
            (false, false, ..) => "noSource",
            (_, false, ..) => "waiting",
            (_, _, true, true) => "staleThreeDimensional",
            (_, _, true, false) => "staleHorizontal",
            (_, _, false, true) => "threeDimensional",
            _ => "horizontal",
        }
    }

    fn fresh(&self, now: MonotonicMs, value: Option<f64>) -> Option<f64> {
        value.filter(|_| !self.stale(now))
    }

    pub fn went_stale(&mut self, now: MonotonicMs) -> bool {
        let first = self.stamped_ms.is_some() && self.stale(now) && !self.stale_announced;
        self.stale_announced = self.stale_announced || first;
        first
    }

    pub fn select_source(&mut self, source: Source) -> Vec<Out> {
        let before = *self;
        *self = GcsPosition { source, ..GcsPosition::default() };
        [
            Some(Out::Reported),
            (before.source != source).then_some(Out::Source(source)),
            (before.error != self.error).then_some(Out::Error(None)),
            (before.coordinate() != self.coordinate()).then_some(Out::Position { latitude: None, longitude: None, altitude: None }),
            (before.heading_deg != self.heading_deg).then_some(Out::Heading(None)),
            (before.horizontal_accuracy_m != self.horizontal_accuracy_m).then_some(Out::HorizontalAccuracy(None)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    pub fn on_error(&mut self, code: i64) -> Vec<Out> {
        let before = self.error;
        self.error = if code == ERROR_NONE { None } else { Error::from_code(code).or(before) };
        (before != self.error).then(|| vec![Out::Error(self.error), Out::Reported]).unwrap_or_default()
    }

    pub fn on_update(&mut self, update: Update, now: MonotonicMs) -> Vec<Out> {
        let before = *self;
        self.error = None;
        if !self.source.listening() {
            return (before.error != self.error).then(|| vec![Out::Error(None), Out::Reported]).unwrap_or_default();
        }
        self.last_report_ms = Some(now.0);

        let latitude = present(update.latitude);
        let longitude = present(update.longitude).map(wrap_longitude);
        let in_range = latitude.filter(|latitude| (-90.0..=90.0).contains(latitude));
        let coordinate = in_range.zip(longitude);
        let off_null_island = coordinate.is_some_and(|(latitude, longitude)| latitude.abs() > NULL_ISLAND_DEG && longitude.abs() > NULL_ISLAND_DEG);

        let reported_horizontal = present(update.horizontal_accuracy_m).filter(|_| off_null_island);
        self.horizontal_accuracy_m = reported_horizontal.or(self.horizontal_accuracy_m);
        let accepted = coordinate.filter(|_| reported_horizontal.is_some_and(|accuracy| accuracy <= MIN_HORIZONTAL_ACCURACY_M));
        (self.latitude, self.longitude, self.stamped_ms, self.stale_announced) = accepted
            .map(|(latitude, longitude)| (Some(latitude), Some(longitude), Some(now.0), false))
            .unwrap_or((self.latitude, self.longitude, self.stamped_ms, self.stale_announced));

        let reported_vertical = present(update.vertical_accuracy_m);
        self.vertical_accuracy_m = reported_vertical.or(self.vertical_accuracy_m);
        self.altitude = if reported_vertical.is_some_and(|accuracy| accuracy <= MIN_VERTICAL_ACCURACY_M) { present(update.altitude) } else { self.altitude };

        self.refusal = match (latitude, longitude, in_range, reported_horizontal) {
            (None, ..) | (_, None, ..) => Some(Refusal::NoCoordinate),
            (_, _, None, _) => Some(Refusal::LatitudeOutOfRange),
            _ if !off_null_island => Some(Refusal::NullIsland),
            (.., None) => Some(Refusal::AccuracyUnknown),
            (.., Some(accuracy)) if accuracy > MIN_HORIZONTAL_ACCURACY_M => Some(Refusal::AccuracyTooCoarse),
            _ => present(update.altitude).and(reported_vertical).filter(|accuracy| *accuracy > MIN_VERTICAL_ACCURACY_M).map(|_| Refusal::VerticalAccuracyTooCoarse),
        };

        let reported_direction = present(update.direction_accuracy_deg);
        self.direction_accuracy_deg = reported_direction.or(self.direction_accuracy_deg);
        let heading = present(update.direction_deg).map(wrap_heading);
        self.heading_deg = match reported_direction {
            Some(accuracy) if accuracy <= MIN_DIRECTION_ACCURACY_DEG => heading,
            None if self.source.trusts_heading_without_accuracy() => heading,
            _ => self.heading_deg,
        };

        diff(&before, self)
    }

    pub fn snapshot(&self, now: MonotonicMs) -> Value {
        json!({
            "kind": "object",
            "class": "GcsPosition",
            "source": self.source.token(),
            "listening": self.source.listening(),
            "sources": SOURCES.iter().map(|source| json!({ "source": source.token(), "listening": source.listening() })).collect::<Vec<Value>>(),
            "hasFix": self.fixed(),
            "usable": self.usable(now),
            "latitude": self.fresh(now, self.latitude),
            "longitude": self.fresh(now, self.longitude),
            "altitude": self.fresh(now, self.altitude),
            "heading": self.fresh(now, self.heading_deg),
            "lastKnownLatitude": self.latitude,
            "lastKnownLongitude": self.longitude,
            "lastKnownAltitude": self.altitude,
            "lastKnownHeading": self.heading_deg,
            "coordinateUnits": "deg",
            "altitudeUnits": "m",
            "headingUnits": "deg",
            "horizontalAccuracy": self.horizontal_accuracy_m,
            "verticalAccuracy": self.vertical_accuracy_m,
            "accuracyUnits": "m",
            "directionAccuracy": self.direction_accuracy_deg,
            "directionAccuracyUnits": "deg",
            "minimumHorizontalAccuracy": MIN_HORIZONTAL_ACCURACY_M,
            "minimumVerticalAccuracy": MIN_VERTICAL_ACCURACY_M,
            "minimumDirectionAccuracy": MIN_DIRECTION_ACCURACY_DEG,
            "ageMs": self.age_ms(now),
            "lastReportAgeMs": self.last_report_age_ms(now),
            "refused": self.refusal.map(Refusal::token),
            "staleAfterMs": STALE_AFTER_MS,
            "stale": self.stamped_ms.is_some() && self.stale(now),
            "fix": self.fix(now),
            "error": self.error.map(Error::token),
        })
    }
}

static POSITION: LazyLock<Mutex<GcsPosition>> = LazyLock::new(|| Mutex::new(GcsPosition::default()));

pub fn lock() -> MutexGuard<'static, GcsPosition> {
    POSITION.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn gcs_position_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    lock().snapshot(now())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64) -> MonotonicMs {
        MonotonicMs(ms)
    }

    fn fix(horizontal: Option<f64>) -> Update {
        Update { latitude: Some(47.397742), longitude: Some(8.545594), altitude: Some(488.0), horizontal_accuracy_m: horizontal, vertical_accuracy_m: Some(5.0), ..Update::default() }
    }

    fn listening(source: Source) -> GcsPosition {
        GcsPosition { source, ..GcsPosition::default() }
    }

    #[test]
    fn the_gates_are_the_numbers_the_position_manager_ships() {
        assert_eq!(
            (MIN_HORIZONTAL_ACCURACY_M, MIN_VERTICAL_ACCURACY_M, MIN_DIRECTION_ACCURACY_DEG, STALE_AFTER_MS),
            (100.0, 10.0, 30.0, 5000),
            "kMinHorizonalAccuracyMeters, kMinVerticalAccuracyMeters and kMinDirectionAccuracyDegrees are PositionManager.h:100-102, and the five second expiry is this module's own budget rather than remoteid's ALLOWED_GPS_DELAY_MS, so tuning one cannot retune the other"
        );
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gcsposition.rs")).unwrap();
        assert!(
            !source.contains(&["use crate::", "remoteid"].concat()),
            "the expiry is this module's own budget and the Remote ID transmission budget is remoteid's; borrowing the constant across the module boundary means anyone tuning the UI staleness retunes an ODID compliance number, with no test pinning either"
        );
        let mut coarse = listening(Source::InternalGps);
        coarse.on_update(Update { horizontal_accuracy_m: Some(100.1), ..fix(None) }, at(1_000));
        assert_eq!((coarse.latitude, coarse.refusal), (None, Some(Refusal::AccuracyTooCoarse)), "a hundred point one metre fix is refused by the hard number, not by whatever the constant happens to say");
        let mut exact = listening(Source::InternalGps);
        exact.on_update(Update { horizontal_accuracy_m: Some(100.0), ..fix(None) }, at(1_000));
        assert_eq!((exact.latitude, exact.refusal), (Some(47.397742), None), "and a hundred metre fix is accepted, because the comparison is less than or equal");
        assert_eq!((Error::from_code(0), Error::from_code(1), Error::from_code(2), Error::from_code(4)), (Some(Error::Access), Some(Error::Closed), Some(Error::UnknownSource), Some(Error::UpdateTimeout)));
        assert_eq!((ERROR_NONE, Error::from_code(ERROR_NONE)), (3, None), "QGeoPositionInfoSource::NoError is the third code and names no error");
    }

    #[test]
    fn a_coarse_fix_records_its_accuracy_and_keeps_the_last_trusted_coordinate() {
        let mut position = listening(Source::InternalGps);
        position.on_update(fix(Some(4.0)), at(1_000));
        assert_eq!((position.latitude, position.stamped_ms), (Some(47.397742), Some(1_000)));
        let outs = position.on_update(Update { latitude: Some(10.0), longitude: Some(10.0), horizontal_accuracy_m: Some(MIN_HORIZONTAL_ACCURACY_M + 0.1), ..Update::default() }, at(2_000));
        assert_eq!((position.latitude, position.stamped_ms), (Some(47.397742), Some(1_000)), "an update outside the hundred metre gate leaves the coordinate alone, so it must leave the arrival stamp alone too or the old fix reads fresh");
        assert_eq!(outs, vec![Out::HorizontalAccuracy(Some(MIN_HORIZONTAL_ACCURACY_M + 0.1)), Out::Reported], "the rejected accuracy is still what the receiver last reported");
        assert_eq!(
            (position.refusal, position.last_report_ms, position.last_report_age_ms(at(2_500))),
            (Some(Refusal::AccuracyTooCoarse), Some(2_000), Some(500)),
            "a receiver whose readings are all refused is still a receiver that is talking, and the reason it is refused is the only thing that tells the operator to walk outside"
        );
        position.on_update(fix(Some(MIN_HORIZONTAL_ACCURACY_M)), at(3_000));
        assert_eq!((position.stamped_ms, position.refusal), (Some(3_000), None), "the gate is inclusive, exactly as kMinHorizonalAccuracyMeters is compared, and an accepted coordinate withdraws the refusal");
    }

    #[test]
    fn an_update_carrying_no_accuracy_moves_nothing_and_says_why() {
        let mut position = listening(Source::InternalGps);
        let outs = position.on_update(Update { latitude: Some(47.0), longitude: Some(8.0), altitude: Some(400.0), ..Update::default() }, at(500));
        assert_eq!(position.coordinate(), (None, None, None), "a reading whose accuracy the receiver never stated is not a reading that passed a gate");
        assert_eq!((position.horizontal_accuracy_m, position.stamped_ms, position.age_ms(at(500))), (None, None, None), "an unknown accuracy is absent, never zero, and an unstamped fix has no age at all");
        assert_eq!(outs, vec![Out::Reported]);
        assert_eq!(position.fix(at(500)), "waiting", "a live receiver that has not yet delivered a fix is waiting, which is a different story from having no receiver at all");
        let view = position.snapshot(at(700));
        assert_eq!(
            (view["refused"].as_str(), view["lastReportAgeMs"].as_u64(), view["fix"].as_str()),
            (Some("accuracyUnknown"), Some(200), Some("waiting")),
            "a refused reading and no reading at all must not read the same, or the operator cannot choose between replugging the receiver and waiting for it"
        );
        let never = GcsPosition::default().snapshot(at(700));
        assert_eq!((never["refused"].as_str(), never["lastReportAgeMs"].as_u64(), never["fix"].as_str()), (None, None, Some("noSource")), "nothing has ever arrived, so there is no last report to age and nothing to explain");
    }

    #[test]
    fn altitude_needs_its_own_ten_metre_gate_and_is_absent_until_it_passes() {
        let mut position = listening(Source::InternalGps);
        position.on_update(Update { vertical_accuracy_m: Some(MIN_VERTICAL_ACCURACY_M + 0.1), ..fix(Some(4.0)) }, at(1_000));
        assert_eq!((position.altitude, position.vertical_accuracy_m), (None, Some(MIN_VERTICAL_ACCURACY_M + 0.1)), "the vertical gate is separate from the horizontal one, so a good lat/lon does not carry a bad altitude in with it");
        assert_eq!((position.fix(at(1_000)), position.refusal), ("horizontal", Some(Refusal::VerticalAccuracyTooCoarse)), "the coordinate landed and the altitude did not, and the altitude is the half that needs explaining");
        position.on_update(fix(Some(4.0)), at(1_500));
        assert_eq!((position.altitude, position.fix(at(1_500)), position.refusal), (Some(488.0), "threeDimensional", None));
        let mut alone = listening(Source::InternalGps);
        alone.on_update(Update { vertical_accuracy_m: None, ..fix(Some(4.0)) }, at(1_000));
        assert_eq!((alone.altitude, alone.vertical_accuracy_m), (None, None), "an accuracy the receiver never stated is unknown rather than zero, and an altitude it cannot vouch for does not land");
        assert!(alone.snapshot(at(1_000)).get("accuracy").is_none(), "_gcsPositionAccuracy has no getter and no reader anywhere upstream, so the hypot of a horizontal CEP and a vertical accuracy is not a number this port publishes");
    }

    #[test]
    fn heading_is_only_trusted_inside_its_own_accuracy_and_the_trust_follows_the_live_source() {
        let mut position = listening(Source::InternalGps);
        position.on_update(Update { direction_deg: Some(90.0), direction_accuracy_deg: Some(MIN_DIRECTION_ACCURACY_DEG), ..fix(Some(4.0)) }, at(1_000));
        assert_eq!(position.heading_deg, Some(90.0));
        position.on_update(Update { direction_deg: Some(270.0), direction_accuracy_deg: Some(MIN_DIRECTION_ACCURACY_DEG + 0.1), ..fix(Some(4.0)) }, at(1_100));
        assert_eq!(position.heading_deg, Some(90.0), "a bearing the receiver cannot vouch for leaves the last trusted one standing");
        position.on_update(Update { direction_deg: Some(270.0), ..fix(Some(4.0)) }, at(1_200));
        assert_eq!(position.heading_deg, Some(90.0), "an internal source that states no direction accuracy states no direction either");
        let mut plugin = listening(Source::Plugin);
        plugin.on_update(Update { direction_deg: Some(270.0), ..fix(Some(4.0)) }, at(1_000));
        assert_eq!(plugin.heading_deg, Some(270.0), "a plugin-supplied source reports a bearing without an accuracy attribute, so it is taken as given");
        plugin.select_source(Source::Nmea);
        plugin.on_update(Update { direction_deg: Some(10.0), ..fix(Some(4.0)) }, at(2_000));
        assert_eq!(plugin.heading_deg, None, "QGCPositionManager::_usingPluginSource never clears, so the plugin's unconditional heading trust survives a switch to NMEA; here the trust is the live source's, and switching withdraws it");
    }

    #[test]
    fn bearings_and_longitudes_wrap_before_anyone_reads_them() {
        assert_eq!((wrap_heading(370.0), wrap_heading(-90.0), wrap_heading(360.0)), (10.0, 270.0, 0.0));
        assert_eq!((wrap_longitude(185.0), wrap_longitude(-185.0), wrap_longitude(180.0)), (-175.0, 175.0, -180.0));
        let mut position = listening(Source::InternalGps);
        position.on_update(Update { longitude: Some(185.0), direction_deg: Some(730.0), direction_accuracy_deg: Some(1.0), ..fix(Some(4.0)) }, at(1_000));
        assert_eq!((position.longitude, position.heading_deg), (Some(-175.0), Some(10.0)), "a bearing past a full turn and a longitude past the antimeridian name real places, and every consumer expects them inside one turn");
        let mut polar = listening(Source::InternalGps);
        polar.on_update(Update { latitude: Some(91.0), ..fix(Some(4.0)) }, at(1_000));
        assert_eq!((polar.latitude, polar.refusal), (None, Some(Refusal::LatitudeOutOfRange)), "latitude does not wrap over the pole without taking the longitude with it, so an impossible one is refused rather than folded, and the refusal is named");
        let mut empty = listening(Source::InternalGps);
        empty.on_update(Update { latitude: None, longitude: None, ..fix(Some(4.0)) }, at(1_000));
        assert_eq!(empty.refusal, Some(Refusal::NoCoordinate), "an update with no coordinate in it is a receiver that is answering without having a position");
    }

    #[test]
    fn a_fix_on_the_null_island_axes_is_refused() {
        let mut position = listening(Source::InternalGps);
        let outs = position.on_update(Update { latitude: Some(0.0), longitude: Some(0.0), ..fix(Some(4.0)) }, at(1_000));
        assert_eq!((position.latitude, position.longitude, position.horizontal_accuracy_m, position.stamped_ms), (None, None, None, None), "an unset receiver reads out as zero, and the accuracy of a position nobody has is not recorded either");
        assert_eq!((position.fixed(), position.fix(at(1_000)), position.refusal), (false, "waiting", Some(Refusal::NullIsland)), "the vertical gate carries no null island guard, so an altitude still lands; on its own it is not a fix, and the reason it is not has to be readable");
        assert_eq!(outs, vec![Out::Position { latitude: None, longitude: None, altitude: Some(488.0) }, Out::Reported]);
        let mut equator = listening(Source::InternalGps);
        equator.on_update(Update { latitude: Some(0.0), longitude: Some(8.545594), ..fix(Some(4.0)) }, at(1_000));
        assert_eq!((equator.latitude, equator.refusal), (None, Some(Refusal::NullIsland)), "QGCPositionManager wants both axes away from zero, so a fix on the equator is dropped rather than trusted; the port keeps that refusal instead of widening it");
    }

    #[test]
    fn a_fix_expires_and_a_fresh_one_revives_it() {
        let mut position = listening(Source::InternalGps);
        position.on_update(fix(Some(4.0)), at(1_000));
        assert_eq!((position.stale(at(1_000 + STALE_AFTER_MS)), position.fix(at(1_000 + STALE_AFTER_MS))), (false, "threeDimensional"), "the expiry is inclusive of its own limit");
        assert_eq!(
            (position.stale(at(1_001 + STALE_AFTER_MS)), position.fix(at(1_001 + STALE_AFTER_MS)), position.age_ms(at(1_001 + STALE_AFTER_MS))),
            (true, "staleThreeDimensional", Some(STALE_AFTER_MS + 1)),
            "an expired three dimensional fix says so without losing the fact that its altitude was once trusted"
        );
        assert!(position.fixed(), "a stale fix is still the last known one, it just no longer counts as current");
        assert!(!position.usable(at(1_001 + STALE_AFTER_MS)), "usable is the field a head binds 'centre on me' and 'use as home' to, and an expired coordinate is not usable");
        assert!(position.went_stale(at(1_001 + STALE_AFTER_MS)), "the first pump tick past the limit is what announces the expiry, since no update arrives to announce it");
        assert!(!position.went_stale(at(1_100 + STALE_AFTER_MS)), "and only the first, or the head is repainted ten times a second forever");
        position.on_update(fix(Some(4.0)), at(20_000));
        assert!(!position.stale(at(20_100)), "an accepted update is what clears the expiry, so the staleness cannot latch");
        assert!(position.went_stale(at(26_000)), "and the announcement arms again with the accepted update");
    }

    #[test]
    fn an_expired_fix_is_withdrawn_from_the_payload_and_kept_as_the_last_known_one() {
        let mut position = listening(Source::InternalGps);
        position.on_update(Update { direction_deg: Some(45.0), direction_accuracy_deg: Some(2.0), ..fix(Some(4.0)) }, at(1_000));
        let live = position.snapshot(at(1_500));
        assert_eq!((live["usable"].as_bool(), live["latitude"].as_f64(), live["heading"].as_f64()), (Some(true), Some(47.397742), Some(45.0)));
        let expired = position.snapshot(at(1_001 + STALE_AFTER_MS));
        assert!(
            [&expired["latitude"], &expired["longitude"], &expired["altitude"], &expired["heading"]].iter().all(|v| **v == Value::Null),
            "detections blanks its boxes once they expire and this is the one module whose payload is a coordinate a vehicle can be commanded toward, so an expired position is withdrawn rather than published as current"
        );
        assert_eq!(
            (expired["hasFix"].as_bool(), expired["usable"].as_bool(), expired["lastKnownLatitude"].as_f64(), expired["lastKnownHeading"].as_f64()),
            (Some(true), Some(false), Some(47.397742), Some(45.0)),
            "the last known reading stays readable under a name no consumer can mistake for a live one"
        );
        assert!(expired.get("available").is_none(), "'available' answered two questions at once and a head binding it to 'use as home' got a ten minute old coordinate; hasFix and usable answer one each");
    }

    #[test]
    fn a_stamp_the_clock_cannot_have_produced_is_not_a_fresh_fix() {
        let mut position = listening(Source::InternalGps);
        position.on_update(fix(Some(4.0)), at(1_700_000_000_000));
        assert_eq!(
            (position.age_ms(at(60_000)), position.stale(at(60_000)), position.usable(at(60_000))),
            (None, true, false),
            "a wall clock stamp against a monotonic reading saturates to a zero age, which would pin the fix fresh forever; a stamp from the future is no age at all and no fix to trust"
        );
        assert_eq!(position.snapshot(at(60_000))["fix"].as_str(), Some("staleThreeDimensional"));
        assert!(now() >= MonotonicMs(0), "the only stamp callers outside this module can build comes from hub::now_ms, so the head cannot feed the gates an epoch");
    }

    #[test]
    fn a_positioning_error_is_cleared_by_the_next_report_and_survives_a_code_nobody_knows() {
        let mut position = listening(Source::InternalGps);
        assert_eq!(position.on_error(0), vec![Out::Error(Some(Error::Access)), Out::Reported], "an error the head is never told about is the same dead end as having no reason at all");
        assert_eq!(position.error, Some(Error::Access));
        position.on_update(fix(Some(4.0)), at(1_000));
        assert_eq!(position.error, None, "any report from the source means it is answering again, which is the only thing that clears the error");
        position.on_error(4);
        assert_eq!(position.on_error(99), Vec::new(), "an unrecognised code says nothing about the receiver, so it announces nothing");
        assert_eq!(position.error, Some(Error::UpdateTimeout), "a code Qt adds later must not read as 'everything is fine' and clobber a live timeout");
        assert_eq!(position.on_error(ERROR_NONE), vec![Out::Error(None), Out::Reported]);
        assert_eq!(position.error, None, "QGeoPositionInfoSource::NoError is a code the head pushes like any other, and it has to clear");
        assert_eq!(Error::from_code(4).map(Error::token), Some("updateTimeoutError"));
        assert_eq!(Error::from_code(99), None);
        let mut quiet = listening(Source::Log);
        quiet.on_error(0);
        assert_eq!(quiet.on_update(fix(Some(4.0)), at(1_000)), vec![Out::Error(None), Out::Reported], "a report clears the error whatever the source is, or an error raised while the source is silent can only be cleared by switching source");
        assert_eq!((quiet.error, quiet.coordinate()), (None, (None, None, None)));
    }

    #[test]
    fn switching_source_throws_away_the_previous_receivers_fix() {
        let mut position = listening(Source::Plugin);
        position.on_update(Update { direction_deg: Some(45.0), ..fix(Some(4.0)) }, at(1_000));
        position.on_error(1);
        let outs = position.select_source(Source::Nmea);
        assert_eq!(
            outs,
            vec![
                Out::Reported,
                Out::Source(Source::Nmea),
                Out::Error(None),
                Out::Position { latitude: None, longitude: None, altitude: None },
                Out::Heading(None),
                Out::HorizontalAccuracy(None)
            ],
            "PositionManager.cpp:181-192 withdraws in this order: the empty position report first, then the coordinate, then the heading, and the horizontal accuracy last"
        );
        assert_eq!(position, GcsPosition { source: Source::Nmea, ..GcsPosition::default() });
        assert_eq!((position.fix(at(1_000)), position.age_ms(at(1_000)), position.error), ("waiting", None, None), "a stale coordinate from a receiver that is gone would otherwise outlive it");
        assert!(position.select_source(Source::Nmea) == vec![Out::Reported], "clearing what is already clear announces nothing but the empty report");
    }

    #[test]
    fn nothing_is_believed_until_a_source_is_live() {
        let mut position = GcsPosition::default();
        assert_eq!((position.source, position.source.listening()), (Source::None, false));
        assert!(position.on_update(fix(Some(4.0)), at(1_000)).is_empty(), "before the head names a source there is no receiver to have produced a fix");
        assert_eq!((position.coordinate(), position.last_report_ms), ((None, None, None), None));
        let quiet: Vec<&'static str> = SOURCES.iter().filter(|source| !source.listening()).map(|source| source.token()).collect();
        assert_eq!(quiet, vec!["none", "log", "external"], "QGCPositionManager::_setPositionSource has no case that binds a receiver to Log or ExternalGPS, so this port models them as delivering nothing; upstream its clear block leaves _currentSource pointing at the previous receiver and restarts it, which this port deliberately does not");
        position.select_source(Source::Log);
        assert!(position.on_update(fix(Some(4.0)), at(1_000)).is_empty());
        let sources = position.snapshot(at(1_000))["sources"].clone();
        assert_eq!(sources.as_array().map(Vec::len), Some(SOURCES.len()), "a head offering a source picker reads the choices from here rather than hardcoding seven tokens");
        assert_eq!(sources[5], json!({ "source": "log", "listening": false }), "and it is told which of them will never deliver a position");
    }

    #[test]
    fn the_snapshot_hands_over_numbers_and_unit_names_only() {
        let mut position = listening(Source::Nmea);
        position.on_update(Update { direction_deg: Some(45.0), direction_accuracy_deg: Some(2.0), ..fix(Some(4.0)) }, at(1_000));
        let view = position.snapshot(at(1_500));
        assert_eq!((view["class"].as_str(), view["source"].as_str(), view["hasFix"].as_bool(), view["usable"].as_bool()), (Some("GcsPosition"), Some("nmea"), Some(true), Some(true)));
        assert_eq!((view["latitude"].as_f64(), view["altitude"].as_f64(), view["heading"].as_f64()), (Some(47.397742), Some(488.0), Some(45.0)));
        assert_eq!((view["altitudeUnits"].as_str(), view["accuracyUnits"].as_str(), view["headingUnits"].as_str(), view["coordinateUnits"].as_str()), (Some("m"), Some("m"), Some("deg"), Some("deg")));
        assert_eq!((view["ageMs"].as_u64(), view["lastReportAgeMs"].as_u64(), view["stale"].as_bool(), view["fix"].as_str()), (Some(500), Some(500), Some(false), Some("threeDimensional")));
        assert_eq!((view["horizontalAccuracy"].as_f64(), view["verticalAccuracy"].as_f64(), view["minimumHorizontalAccuracy"].as_f64()), (Some(4.0), Some(5.0), Some(MIN_HORIZONTAL_ACCURACY_M)));
        assert!(view.as_object().unwrap().values().all(|v| !v.as_str().is_some_and(|token| token.contains(['.', ',']))), "a token is not a formatted number, and the decimal separator belongs to whoever knows the locale");
        let empty = GcsPosition::default().snapshot(at(1_500));
        assert!(
            [&empty["latitude"], &empty["heading"], &empty["ageMs"], &empty["horizontalAccuracy"], &empty["lastKnownLatitude"], &empty["error"]].iter().all(|v| **v == Value::Null),
            "with no receiver every reading is absent, and absent is null rather than a zero a map would happily fly to"
        );
        assert_eq!((empty["stale"].as_bool(), empty["fix"].as_str(), empty["usable"].as_bool()), (Some(false), Some("noSource"), Some(false)), "a fix that was never taken cannot be current, and it cannot be stale either: there is no reading to have expired");
    }
}
