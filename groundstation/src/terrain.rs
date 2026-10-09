use serde_json::{Value, json};

use crate::read::Unit;
use crate::router::Backend;

pub const DEPS: &[&str] = &["plan.missionController.visualItems.count", "plan.missionController.containsItems", "plan.dirty", "vehicles.activeVehicleAvailable", "plan.missionController@recalcTerrainProfile", "plan.missionController.simpleFlightPathSegments.count", "plan.missionController.minAMSLAltitude", "plan.missionController.maxAMSLAltitude",
    "settings.unitsSettings.verticalDistanceUnits", "settings.unitsSettings.horizontalDistanceUnits", crate::coreplan::CHANGED, crate::terrainservice::TERRAIN_CHANGED,
];

const FIELDS: &str = "specifiesCoordinate,specifiesAltitudeOnly,altitudeFrame,distanceFromStart,amslEntryAlt,terrainAltitude,terrainCollision,sequenceNumber,complexDistance,isStandaloneCoordinate,isSimpleItem,abbreviation,lastSequenceNumber,patternName,commandName,isSingleItem,homePosition";

#[derive(Debug, PartialEq, Clone)]
pub struct Point {
    pub sequence: i64,
    pub distance: f64,
    pub mission_altitude: f64,
    pub terrain_altitude: Option<f64>,
    pub collision: bool,
}

#[derive(Debug, PartialEq)]
pub struct Profile {
    pub points: Vec<Point>,
    pub min_altitude: f64,
    pub max_altitude: f64,
    pub total_distance: f64,
    pub unknown_terrain: usize,
    pub min_clearance: Option<f64>,
}

pub fn profile(points: Vec<Point>) -> Profile {
    banded(points, None)
}

const LEVEL_ROUTE_PADDING: f64 = 1.0;

pub fn banded(points: Vec<Point>, mission: Option<(f64, f64)>) -> Profile {
    let unknown_terrain = points.iter().filter(|p| p.terrain_altitude.is_none()).count();
    let total_distance = points.iter().map(|p| p.distance).fold(0.0, f64::max);
    let altitudes: Vec<f64> = points.iter().map(|p| p.mission_altitude).chain(points.iter().filter_map(|p| p.terrain_altitude)).chain(mission.into_iter().flat_map(|(low, high)| [low, high]).filter(|v| v.is_finite())).collect();
    let low = altitudes.iter().copied().fold(f64::INFINITY, f64::min);
    let high = altitudes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (low, high) = if altitudes.is_empty() { (0.0, 0.0) } else { (low, high) };
    let padding = if altitudes.is_empty() { 0.0 } else { ((high - low) * 0.1).max(LEVEL_ROUTE_PADDING) };
    let floor = match (low > 0.0, high > low) {
        (true, _) => (low - padding).max(0.0),
        (false, true) => low,
        (false, false) => low - padding,
    };
    let min_clearance = points
        .iter()
        .filter(|p| p.distance > 0.0)
        .filter_map(|p| p.terrain_altitude.map(|ground| p.mission_altitude - ground))
        .fold(None, |worst: Option<f64>, clearance| Some(worst.map_or(clearance, |worst| worst.min(clearance))));
    Profile { points, min_altitude: floor, max_altitude: high + padding, total_distance, unknown_terrain, min_clearance }
}

pub fn clearance_complete(profile: &Profile) -> bool {
    profile.min_clearance.is_some() && profile.unknown_terrain == 0
}

const TERRAIN_FRAME: i64 = 4;
const RETURN_TO_LAUNCH: i64 = 20;

fn drawable(item: &Value) -> bool {
    let flag = |key: &str| item.get(key).and_then(Value::as_bool).unwrap_or(false);
    if flag("specifiesCoordinate") {
        return true;
    }
    if !flag("specifiesAltitudeOnly") {
        return false;
    }
    let terrain_framed = item.get("altitudeFrame").or_else(|| item.get("altitudeMode")).and_then(Value::as_i64) == Some(TERRAIN_FRAME);
    let ground_known = item.get("terrainAltitude").and_then(Value::as_f64).is_some_and(f64::is_finite);
    !terrain_framed || ground_known
}

pub fn points(model: &Value) -> Vec<Point> {
    model
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .filter(|e| drawable(e))
                .filter_map(|e| {
                    let number = |key: &str| e.get(key).and_then(Value::as_f64).filter(|v| v.is_finite());
                    Some(Point {
                        sequence: e.get("sequenceNumber").and_then(Value::as_i64).unwrap_or(-1),
                        distance: number("distanceFromStart").unwrap_or(0.0),
                        mission_altitude: number("amslEntryAlt")?,
                        terrain_altitude: number("terrainAltitude"),
                        collision: e.get("terrainCollision").and_then(Value::as_bool).unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub const TILE_SPACING_M: f64 = 30.0;
const MAX_SAMPLES: usize = 10_000;
const EARTH_MEAN_RADIUS_M: f64 = 6_371_007.2;

pub fn qt_distance(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (lat1, lat2) = (from.0.to_radians(), to.0.to_radians());
    let half = |delta: f64| (delta / 2.0).sin().powi(2);
    let y = half((to.0 - from.0).to_radians()) + lat1.cos() * lat2.cos() * half((to.1 - from.1).to_radians());
    2.0 * y.sqrt().asin() * EARTH_MEAN_RADIUS_M
}

fn geodesic_distance(from: (f64, f64), to: (f64, f64)) -> f64 {
    use geographiclib_rs::InverseGeodesic;
    let distance: f64 = geographiclib_rs::Geodesic::wgs84().inverse(from.0, from.1, to.0, to.1);
    distance
}

pub fn samples(from: (f64, f64), to: (f64, f64)) -> Vec<(f64, f64)> {
    use geographiclib_rs::{DirectGeodesic, InverseGeodesic};
    let geodesic = geographiclib_rs::Geodesic::wgs84();
    let (total, azimuth, _, _): (f64, f64, f64, f64) = geodesic.inverse(from.0, from.1, to.0, to.1);
    let count = ((total / TILE_SPACING_M).ceil() as usize + 1).clamp(2, MAX_SAMPLES);
    match from == to {
        true => vec![from; count],
        false => (0..count).map(|i| geodesic.direct(from.0, from.1, azimuth, total * i as f64 / (count - 1) as f64)).collect(),
    }
}

const COLLISION_IGNORE_M: f64 = 10.0;
const SEGMENT_TYPE_TERRAIN_FRAME: i64 = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentKind {
    Generic,
    Takeoff,
    Land,
    TerrainFrame,
}

pub fn segment(from: (f64, f64), from_alt: f64, to: (f64, f64), to_alt: f64, height: &dyn Fn(f64, f64) -> Option<f64>) -> Value {
    shaped_segment(from, from_alt, to, to_alt, SegmentKind::Generic, height)
}

fn landing_segment(from: (f64, f64), from_alt: f64, to: (f64, f64), to_alt: f64, height: &dyn Fn(f64, f64) -> Option<f64>) -> Value {
    shaped_segment(from, from_alt, to, to_alt, SegmentKind::Land, height)
}

pub fn shaped_segment(from: (f64, f64), from_alt: f64, to: (f64, f64), to_alt: f64, kind: SegmentKind, height: &dyn Fn(f64, f64) -> Option<f64>) -> Value {
    let points = samples(from, to);
    let between = geodesic_distance(points[0], points[1]);
    let last_between = geodesic_distance(points[points.len() - 2], points[points.len() - 1]);
    let heights: Option<Vec<f64>> = points.iter().map(|(lat, lon)| height(*lat, *lon)).collect();
    let total = qt_distance(from, to);
    let heights = heights.unwrap_or_default();
    let slope = (to_alt - from_alt) / total;
    let collision = heights
        .iter()
        .enumerate()
        .scan(0.0, |x, (i, ground)| {
            let here = *x;
            *x += if i + 2 == heights.len() { last_between } else { between };
            let ignored = match kind {
                SegmentKind::Takeoff => here < COLLISION_IGNORE_M,
                SegmentKind::Land => here > total - COLLISION_IGNORE_M,
                SegmentKind::TerrainFrame => true,
                SegmentKind::Generic => false,
            };
            Some(!ignored && *ground > slope * here + from_alt)
        })
        .any(|hit| hit);
    json!({
        "kind": "object",
        "coord1AMSLAlt": from_alt,
        "coord2AMSLAlt": to_alt,
        "amslTerrainHeights": heights,
        "totalDistance": total,
        "distanceBetween": if heights.is_empty() { 0.0 } else { between },
        "finalDistanceBetween": if heights.is_empty() { 0.0 } else { last_between },
        "terrainCollision": collision,
        "terrainFrame": kind == SegmentKind::TerrainFrame,
    })
}

fn landing_segments(item: &Value, home_altitude: f64, height: &dyn Fn(f64, f64) -> Option<f64>) -> Option<Vec<Value>> {
    let row = crate::landingpattern::row(item)?;
    let slope = crate::landingpattern::slope_start(item)?;
    let base = if row.relative { home_altitude } else { 0.0 };
    let (entry, exit) = (row.approach_altitude + base, row.land_altitude + base);
    let loiter = item.get("useLoiterToAlt").and_then(Value::as_bool).unwrap_or(false);
    let vtol = item.get("complexItemType").and_then(Value::as_str) == Some(crate::landingpattern::VTOL_PATTERN);
    Some(match (vtol, loiter) {
        (false, true) => vec![segment(row.approach, entry, slope, entry, height), landing_segment(slope, entry, row.land, exit, height)],
        (false, false) => vec![landing_segment(row.approach, entry, row.land, exit, height)],
        (true, true) => vec![segment(row.approach, entry, slope, entry, height), segment(slope, entry, row.land, entry, height), landing_segment(row.land, entry, row.land, exit, height)],
        (true, false) => vec![segment(row.approach, entry, row.land, entry, height), landing_segment(row.land, entry, row.land, exit, height)],
    })
}

fn structure_segments(item: &Value, home_altitude: f64, height: &dyn Fn(f64, f64) -> Option<f64>) -> Option<Vec<Value>> {
    let flight = crate::structurescan::saved_flight(item).ok().filter(|f| f.len() > 2)?;
    let plan = crate::structurescan::saved_plan(item);
    let half = plan.adjusted_frontal / 2.0;
    let step = if plan.start_from_top { -half } else { half };
    let first = if plan.start_from_top { plan.structure_height } else { plan.scan_bottom_alt } + step + home_altitude;
    let entrance = flight[plan.entry_vertex % flight.len()];
    let entrance_alt = plan.entrance_alt + home_altitude;
    let layers: Vec<f64> = (0..plan.layers.max(0)).map(|i| first + 2.0 * step * i as f64).collect();
    let ring: Vec<((f64, f64), (f64, f64))> = flight.windows(2).map(|pair| (pair[0], pair[1])).chain(std::iter::once((flight[flight.len() - 1], flight[0]))).collect();
    let last = layers.last().copied().unwrap_or(0.0);
    Some(
        std::iter::once(segment(entrance, entrance_alt, entrance, first, height))
            .chain(layers.iter().enumerate().flat_map(|(i, alt)| {
                let climb = (i > 0).then(|| segment(entrance, layers[i - 1], entrance, *alt, height));
                climb.into_iter().chain(ring.iter().map(|(a, b)| segment(*a, *alt, *b, *alt, height))).collect::<Vec<_>>()
            }))
            .chain(std::iter::once(segment(entrance, last, entrance, entrance_alt, height)))
            .collect(),
    )
}

fn flown_segments(items: &[Value], height: &dyn Fn(f64, f64) -> Option<f64>) -> Vec<Value> {
    let stops: Vec<((f64, f64), f64)> = items
        .iter()
        .filter(|item| matches!(item.get("command").and_then(Value::as_i64), Some(16 | 4501)))
        .filter_map(|item| {
            let param = |i: usize| item.get("params")?.get(i)?.as_f64();
            Some(((param(4)?, param(5)?), param(6)?))
        })
        .collect();
    stops.windows(2).map(|pair| segment(pair[0].0, pair[0].1, pair[1].0, pair[1].1, height)).collect()
}

pub fn transect_segments(item: &Value, home_altitude: f64, height: &dyn Fn(f64, f64) -> Option<f64>) -> Option<Vec<Value>> {
    let kind = item.get("complexItemType").and_then(Value::as_str)?;
    if crate::landingpattern::is_landing(kind) {
        return landing_segments(item, home_altitude, height);
    }
    if kind == "StructureScan" {
        return structure_segments(item, home_altitude, height);
    }
    (kind == "survey" || kind == "CorridorScan").then_some(())?;
    let transect = item.get("TransectStyleComplexItem")?;
    let calc = transect.get("CameraCalc")?;
    let surface = calc.get("DistanceToSurface")?.as_f64()?;
    let amsl = match calc.get("DistanceMode")?.as_i64()? {
        crate::altitudemodes::RELATIVE => surface + home_altitude,
        crate::altitudemodes::ABSOLUTE => surface,
        crate::altitudemodes::CALC_ABOVE_TERRAIN => return Some(flown_segments(transect.get("Items")?.as_array()?, height)),
        TERRAIN_FRAME => f64::NAN,
        _ => return None,
    };
    let points: Vec<(f64, f64)> = transect.get("VisualTransectPoints")?.as_array()?.iter().filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))).collect();
    if amsl.is_nan() {
        let above = |at: (f64, f64)| height(at.0, at.1).map(|ground| ground + surface);
        return Some(points.windows(2).filter_map(|pair| Some(shaped_segment(pair[0], above(pair[0])?, pair[1], above(pair[1])?, SegmentKind::TerrainFrame, height))).collect());
    }
    Some(points.windows(2).map(|pair| segment(pair[0], amsl, pair[1], amsl, height)).collect())
}

fn core_segments(index: usize) -> Option<Vec<Value>> {
    (crate::vehiclefacade::switched_on() && crate::coreplan::enabled()).then_some(())?;
    let document = crate::coreplan::current_document();
    let crate::plandoc::Item::Complex { json, .. } = document.items.get(index.checked_sub(1)?)? else { return None };
    transect_segments(json, document.home.map_or(0.0, |h| h[2]), &crate::terrainservice::height)
}

pub fn along_segments(backend: &dyn Backend, index: usize, sequence: i64, start: f64) -> Vec<Point> {
    let listed = core_segments(index).unwrap_or_else(|| {
        let segments = backend.value_fields(&format!("plan.missionController.visualItems.{index}.flightPathSegments"), "coord1AMSLAlt,coord2AMSLAlt,amslTerrainHeights,totalDistance,distanceBetween,terrainCollision,segmentType");
        segments.get("elements").and_then(Value::as_array).cloned().unwrap_or_default()
    });
    listed
        .iter()
        .scan(start, |walked, segment| {
            let from = *walked;
            *walked += segment.get("totalDistance").and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(0.0);
            Some(segment_points(segment, sequence, from))
        })
        .flatten()
        .collect()
}

fn segment_points(segment: &Value, sequence: i64, from: f64) -> Vec<Point> {
    let number = |key: &str| segment.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
    let length = number("totalDistance").unwrap_or(0.0);
    let spacing = number("distanceBetween").unwrap_or(0.0);
    let (Some(low), Some(high)) = (number("coord1AMSLAlt"), number("coord2AMSLAlt")) else { return Vec::new() };
    let collision = segment.get("terrainCollision").and_then(Value::as_bool).unwrap_or(false);
    let heights: Vec<Option<f64>> = segment
        .get("amslTerrainHeights")
        .and_then(Value::as_array)
        .map(|heights| heights.iter().map(|h| h.as_f64().filter(|v| v.is_finite())).collect())
        .unwrap_or_default();
    let steps = heights.len().max(2);
    let terrain_frame = segment.get("terrainFrame") == Some(&Value::Bool(true)) || segment.get("segmentType").and_then(Value::as_i64) == Some(SEGMENT_TYPE_TERRAIN_FRAME);
    let above_ground = terrain_frame.then(|| heights.first().copied().flatten().map(|ground| low - ground)).flatten();
    if terrain_frame && above_ground.is_none() {
        let unknown = |distance: f64| Point { sequence, distance, mission_altitude: f64::NAN, terrain_altitude: None, collision: false };
        return vec![unknown(from), unknown(from + length)];
    }
    (0..steps)
        .map(|step| {
            let along = sample_at(step, steps, length, spacing);
            let straight = low + (high - low) * if length > 0.0 { along / length } else { 0.0 };
            Point {
                sequence,
                distance: from + along,
                mission_altitude: above_ground.zip(heights.get(step).copied().flatten()).map_or(straight, |(offset, ground)| ground + offset),
                terrain_altitude: heights.get(step).copied().flatten(),
                collision,
            }
        })
        .collect()
}

fn sample_at(step: usize, steps: usize, length: f64, spacing: f64) -> f64 {
    match (step + 1 == steps, spacing > 0.0) {
        (true, _) => length,
        (false, true) => (step as f64 * spacing).min(length),
        (false, false) => length * step as f64 / (steps as f64 - 1.0),
    }
}

pub fn markers(model: &Value) -> Vec<Value> {
    let elements = model.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
    elements
        .iter()
        .filter(|e| e.get("specifiesCoordinate") == Some(&Value::Bool(true)) && e.get("isStandaloneCoordinate") != Some(&Value::Bool(true)))
        .map(|e| {
            let text = |key: &str| e.get(key).and_then(Value::as_str).unwrap_or_default();
            let sequence = e.get("sequenceNumber").and_then(Value::as_i64).unwrap_or(-1);
            let distance = e.get("distanceFromStart").and_then(Value::as_f64).unwrap_or(0.0);
            let flagged = |key: &str| e.get(key) == Some(&Value::Bool(true));
            let complex = e.get("isSimpleItem") == Some(&Value::Bool(false)) && !flagged("isSingleItem") && !flagged("homePosition");
            let lettered = text("abbreviation").chars().next().filter(|c| !complex && *c > 'A' && *c < 'z');
            json!({
                "sequence": sequence,
                "distance": distance,
                "label": lettered.map_or_else(|| sequence.to_string(), |c| c.to_string()),
                "complex": complex.then(|| json!({
                    "endDistance": distance + e.get("complexDistance").and_then(Value::as_f64).unwrap_or(0.0),
                    "lastSequence": e.get("lastSequenceNumber").and_then(Value::as_i64).unwrap_or(sequence),
                    "pattern": Some(text("patternName")).filter(|p| !p.is_empty()).unwrap_or(text("commandName")),
                })),
            })
        })
        .collect()
}

fn walked(backend: &dyn Backend, model: &Value) -> Vec<Point> {
    let Some(elements) = model.get("elements").and_then(Value::as_array) else { return Vec::new() };
    elements
        .iter()
        .enumerate()
        .flat_map(|(index, element)| {
            let number = |key: &str| element.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
            let sequence = element.get("sequenceNumber").and_then(Value::as_i64).unwrap_or(-1);
            let start = number("distanceFromStart").unwrap_or(0.0);
            match number("complexDistance").filter(|metres| *metres > 0.0) {
                Some(_) => along_segments(backend, index, sequence, start),
                None => Vec::new(),
            }
        })
        .collect()
}

pub fn collides(mission_altitude: f64, ground: Option<f64>, altitude_range: f64) -> bool {
    altitude_range != 0.0 && ground.is_some_and(|ground| mission_altitude < ground)
}

fn core_model(backend: &dyn Backend, reads: &[Value]) -> Value {
    let summary = crate::coreplan::summary_fields(backend).unwrap_or(Value::Null);
    let range = summary.get("maxAMSLAltitude").and_then(Value::as_f64).zip(summary.get("minAMSLAltitude").and_then(Value::as_f64)).map_or(0.0, |(high, low)| high - low);
    let elements: Vec<Value> = reads
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let coordinate = item.get("coordinate");
            let at = |key: &str| coordinate.and_then(|c| c.get(key)).and_then(Value::as_f64);
            let placed = item.get("specifiesCoordinate").and_then(Value::as_bool) == Some(true);
            let ground = at("latitude").zip(at("longitude")).filter(|(latitude, longitude)| placed && !(*latitude == 0.0 && *longitude == 0.0)).and_then(|(latitude, longitude)| crate::terrainservice::height(latitude, longitude));
            let amsl = item.get("amslEntryAlt").and_then(Value::as_f64).unwrap_or(f64::NAN);
            let complex = item.get("isSimpleItem") == Some(&Value::Bool(false)) && item.get("homePosition") != Some(&Value::Bool(true));
            let segments_collide = complex && core_segments(index).is_some_and(|segments| segments.iter().any(|s| s["terrainCollision"] == true));
            let mut element = json!({ "terrainAltitude": ground, "terrainCollision": segments_collide || (placed && collides(amsl, ground, range)) });
            FIELDS.split(',').filter(|f| !matches!(*f, "terrainAltitude" | "terrainCollision")).for_each(|f| element[f] = item.get(f).cloned().unwrap_or(Value::Null));
            element
        })
        .collect();
    json!({ "kind": "list", "elements": elements })
}

fn spot(value: Option<&Value>) -> Option<(f64, f64)> {
    let v = value?;
    let latitude = v.get("latitude").and_then(Value::as_f64).filter(|l| l.is_finite())?;
    let longitude = v.get("longitude").and_then(Value::as_f64).filter(|l| l.is_finite())?;
    (latitude != 0.0 || longitude != 0.0).then_some((latitude, longitude))
}

const SEGMENT_FIELDS: &str = "coordinate1,coordinate2,terrainCollision";

fn leg_json(from: (f64, f64), to: (f64, f64)) -> Value {
    json!({ "from": { "latitude": from.0, "longitude": from.1 }, "to": { "latitude": to.0, "longitude": to.1 } })
}

fn flag_of(read: &Value, key: &str) -> bool {
    read.get(key) == Some(&Value::Bool(true))
}

pub fn simple_legs(reads: &[Value], fixed_wing: bool, rover: bool, height: &dyn Fn(f64, f64) -> Option<f64>) -> Vec<Value> {
    simple_segments(reads, fixed_wing, rover, height).into_iter().filter(|leg| leg.segment["terrainCollision"] == true).map(|leg| leg.line).collect()
}

pub struct SimpleSegment {
    pub line: Value,
    pub segment: Value,
    pub sequence: i64,
    pub start_distance: Option<f64>,
}

pub fn simple_segments(reads: &[Value], fixed_wing: bool, rover: bool, height: &dyn Fn(f64, f64) -> Option<f64>) -> Vec<SimpleSegment> {
    let home = reads.first().filter(|h| spot(h.get("coordinate")).is_some());
    let flown: Vec<&Value> = reads.iter().skip(1).filter(|r| flag_of(r, "specifiesCoordinate") && !flag_of(r, "isStandaloneCoordinate")).collect();
    let before_rtl = reads.iter().skip(1).position(|r| r.get("command").and_then(Value::as_i64) == Some(RETURN_TO_LAUNCH));
    let rtl_sequence = before_rtl.and_then(|at| reads.get(at + 1)).and_then(|r| r.get("sequenceNumber")).and_then(Value::as_i64);
    let flown: Vec<&Value> = flown
        .into_iter()
        .filter(|r| rtl_sequence.is_none_or(|rtl| r.get("sequenceNumber").and_then(Value::as_i64).is_some_and(|s| s < rtl)))
        .collect();
    let trailing_incomplete = match rtl_sequence {
        Some(_) => flown.iter().rev().take_while(|r| flag_of(r, "isIncomplete")).count(),
        None => 0,
    };
    let flown: Vec<&Value> = flown[..flown.len() - trailing_incomplete].to_vec();
    let first_flown = flown.first().and_then(|first| reads.iter().position(|r| std::ptr::eq(r, *first))).unwrap_or(reads.len());
    let starts_on_ground = rover || reads.iter().take(first_flown + 1).skip(1).any(|r| flag_of(r, "isTakeoffItem"));
    let path: Vec<&Value> = home.filter(|_| starts_on_ground).into_iter().chain(flown).chain(home.filter(|_| rtl_sequence.is_some())).collect();
    let to_home = |second: &Value| home.is_some_and(|h| std::ptr::eq(h, second));
    path.windows(2)
        .filter(|pair| !flag_of(pair[0], "isLandCommand") || to_home(pair[1]))
        .filter(|pair| !flag_of(pair[0], "isIncomplete") && !flag_of(pair[1], "isIncomplete"))
        .filter_map(|pair| {
            let (first, second) = (pair[0], pair[1]);
            let from = spot(first.get("exitCoordinate")).or_else(|| spot(first.get("coordinate")))?;
            let to = spot(second.get("entryCoordinate")).or_else(|| spot(second.get("coordinate")))?;
            let to_alt = second.get("amslEntryAlt")?.as_f64()?;
            let straight_up = flag_of(second, "isTakeoffItem") && !fixed_wing;
            let from_alt = if straight_up { to_alt } else { first.get("amslExitAlt").or_else(|| first.get("amslEntryAlt"))?.as_f64()? };
            let terrain_frame = !to_home(second) && first.get("altitudeFrame").or_else(|| first.get("altitudeMode")).and_then(Value::as_i64) == Some(TERRAIN_FRAME);
            let kind = match (flag_of(second, "isTakeoffItem"), flag_of(second, "isLandCommand"), terrain_frame) {
                (true, ..) => SegmentKind::Takeoff,
                (_, true, _) => SegmentKind::Land,
                (.., true) => SegmentKind::TerrainFrame,
                _ => SegmentKind::Generic,
            };
            let segment = shaped_segment(from, from_alt, to, to_alt, kind, height);
            let number = |read: &Value, key: &str| read.get(key).and_then(Value::as_f64).filter(|d| d.is_finite());
            let counted_from_home = number(second, "distanceFromStart").is_some_and(|d| d > 0.0);
            let start_distance = match flag_of(first, "homePosition") && !to_home(second) {
                true => counted_from_home.then_some(0.0),
                false => Some(number(first, "distanceFromStart").unwrap_or(0.0) + number(first, "complexDistance").unwrap_or(0.0)),
            };
            Some(SimpleSegment { line: leg_json(from, to), segment, sequence: first.get("sequenceNumber").and_then(Value::as_i64).unwrap_or(-1), start_distance })
        })
        .collect()
}

fn simple_points(legs: &[SimpleSegment]) -> Vec<Point> {
    legs.iter().filter_map(|leg| leg.start_distance.map(|start| segment_points(&leg.segment, leg.sequence, start))).flatten().collect()
}

fn collision_legs(backend: &dyn Backend) -> Vec<Value> {
    backend.value_fields("plan.missionController.simpleFlightPathSegments", SEGMENT_FIELDS)
        .get("elements")
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .filter(|s| s.get("terrainCollision") == Some(&Value::Bool(true)))
                .filter_map(|s| Some(leg_json(spot(s.get("coordinate1"))?, spot(s.get("coordinate2"))?)))
                .collect()
        })
        .unwrap_or_default()
}

fn colliding_items(model: &Value, simple: bool) -> Vec<usize> {
    model.get("elements").and_then(Value::as_array).map_or_else(Vec::new, |elements| {
        elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.get("isSimpleItem") == Some(&Value::Bool(simple)) && e.get("homePosition") != Some(&Value::Bool(true)) && e.get("terrainCollision") == Some(&Value::Bool(true)))
            .map(|(index, _)| index)
            .collect()
    })
}

fn axis_ticks(min: f64, max: f64, intervals: usize) -> Vec<String> {
    let step = if max - min > 0.0 { (max - min) / intervals as f64 } else { 1.0 };
    (0..=intervals).map(|i| min + step * i as f64).take_while(|v| *v <= max + step * 1e-9).map(|v| format!("{v:.1}")).collect()
}

pub fn terrain_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let core = (crate::vehiclefacade::switched_on() && crate::coreplan::enabled()).then(|| {
        let document = crate::coreplan::current_document();
        let class = crate::plandoc::vehicle_class(document.vehicle_type);
        (crate::missionitems::document_reads(&document, 0).unwrap_or_default(), class == crate::cmdinfo::VehicleClass::FixedWing, class == crate::cmdinfo::VehicleClass::Rover)
    });
    let model = match &core {
        Some((reads, ..)) => core_model(backend, reads),
        None => backend.value_fields("plan.missionController.visualItems", FIELDS),
    };
    let entries = points(&model);
    let inside = walked(backend, &model);
    let legs = core.as_ref().map(|(reads, fixed_wing, rover)| simple_segments(reads, *fixed_wing, *rover, &crate::terrainservice::height)).unwrap_or_default();
    let mut all: Vec<Point> = entries.into_iter().chain(inside).chain(simple_points(&legs)).collect();
    all.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
    let summary = match &core {
        Some(_) => crate::coreplan::summary_fields(backend).unwrap_or(Value::Null),
        None => backend.value_fields("plan.missionController", "minAMSLAltitude,maxAMSLAltitude"),
    };
    let mission = summary.get("minAMSLAltitude").and_then(Value::as_f64).zip(summary.get("maxAMSLAltitude").and_then(Value::as_f64));
    let profile = banded(all, mission);
    let vertical = Unit::vertical(backend);
    let usable = profile.points.len() > 1 && profile.max_altitude > profile.min_altitude;
    let checking = core.is_some() && crate::coreplan::home_ground_pending();
    json!({
        "checking": checking,
        "kind": "object",
        "class": "TerrainProfile",
        "usable": usable,
        "groundKnown": profile.unknown_terrain == 0 && profile.points.len() > 1,
        "hasCollision": !checking && profile.points.iter().any(|p| p.collision),
        "minClearanceMetres": profile.min_clearance,
        "clearanceComplete": clearance_complete(&profile),
        "clearanceText": profile.min_clearance.map(|clearance| crate::read::format_measure(vertical.show(clearance.abs()), &vertical.name)),
        "unknownTerrain": profile.unknown_terrain,
        "totalDistanceMeters": profile.total_distance,
        "minAltitudeMeters": profile.min_altitude,
        "maxAltitudeMeters": profile.max_altitude,
        "distanceText": crate::missionsummary::distance_text(profile.total_distance, crate::missionsummary::imperial(backend)),
        "lowestText": crate::read::format_measure(vertical.show(profile.min_altitude), &vertical.name),
        "highestText": crate::read::format_measure(vertical.show(profile.max_altitude), &vertical.name),
        "bandText": crate::read::range_text(profile.min_altitude, profile.max_altitude, &vertical),
        "heightHeader": format!("Height AMSL ({})", vertical.name),
        "distanceTicks": axis_ticks(0.0, Unit::horizontal(backend).show(profile.total_distance), 4),
        "heightTicks": axis_ticks(vertical.show(profile.min_altitude), vertical.show(profile.max_altitude), 3),
        "markers": markers(&model),
        "collidingItems": settled(checking, colliding_items(&model, false)),
        "collidingSimpleItems": settled(checking, colliding_items(&model, true)),
        "collisionLegs": settled::<Vec<Value>>(checking, match core {
            Some(_) => legs.iter().filter(|leg| leg.segment["terrainCollision"] == true).map(|leg| leg.line.clone()).collect(),
            None => collision_legs(backend),
        }),
        "points": profile.points.iter().map(|p| json!({
            "sequence": p.sequence,
            "distance": p.distance,
            "x": if profile.total_distance > 0.0 { p.distance / profile.total_distance } else { 0.0 },
            "missionAltitude": p.mission_altitude,
            "terrainAltitude": p.terrain_altitude,
            "collision": !checking && p.collision,
        })).collect::<Vec<_>>(),
    })
}

fn settled<T: Default>(checking: bool, conflicts: T) -> T {
    if checking { T::default() } else { conflicts }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicts_wait_for_the_ground_under_home_so_a_new_route_does_not_flash_red() {
        assert_eq!(settled(true, vec![2, 3]), Vec::<usize>::new(), "home sits at sea level until its ground height arrives, so every leg would read as underground");
        assert_eq!(settled(false, vec![2, 3]), vec![2, 3]);
    }

    #[test]
    fn a_takeoff_before_the_first_waypoint_or_a_rover_links_home_like_recalc_flight_path_segments() {
        let home = json!({ "homePosition": true, "coordinate": { "latitude": 47.0, "longitude": 8.0 }, "amslEntryAlt": 500.0 });
        let takeoff = json!({ "isTakeoffItem": true, "specifiesCoordinate": false });
        let waypoint = json!({ "specifiesCoordinate": true, "coordinate": { "latitude": 47.01, "longitude": 8.0 }, "amslEntryAlt": 550.0, "sequenceNumber": 2 });
        let flat = |_: f64, _: f64| Some(400.0);
        assert_eq!(simple_segments(&[home.clone(), takeoff, waypoint.clone()], false, false, &flat).len(), 1, "ArduPilot's NAV_TAKEOFF has no coordinate and still starts the mission from the ground");
        assert_eq!(simple_segments(&[home.clone(), waypoint.clone()], false, true, &flat).len(), 1, "a rover always starts at home");
        assert!(simple_segments(&[home, waypoint], false, false, &flat).is_empty());
    }

    #[test]
    fn a_level_route_keeps_a_band_so_its_line_is_not_drawn_on_the_floor() {
        let level = |distance: f64| Point { sequence: 1, distance, mission_altitude: 50.0, terrain_altitude: None, collision: false };
        let profile = banded(vec![level(0.0), level(100.0)], None);
        assert!(profile.min_altitude < 50.0 && profile.max_altitude > 50.0, "{} .. {}", profile.min_altitude, profile.max_altitude);
        let sea = |distance: f64| Point { sequence: 1, distance, mission_altitude: 0.0, terrain_altitude: None, collision: false };
        let low = banded(vec![sea(0.0), sea(100.0)], None);
        assert!(low.min_altitude < 0.0 && low.max_altitude > 0.0, "a level route at sea level is centred too: {} .. {}", low.min_altitude, low.max_altitude);
    }

    #[test]
    fn the_height_band_pads_a_tenth_and_stays_above_sea_level_like_terrain_profile() {
        let at = |distance: f64, mission: f64, ground: f64| Point { sequence: 1, distance, mission_altitude: mission, terrain_altitude: Some(ground), collision: false };
        let band = profile(vec![at(0.0, 600.0, 500.0), at(100.0, 600.0, 400.0)]);
        assert_eq!((band.min_altitude, band.max_altitude), (380.0, 620.0));
        let low = profile(vec![at(0.0, 30.0, 5.0), at(100.0, 30.0, 0.0)]);
        assert_eq!(low.min_altitude, 0.0, "the floor never goes below zero");
    }

    #[test]
    fn the_profile_walks_terrain_along_simple_legs_like_terrain_profile() {
        let item = |lat: f64, alt: f64, sequence: i64, from_start: f64| json!({ "specifiesCoordinate": true, "isSimpleItem": true, "coordinate": { "latitude": lat, "longitude": 8.0 }, "amslEntryAlt": alt, "sequenceNumber": sequence, "distanceFromStart": from_start });
        let reads = vec![json!({ "homePosition": true }), item(47.0, 600.0, 1, 0.0), item(47.01, 600.0, 2, 1112.0)];
        let ridge = |lat: f64, _lon: f64| Some(if lat > 47.004 && lat < 47.006 { 700.0 } else { 500.0 });
        let walked = simple_points(&simple_segments(&reads, false, false, &ridge));
        assert!(walked.len() > 30, "sampled every 30 m between the two waypoints, not just at them: {}", walked.len());
        assert!(walked.iter().any(|p| p.terrain_altitude == Some(700.0) && p.collision), "the ridge between the waypoints is on the profile and red");
        assert!(walked.first().unwrap().distance.abs() < 1.0 && (walked.last().unwrap().distance - 1112.0).abs() < 1.0, "placed on the mission's distance axis");
        assert!(walked.iter().all(|p| p.sequence == 1), "a leg belongs to the item it leaves, as setSimpleFlighPathSegment does, so labels stay on their waypoint");
        let home = json!({ "homePosition": true, "coordinate": { "latitude": 46.99, "longitude": 8.0 }, "amslEntryAlt": 500.0 });
        let rover = simple_segments(&[home.clone(), item(47.0, 600.0, 1, 0.0), item(47.01, 600.0, 2, 1112.0)], false, true, &ridge);
        assert_eq!((rover.len(), rover[0].start_distance), (2, None), "a rover's home leg is red on the map but not counted in the mission distance, so it is not on the profile");
        let survey = json!({ "specifiesCoordinate": true, "isSimpleItem": false, "coordinate": { "latitude": 47.0, "longitude": 8.0 }, "exitCoordinate": { "latitude": 47.01, "longitude": 8.0 }, "amslEntryAlt": 600.0, "amslExitAlt": 600.0, "sequenceNumber": 1, "distanceFromStart": 0.0, "complexDistance": 2500.0 });
        let rtl = json!({ "command": 20, "sequenceNumber": 9 });
        let back = simple_segments(&[home, survey, rtl], false, false, &ridge);
        assert_eq!(back.last().unwrap().start_distance, Some(2500.0), "the leg home starts where the pattern ends, not at its entry");
    }

    #[test]
    fn a_leg_that_dips_into_the_ground_is_listed_like_a_red_missionlineview() {
        let item = |lat: f64, alt: f64, simple: bool| json!({ "specifiesCoordinate": true, "isSimpleItem": simple, "coordinate": { "latitude": lat, "longitude": 8.0 }, "amslEntryAlt": alt });
        let reads = vec![json!({ "homePosition": true }), item(47.0, 600.0, true), item(47.01, 600.0, true), item(47.02, 800.0, true), item(47.03, 800.0, false)];
        let ridge = |lat: f64, _lon: f64| Some(if lat > 47.004 && lat < 47.006 { 700.0 } else { 500.0 });
        let legs = simple_legs(&reads, false, false, &ridge);
        assert_eq!(legs.len(), 1, "only the first leg crosses the ridge, and a leg into a complex item is left to its own segments");
        assert_eq!(legs[0]["from"]["latitude"], 47.0);
        assert!(simple_legs(&reads, false, false, &|_, _| None).is_empty(), "no terrain known, nothing is red");
        let land = json!({ "specifiesCoordinate": true, "isSimpleItem": true, "isLandCommand": true, "coordinate": { "latitude": 47.01, "longitude": 8.0 }, "amslEntryAlt": 500.0 });
        let after = item(47.02, 300.0, true);
        let rtl = json!({ "command": 20, "sequenceNumber": 9, "specifiesCoordinate": false });
        let home = json!({ "homePosition": true, "coordinate": { "latitude": 46.99, "longitude": 8.0 }, "amslEntryAlt": 500.0 });
        let flat = |_: f64, _: f64| Some(500.0);
        let sequenced = |mut item: Value, seq: i64| { item["sequenceNumber"] = json!(seq); item };
        let route = vec![home, sequenced(item(47.0, 600.0, true), 1), sequenced(land, 2), sequenced(after, 3), rtl];
        let hits = simple_legs(&route, false, false, &flat);
        assert!(hits.iter().all(|leg| leg["from"]["latitude"] != 47.01), "no segment is drawn out of a landing");
        assert!(hits.iter().any(|leg| leg["to"]["latitude"] == 46.99), "after an RTL the last item links home, and that low leg is red");
        assert!(!hits.iter().any(|leg| leg["to"]["latitude"] == 47.01), "a land leg's last 10 m touching the ground is not a collision");
    }

    #[test]
    fn a_leg_out_of_a_terrain_waypoint_into_a_landing_is_still_checked_and_an_incomplete_item_breaks_the_line() {
        let at = |lat: f64, alt: f64| json!({ "specifiesCoordinate": true, "isSimpleItem": true, "coordinate": { "latitude": lat, "longitude": 8.0 }, "amslEntryAlt": alt });
        let mut terrain_wp = at(47.0, 600.0);
        terrain_wp["altitudeFrame"] = json!(TERRAIN_FRAME);
        let mut land = at(47.02, 500.0);
        land["isLandCommand"] = json!(true);
        let ridge = |lat: f64, _lon: f64| Some(if lat > 47.008 && lat < 47.012 { 700.0 } else { 500.0 });
        let legs = simple_legs(&[json!({ "homePosition": true }), terrain_wp, land], false, false, &ridge);
        assert_eq!(legs.len(), 1, "segmentTypeForPair makes a pair into a landing a Land segment before it looks at the terrain frame");
        let mut survey = at(47.01, 600.0);
        survey["isIncomplete"] = json!(true);
        let gapped = simple_legs(&[json!({ "homePosition": true }), at(47.0, 600.0), survey, at(47.02, 600.0)], false, false, &ridge);
        assert!(gapped.is_empty(), "MissionController draws no segment into or out of an incomplete item, so none is checked: {gapped:?}");
        let home = json!({ "homePosition": true, "coordinate": { "latitude": 47.0, "longitude": 8.0 }, "amslEntryAlt": 600.0 });
        let mut landing = at(47.02, 600.0);
        landing["isLandCommand"] = json!(true);
        landing["sequenceNumber"] = json!(1);
        let rtl = json!({ "command": 20, "sequenceNumber": 2, "specifiesCoordinate": false });
        let back = simple_legs(&[home, landing, rtl], false, false, &ridge);
        assert_eq!(back.len(), 1, "after an RTL the leg from the last landing back home is a generic segment and is checked: {back:?}");
        let mut tail = at(47.01, 600.0);
        tail["isIncomplete"] = json!(true);
        tail["sequenceNumber"] = json!(2);
        let mut before = at(47.02, 600.0);
        before["sequenceNumber"] = json!(1);
        let home = json!({ "homePosition": true, "coordinate": { "latitude": 47.0, "longitude": 8.0 }, "amslEntryAlt": 600.0 });
        let rtl = json!({ "command": 20, "sequenceNumber": 3, "specifiesCoordinate": false });
        let home_leg = simple_legs(&[home, before, tail, rtl], false, false, &ridge);
        assert_eq!(home_leg.len(), 1, "QGC keeps the last valid item as lastFlyThroughVI past an incomplete one, so the leg home still starts there: {home_leg:?}");
    }

    #[test]
    fn markers_follow_terrain_status_item_labels() {
        let model = json!({ "elements": [
            { "specifiesCoordinate": true, "isSimpleItem": false, "isSingleItem": true, "abbreviation": "L", "sequenceNumber": 0, "distanceFromStart": 0.0 },
            { "specifiesCoordinate": true, "isSimpleItem": true, "abbreviation": "", "sequenceNumber": 1, "distanceFromStart": 120.0 },
            { "specifiesCoordinate": true, "isStandaloneCoordinate": true, "isSimpleItem": true, "sequenceNumber": 2, "distanceFromStart": 120.0 },
            { "specifiesCoordinate": false, "isSimpleItem": true, "sequenceNumber": 3 },
            { "specifiesCoordinate": true, "isSimpleItem": false, "abbreviation": "S", "sequenceNumber": 4, "lastSequenceNumber": 9, "distanceFromStart": 300.0, "complexDistance": 500.0, "patternName": "Survey" },
        ]});
        let shown = markers(&model);
        assert_eq!(shown.iter().map(|m| m["label"].as_str().unwrap()).collect::<Vec<_>>(), ["L", "1", "4"], "a complex item is numbered, and a standalone coordinate and an item with no coordinate get no marker");
        assert_eq!(shown[2]["complex"], json!({ "endDistance": 800.0, "lastSequence": 9, "pattern": "Survey" }));
        assert_eq!(shown[1]["complex"], Value::Null);
    }

    fn point(distance: f64, mission: f64, terrain: Option<f64>) -> Point {
        Point { sequence: 1, distance, mission_altitude: mission, terrain_altitude: terrain, collision: false }
    }

    #[test]
    fn a_terrain_following_scan_profiles_the_altitudes_it_flies() {
        let item = serde_json::json!({ "complexItemType": "survey", "TransectStyleComplexItem": {
            "CameraCalc": { "DistanceToSurface": 50.0, "DistanceMode": crate::altitudemodes::CALC_ABOVE_TERRAIN },
            "Items": [
                { "command": 16, "params": [0, 0, 0, null, 47.0, 8.0, 150.0] },
                { "command": 206, "params": [40, 0, 1, 0, 0, 0, 0] },
                { "command": 16, "params": [0, 0, 0, null, 47.001, 8.0, 170.0] }
            ]
        }});
        let segments = transect_segments(&item, 0.0, &|_, _| Some(100.0)).unwrap();
        assert_eq!(segments.len(), 1, "TransectStyleComplexItem draws one segment between consecutive waypoints of the saved items");
        assert_eq!((segments[0]["coord1AMSLAlt"].as_f64(), segments[0]["coord2AMSLAlt"].as_f64()), (Some(150.0), Some(170.0)));
    }

    #[test]
    fn the_profile_says_how_far_below_the_ground_it_runs_and_not_only_that_it_does() {
        let below = profile(vec![point(0.0, 700.0, Some(600.0)), point(100.0, 500.0, Some(668.0)), point(200.0, 700.0, Some(650.0))]);
        assert_eq!(below.min_clearance, Some(-168.0), "having been told the mission is below terrain the operator has to pick a new altitude, and the worst deficit is the number that choice is made from");

        let launched = profile(vec![point(0.0, 491.0, Some(491.0)), point(100.0, 543.0, Some(491.0)), point(200.0, 560.0, Some(500.0))]);
        assert_eq!(launched.min_clearance, Some(52.0), "the aircraft is ON THE GROUND at the launch point by construction, mission altitude equal to terrain altitude, so folding it in reported every mission as clearing terrain by zero - which reads as touching, and understates the real margin by the whole margin");

        let clear = profile(vec![point(0.0, 700.0, Some(600.0)), point(100.0, 700.0, Some(690.0))]);
        assert_eq!(clear.min_clearance, Some(10.0), "the same field answers how much room is left when there is room, so a head draws one number rather than two");

        let unknown = profile(vec![point(0.0, 700.0, None), point(100.0, 700.0, None)]);
        assert_eq!(unknown.min_clearance, None, "no ground under any sample is not a clearance of zero, which would read as touching");

        let partial = profile(vec![point(0.0, 700.0, None), point(100.0, 700.0, Some(720.0))]);
        assert_eq!(partial.min_clearance, Some(-20.0), "one sample with ground under it is enough to know the mission is below it somewhere");
    }

    #[test]
    fn a_clearance_measured_over_missing_ground_is_a_bound_and_the_core_says_so() {
        let complete = profile(vec![point(0.0, 700.0, Some(600.0)), point(100.0, 700.0, Some(690.0))]);
        assert_eq!(complete.unknown_terrain, 0);
        assert_eq!(complete.min_clearance, Some(10.0));

        let partial = profile(vec![point(0.0, 700.0, Some(600.0)), point(100.0, 700.0, None), point(200.0, 700.0, Some(690.0))]);
        assert_eq!(partial.min_clearance, Some(10.0), "the samples that do have ground still measure, so the number is a bound rather than nothing");
        assert_eq!(partial.unknown_terrain, 1);

        assert!(clearance_complete(&complete));
        assert!(!clearance_complete(&partial));

        let nothing = profile(Vec::new());
        assert_eq!(nothing.unknown_terrain, 0, "an empty profile has no unknown points, which is why the count is the wrong instrument");
        assert_eq!(nothing.min_clearance, None);
        assert!(!clearance_complete(&nothing), "there is no figure, so there is nothing for completeness to be true of");

        let no_ground = profile(vec![point(0.0, 700.0, None), point(100.0, 700.0, None)]);
        assert_eq!(no_ground.min_clearance, None, "ground under nothing is not a clearance of zero");
        assert!(!clearance_complete(&no_ground));
    }

    #[test]
    fn an_item_collides_only_below_known_ground_in_a_mission_with_some_height_to_it() {
        assert!(collides(500.0, Some(520.0), 40.0));
        assert!(!collides(530.0, Some(520.0), 40.0));
        assert!(!collides(500.0, None, 40.0), "unknown ground is not a collision");
        assert!(!collides(500.0, Some(520.0), 0.0), "MissionFlightStatusCalculator clears every collision when the mission has no altitude range");
    }

    #[test]
    fn a_segment_samples_every_thirty_metres_and_collides_where_the_ground_rises_above_the_line() {
        let from = (-35.3632621, 149.1652375);
        let to = (-35.3632621, 149.1662375);
        let total = qt_distance(from, to);
        assert!((total - 90.7).abs() < 0.2, "a thousandth of a degree east at this latitude is about 90.7 m, got {total}");
        assert_eq!(samples(from, to).len(), 5, "ceil(90.7 / 30) + 1");
        let flat = segment(from, 600.0, to, 600.0, &|_, _| Some(590.0));
        assert_eq!((flat["terrainCollision"].as_bool(), flat["amslTerrainHeights"].as_array().map(Vec::len)), (Some(false), Some(5)));
        let rising = segment(from, 600.0, to, 600.0, &|_, lon| Some(if lon > 149.1661 { 610.0 } else { 590.0 }));
        assert_eq!(rising["terrainCollision"], true);
        let unknown = segment(from, 600.0, to, 600.0, &|_, lon| (lon < 149.1660).then_some(590.0));
        assert_eq!((unknown["terrainCollision"].as_bool(), unknown["amslTerrainHeights"].as_array().map(Vec::len)), (Some(false), Some(0)), "a path query with a missing tile answers nothing, as TerrainTileManager does");
    }

    #[test]
    fn a_flat_mission_still_gets_a_band_to_draw_in() {
        let flat = profile(vec![point(0.0, 100.0, Some(50.0)), point(200.0, 100.0, Some(50.0))]);
        assert_eq!(flat.min_altitude, 45.0);
        assert_eq!(flat.max_altitude, 105.0);
        assert_eq!(flat.total_distance, 200.0);
        assert_eq!(flat.unknown_terrain, 0);
    }

    #[test]
    fn a_pattern_that_clips_terrain_is_named_for_the_map_tint() {
        let model = json!({ "elements": [{ "homePosition": true, "isSimpleItem": false, "terrainCollision": true }, { "isSimpleItem": true, "terrainCollision": true }, { "isSimpleItem": false, "terrainCollision": true }, { "isSimpleItem": false, "terrainCollision": false }] });
        assert_eq!(colliding_items(&model, false), [2], "complex items by visual index");
        assert_eq!(colliding_items(&model, true), [1], "and simple items apart, for SimpleItemMapVisual's loiter border");
    }

    #[test]
    fn a_terrain_frame_pattern_follows_the_ground_and_never_collides() {
        let corridor = json!({ "complexItemType": "CorridorScan", "TransectStyleComplexItem": { "CameraCalc": { "DistanceToSurface": 50.0, "DistanceMode": TERRAIN_FRAME }, "VisualTransectPoints": [[47.0, 8.0], [47.01, 8.0]] } });
        let ridge = |lat: f64, _lon: f64| Some(if lat > 47.004 && lat < 47.006 { 700.0 } else { 500.0 });
        let segments = transect_segments(&corridor, 0.0, &ridge).unwrap();
        assert_eq!((segments.len(), segments[0]["terrainCollision"].clone(), segments[0]["terrainFrame"].clone()), (1, json!(false), json!(true)), "SegmentTypeTerrainFrame skips the collision check");
        let flown = segment_points(&segments[0], 3, 0.0);
        assert!(flown.iter().all(|p| p.terrain_altitude.is_some_and(|ground| (p.mission_altitude - ground - 50.0).abs() < 1e-6)), "drawn at the ground plus the distance to the surface, as TerrainProfile::_addFlightPoints does");
        assert!(transect_segments(&corridor, 0.0, &|_, _| None).unwrap().is_empty(), "no ground, no terrain-frame segments, as _buildFlightPathCoordInfoFromPathHeightInfoForTerrainFrame returns early");
        let waiting = json!({ "coord1AMSLAlt": 550.0, "coord2AMSLAlt": 550.0, "totalDistance": 300.0, "amslTerrainHeights": [], "segmentType": 3 });
        let gap = segment_points(&waiting, 4, 100.0);
        assert_eq!(gap.iter().map(|p| (p.distance, p.terrain_altitude, p.mission_altitude.is_nan())).collect::<Vec<_>>(), vec![(100.0, None, true), (400.0, None, true)], "a Qt terrain-frame segment still waiting for heights is missing terrain with no flight line, as TerrainProfile draws it");
        let home = json!({ "homePosition": true, "coordinate": { "latitude": 47.0, "longitude": 8.0 }, "amslEntryAlt": 500.0 });
        let terrain_wp = json!({ "specifiesCoordinate": true, "altitudeFrame": TERRAIN_FRAME, "coordinate": { "latitude": 47.01, "longitude": 8.0 }, "amslEntryAlt": 550.0, "sequenceNumber": 1, "isTakeoffItem": true });
        let back = simple_segments(&[home, terrain_wp, json!({ "command": 20, "sequenceNumber": 2 })], false, false, &ridge);
        assert_eq!(back.last().unwrap().segment["terrainFrame"], json!(false), "the leg home is always generic, so it is still checked for collision");
    }

    #[test]
    fn the_axes_tick_like_terrain_status() {
        assert_eq!(axis_ticks(0.0, 1000.0, 4), ["0.0", "250.0", "500.0", "750.0", "1000.0"], "tickInterval max / 4, one decimal");
        assert_eq!(axis_ticks(380.0, 620.0, 3), ["380.0", "460.0", "540.0", "620.0"]);
        assert_eq!(axis_ticks(0.0, 0.0, 4), ["0.0"], "an empty axis keeps the interval of 1 and shows its single tick");
    }

    #[test]
    fn the_band_folds_in_the_missions_own_altitudes_like_terrain_profile() {
        let band = banded(vec![point(0.0, 100.0, Some(50.0)), point(200.0, 100.0, Some(50.0))], Some((40.0, 160.0)));
        assert_eq!((band.min_altitude, band.max_altitude), (28.0, 172.0), "fmin/fmax with minAMSLAltitude and maxAMSLAltitude before the 10% pad");
    }

    #[test]
    fn unknown_ground_is_counted() {
        let hilly = profile(vec![point(0.0, 100.0, None), point(500.0, 150.0, Some(20.0))]);
        assert_eq!(hilly.unknown_terrain, 1);
        assert_eq!((hilly.min_altitude, hilly.max_altitude), (7.0, 163.0));
    }

    #[test]
    fn points_come_only_from_items_with_a_position_and_an_altitude() {
        let model = json!({ "elements": [
            { "specifiesCoordinate": false, "sequenceNumber": 0, "distanceFromStart": 0.0, "amslEntryAlt": 500.0 },
            { "specifiesCoordinate": true, "sequenceNumber": 1, "distanceFromStart": 0.0, "amslEntryAlt": 120.0, "terrainAltitude": 100.0, "terrainCollision": false },
            { "specifiesCoordinate": true, "sequenceNumber": 2, "distanceFromStart": 300.0, "amslEntryAlt": null, "terrainAltitude": 100.0 },
            { "specifiesCoordinate": true, "sequenceNumber": 3, "distanceFromStart": 600.0, "amslEntryAlt": 90.0, "terrainAltitude": 110.0, "terrainCollision": true },
        ] });
        let listed = points(&model);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[1].sequence, 3);
        assert!(listed[1].collision);
    }

    #[test]
    fn a_takeoff_that_states_only_a_height_is_still_a_height_the_profile_draws() {
        let model = json!({ "elements": [
            { "specifiesCoordinate": true, "sequenceNumber": 0, "distanceFromStart": 0.0, "amslEntryAlt": 100.0, "terrainAltitude": 100.0 },
            { "specifiesCoordinate": false, "specifiesAltitudeOnly": true, "sequenceNumber": 1, "distanceFromStart": 0.0, "amslEntryAlt": 150.0, "terrainAltitude": 100.0 },
            { "specifiesCoordinate": true, "sequenceNumber": 2, "distanceFromStart": 500.0, "amslEntryAlt": 150.0, "terrainAltitude": 140.0 },
        ] });
        let listed = points(&model);
        assert_eq!(listed.iter().map(|p| p.sequence).collect::<Vec<_>>(), vec![0, 1, 2], "ArduPilot declares MAV_CMD_NAV_TAKEOFF specifiesCoordinate false and specifiesAltitudeOnly true, so filtering on a coordinate alone drops the climb from the profile on every APM plan");
        assert_eq!(listed[1].distance, 0.0, "the climb covers no ground, so it shares the launch point's place on the axis rather than being given width it does not have");
        assert_eq!(listed[1].mission_altitude, 150.0);

        let terrain_framed = json!({ "elements": [
            { "specifiesCoordinate": true, "sequenceNumber": 0, "distanceFromStart": 0.0, "amslEntryAlt": 585.0, "terrainAltitude": 585.0 },
            { "specifiesCoordinate": false, "specifiesAltitudeOnly": true, "altitudeFrame": 4, "sequenceNumber": 1, "distanceFromStart": 0.0, "amslEntryAlt": 50.0 },
            { "specifiesCoordinate": true, "sequenceNumber": 2, "distanceFromStart": 500.0, "amslEntryAlt": 660.0, "terrainAltitude": 640.0 },
        ] });
        let drawn = points(&terrain_framed);
        assert_eq!(drawn.iter().map(|p| p.sequence).collect::<Vec<_>>(), vec![0, 2], "a terrain-framed altitude is param7 plus the ground under the item, and an item with no place has no ground to query - so its amslEntryAlt is the bare relative height and putting it on an AMSL axis drags the band down by the height of the hill");

        let at_launch: Vec<f64> = listed.iter().filter(|p| p.distance == 0.0).map(|p| p.mission_altitude).collect();
        assert_eq!(at_launch, vec![100.0, 150.0], "two heights at one place is what a vertical climb is; without the second the head interpolates from the ground straight to the first waypoint and draws a diagonal the aircraft never flies");
    }
}

#[cfg(test)]
mod walking {
    use super::*;

    struct Route(Value);

    impl Backend for Route {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "plan.missionController.visualItems" => self.0.to_string(),
                _ => json!({ "kind": "object", "appSettingsVerticalDistanceUnitsString": "m", "appSettingsHorizontalDistanceUnitsString": "m" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true, "result": 1.0 }).to_string() }
        fn watch(&self, _p: &[String]) {}
    }

    fn leg(distance: f64, mission: f64, ground: f64, collision: bool) -> Value {
        json!({ "specifiesCoordinate": true, "distanceFromStart": distance, "amslEntryAlt": mission, "terrainAltitude": ground, "terrainCollision": collision, "sequenceNumber": 1 })
    }

    #[test]
    fn a_route_that_flies_into_the_ground_says_so_and_one_that_does_not_says_that() {
        let hits = terrain_view(&Route(json!({ "kind": "object", "elements": [leg(0.0, 700.0, 600.0, false), leg(100.0, 500.0, 650.0, true)] })), &[]);
        assert_eq!(hits["hasCollision"], true);
        assert_eq!(hits["minClearanceMetres"], -150.0, "the depth comes from the samples, not from the flag, so the two can disagree and this says they do not");
        assert_eq!(hits["clearanceComplete"], true);

        assert!(hits.get("bandText").is_some(), "the key must reach a head in whatever profile it was built with");
        assert!(hits["bandText"].as_str().unwrap().contains(" to "));
        let clears = terrain_view(&Route(json!({ "kind": "object", "elements": [leg(0.0, 700.0, 600.0, false), leg(100.0, 700.0, 650.0, false)] })), &[]);
        assert_eq!(clears["hasCollision"], false);
        assert_eq!(clears["minClearanceMetres"], 50.0);
        assert_eq!(clears["usable"], true, "a profile a head can draw, which is what makes the false meaningful rather than empty");
    }

    #[test]
    fn a_complete_clearance_always_carries_a_figure_to_state() {
        let cases = vec![
            vec![leg(0.0, 700.0, 600.0, false), leg(100.0, 700.0, 690.0, false)],
            vec![leg(0.0, 700.0, 700.04, false), leg(100.0, 700.0, 700.0, false)],
            vec![leg(0.0, 500.0, 650.0, true), leg(100.0, 500.0, 668.0, true)],
            vec![leg(0.0, 700.0, 600.0, false)],
        ];
        for points in cases {
            let view = terrain_view(&Route(json!({ "kind": "object", "elements": points })), &[]);
            if view["clearanceComplete"] == true {
                let text = view["clearanceText"].as_str().unwrap_or("");
                assert!(!text.is_empty(), "clearanceComplete was true with no figure to state: {view}");
                assert!(text.chars().any(|c| c.is_ascii_digit()), "the figure has to be a number a head can put in a sentence, not {text:?}");
                assert!(view["minClearanceMetres"].is_number(), "and the signed metres travel with it, or a head cannot tell headroom from depth");
            }
        }
    }

    struct Pattern(Value);

    impl Backend for Pattern {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "plan.missionController.visualItems.1.flightPathSegments" => self.0.to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn segment(low: f64, high: f64, length: f64, heights: Vec<f64>, collision: bool) -> Value {
        json!({ "coord1AMSLAlt": low, "coord2AMSLAlt": high, "totalDistance": length, "distanceBetween": 0.0, "amslTerrainHeights": heights, "terrainCollision": collision })
    }

    #[test]
    fn terrain_samples_sit_where_qt_put_them_rather_than_spread_evenly() {
        let spaced = json!({ "kind": "object", "elements": [json!({
            "coord1AMSLAlt": 100.0, "coord2AMSLAlt": 200.0, "totalDistance": 100.0, "distanceBetween": 30.0,
            "amslTerrainHeights": [10.0, 20.0, 30.0], "terrainCollision": false,
        })] });
        let walked = along_segments(&Pattern(spaced), 1, 3, 0.0);
        let along: Vec<f64> = walked.iter().map(|point| point.distance).collect();
        assert_eq!(along, vec![0.0, 30.0, 100.0], "Qt steps the samples by distanceBetween and closes the last gap, so spreading them evenly moves every one of them");
        assert_eq!(walked[1].mission_altitude, 130.0, "the flown altitude at a sample follows where the sample actually is");
    }

    #[test]
    fn a_pattern_is_sampled_along_its_own_path_rather_than_at_its_entry() {
        let segments = json!({ "kind": "object", "elements": [segment(600.0, 620.0, 400.0, vec![500.0, 540.0, 580.0], false)] });
        let walked = along_segments(&Pattern(segments), 1, 3, 1000.0);
        assert_eq!(walked.len(), 3, "a survey covering four hundred metres of ground is three samples of it, not one point at the corner it started from");
        assert_eq!(walked[0].distance, 1000.0, "the first sample sits where the pattern begins");
        assert_eq!(walked[2].distance, 1400.0, "and the last where it ends, which is the entry plus the distance flown");
        assert_eq!(walked[1].mission_altitude, 610.0, "the flown altitude runs between the ends of the segment");
        assert_eq!(walked[1].terrain_altitude, Some(540.0), "the ground under the middle of the pattern is what a single entry point cannot show");
        assert!(walked.iter().all(|point| point.sequence == 3), "every sample belongs to the item it came from");
    }

    #[test]
    fn a_collision_inside_a_pattern_is_carried_by_the_samples_that_are_in_it() {
        let segments = json!({ "kind": "object", "elements": [segment(600.0, 600.0, 100.0, vec![700.0, 700.0], true), segment(600.0, 600.0, 100.0, vec![500.0, 500.0], false)] });
        let walked = along_segments(&Pattern(segments), 1, 3, 0.0);
        assert_eq!(walked.len(), 4);
        assert!(walked[0].collision && walked[1].collision, "the leg that flies into the hill says so");
        assert!(!walked[2].collision && !walked[3].collision);
        assert_eq!(walked[2].distance, 100.0, "the second segment starts where the first one ended");
        assert!(profile(walked).points.iter().any(|point| point.collision));
    }

    #[test]
    fn a_segment_with_no_terrain_yet_is_sampled_with_none_rather_than_skipped() {
        let segments = json!({ "kind": "object", "elements": [segment(600.0, 620.0, 200.0, Vec::new(), false)] });
        let walked = along_segments(&Pattern(segments), 1, 3, 0.0);
        assert_eq!(walked.len(), 2, "the two ends are known even when the ground between them has not arrived");
        assert!(walked.iter().all(|point| point.terrain_altitude.is_none()));
        assert_eq!(profile(walked).unknown_terrain, 2, "and the profile counts them as unknown rather than as ground at zero");
    }

    #[test]
    fn a_segment_that_has_not_said_how_high_it_flies_contributes_nothing() {
        let segments = json!({ "kind": "object", "elements": [json!({ "totalDistance": 100.0, "amslTerrainHeights": [500.0] })] });
        assert!(along_segments(&Pattern(segments), 1, 3, 0.0).is_empty(), "a sample with no flown altitude cannot be drawn against the ground, and drawing it at zero puts the mission underground");
    }

    #[test]
    fn an_item_with_no_segments_is_left_to_its_entry_point() {
        assert!(along_segments(&Pattern(json!({ "kind": "null" })), 1, 3, 0.0).is_empty());
    }
    #[test]
    fn the_terrain_frame_number_is_the_same_enum_as_everywhere_else() {
        assert_eq!(TERRAIN_FRAME, crate::altitudemodes::TERRAIN_FRAME, "a fourth copy of the same AltMode ordinal, and altitudemodes is the one pinned against the header");
    }

}
