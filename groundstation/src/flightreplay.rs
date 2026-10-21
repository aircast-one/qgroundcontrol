use mavlink::MavHeader;
use mavlink::dialects::ardupilotmega::MavMessage;
use serde_json::{Value, json};

use crate::autodrop::TargetSource;
use crate::dropsim::Point;
use crate::hub::{Hub, Origin};

const TICK_US: u64 = 100_000;
const LINK: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub received_us: u64,
    pub captured_us: u64,
    pub payload: Value,
}

pub fn detections(text: &str) -> Vec<Detection> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|record| {
            Some(Detection {
                received_us: record.get("wallUs")?.as_u64()?,
                captured_us: record.get("capturedWallUs")?.as_u64()?,
                payload: record.get("payload")?.clone(),
            })
        })
        .collect()
}

pub fn truth_csv(text: &str) -> Vec<(u64, f64, f64)> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<f64> = line
                .split([',', ';', '\t'])
                .map(str::trim)
                .map(str::parse::<f64>)
                .collect::<Result<_, _>>()
                .ok()?;
            match fields.as_slice() {
                [seconds, lat, lon, ..] => Some(((seconds * 1e6) as u64, *lat, *lon)),
                _ => None,
            }
        })
        .collect()
}

pub fn truth_gpx(text: &str) -> Vec<(u64, f64, f64)> {
    roxmltree::Document::parse(text)
        .map(|document| {
            document
                .descendants()
                .filter(|node| node.has_tag_name("trkpt"))
                .filter_map(|point| {
                    let at = point
                        .children()
                        .find(|child| child.has_tag_name("time"))?
                        .text()?;
                    let when = chrono::DateTime::parse_from_rfc3339(at.trim()).ok()?;
                    Some((
                        when.timestamp_micros().max(0) as u64,
                        point.attribute("lat")?.parse().ok()?,
                        point.attribute("lon")?.parse().ok()?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn truth_from_tlog(tlog: &[u8], system: u8) -> Vec<(u64, f64, f64)> {
    frames(tlog)
        .into_iter()
        .filter(|(_, header, _)| header.system_id == system)
        .filter_map(|(at, _, message)| match message {
            MavMessage::GLOBAL_POSITION_INT(d) => {
                Some((at, f64::from(d.lat) / 1e7, f64::from(d.lon) / 1e7))
            }
            _ => None,
        })
        .collect()
}

fn frames(tlog: &[u8]) -> Vec<(u64, MavHeader, MavMessage)> {
    let mut decoded = Vec::new();
    crate::tlog::for_each(tlog, |at, header, message| {
        decoded.push((at, *header, message.clone()))
    });
    decoded
}

pub fn truth_at(truth: &[(u64, f64, f64)], at_us: u64) -> Option<(f64, f64)> {
    let after = truth.partition_point(|(t, _, _)| *t < at_us);
    let (t1, lat1, lon1) = *truth.get(after)?;
    let (t0, lat0, lon0) = *truth.get(after.checked_sub(1)?)?;
    let share = match t1 - t0 {
        0 => 0.0,
        span => (at_us - t0) as f64 / span as f64,
    };
    Some((lat0 + (lat1 - lat0) * share, lon0 + (lon1 - lon0) * share))
}

enum Event<'a> {
    Frame(&'a MavHeader, &'a MavMessage),
    Seen(&'a Detection),
    Tick,
}

fn percentile(sorted: &[f64], share: f64) -> f64 {
    sorted
        .get(((sorted.len() as f64 - 1.0) * share).round() as usize)
        .copied()
        .unwrap_or(f64::NAN)
}

fn stats(errors: &[f64]) -> Value {
    let sorted = {
        let mut copy = errors.to_vec();
        copy.sort_by(f64::total_cmp);
        copy
    };
    json!({
        "count": sorted.len(),
        "meanM": sorted.iter().sum::<f64>() / sorted.len().max(1) as f64,
        "p50M": percentile(&sorted, 0.5),
        "p95M": percentile(&sorted, 0.95),
        "maxM": sorted.last().copied().unwrap_or(f64::NAN),
    })
}

fn point_json(point: Point) -> Value {
    json!({ "latitude": point.lat, "longitude": point.lon })
}

pub fn replay(
    tlog: &[u8],
    detections: &[Detection],
    truth: &[(u64, f64, f64)],
    vehicle: Option<u8>,
) -> Value {
    let recorded = frames(tlog);
    let gcs = crate::mavout::gcs_system();
    let first = recorded.first().map_or(0, |(at, _, _)| *at);
    let last = recorded.last().map_or(0, |(at, _, _)| *at);
    let ticks: Vec<u64> = (0..=(last.saturating_sub(first) / TICK_US))
        .map(|n| first + n * TICK_US)
        .collect();
    let mut timeline: Vec<(u64, u8, Event)> = recorded
        .iter()
        .filter(|(_, header, _)| header.system_id != gcs)
        .map(|(at, header, message)| (*at, 0, Event::Frame(header, message)))
        .chain(
            detections
                .iter()
                .map(|seen| (seen.received_us, 1, Event::Seen(seen))),
        )
        .chain(ticks.iter().map(|at| (*at, 2, Event::Tick)))
        .collect();
    timeline.sort_by_key(|(at, order, _)| (*at, *order));

    *crate::detections::lock() = Default::default();
    crate::detections::lock().retarget(crate::detections::parse("rtsp://replay/cam0", None));
    let mut hub = Hub::default();
    let origin = Origin {
        link: LINK,
        replay: true,
        v2: true,
    };
    let mut samples = Vec::new();
    let mut releases = Vec::new();
    let mut errors = Vec::new();
    timeline.iter().for_each(|(at_us, _, event)| {
        let now_ms = at_us / 1000;
        match event {
            Event::Frame(header, message) => {
                hub.on_frame(origin, header, message, *at_us, now_ms);
                if hub.active().is_none()
                    || vehicle.is_some_and(|id| hub.active().map(|v| v.id) != Some(id))
                {
                    hub.set_active(vehicle.or_else(|| hub.vehicle_ids().first().copied()));
                }
            }
            Event::Seen(seen) => {
                let age_ms = seen.received_us.saturating_sub(seen.captured_us) / 1000;
                let payload = json!({
                    "ageMs": age_ms,
                    "boxes": seen.payload.get("boxes").cloned().unwrap_or(json!([])),
                });
                let frame = crate::detections::frame_from(&payload, now_ms, 0);
                if let Some(active) = hub.active() {
                    let captured_ms = seen.captured_us / 1000;
                    let grounded = crate::detections::grounded(frame, |centre| {
                        active.object_at(centre, captured_ms)
                    });
                    crate::detections::lock().receive(1, grounded);
                }
            }
            Event::Tick => {
                hub.tick(now_ms);
                let Some(active) = hub.active() else {
                    return;
                };
                let Some(release) = active.release_now() else {
                    return;
                };
                let Some(target) = active.target_for_autodrop(
                    TargetSource::Tracked,
                    release.position,
                    release.altitude_agl_m,
                    now_ms,
                ) else {
                    samples.push(json!({ "atUs": at_us, "tracked": false }));
                    return;
                };
                let roads = crate::autodrop::roads_under(target);
                let plan = crate::autodrop::plan(release, target, roads.as_deref());
                let error_m = truth_at(truth, *at_us).map(|(lat, lon)| {
                    crate::track::distance_m((lat, lon), (target.position.lat, target.position.lon))
                });
                errors.extend(error_m);
                samples.push(json!({
                    "atUs": at_us,
                    "tracked": true,
                    "target": point_json(target.position),
                    "speedMps": target.velocity.north_mps.hypot(target.velocity.east_mps),
                    "releaseInSeconds": plan.release_in_s,
                    "predictedMissM": plan.miss_m,
                    "truthErrorM": error_m,
                }));
                let due = plan.release_in_s - active.release_latency_s <= TICK_US as f64 / 1e6;
                if due && crate::autodrop::releasable(plan, 0.0) {
                    let landing = crate::dropsim::impact(release);
                    let fall_s = crate::dropsim::fall(release, crate::dropsim::air()).seconds;
                    let landed_us = at_us + ((active.release_latency_s + fall_s) * 1e6) as u64;
                    let actual_m = truth_at(truth, landed_us).map(|(lat, lon)| {
                        crate::track::distance_m((lat, lon), (landing.lat, landing.lon))
                    });
                    releases.push(json!({
                        "atUs": at_us,
                        "landing": point_json(landing),
                        "targetAtImpact": point_json(plan.target_at_impact),
                        "predictedMissM": plan.miss_m,
                        "truthMissM": actual_m,
                    }));
                }
            }
        }
    });
    json!({
        "frames": recorded.len(),
        "detections": detections.len(),
        "samples": samples,
        "wouldRelease": releases,
        "trackErrorAgainstTruth": stats(&errors),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truth_reads_from_csv_and_gpx_and_interpolates() {
        let csv = truth_csv("time,lat,lon\n1700000000.0,47.0,8.0\n1700000001.0,47.0001,8.0\n");
        assert_eq!(csv.len(), 2, "the header line is skipped");
        let (lat, _) = truth_at(&csv, 1_700_000_000_500_000).expect("between the two fixes");
        assert!((lat - 47.00005).abs() < 1e-9);
        let gpx = truth_gpx(
            r#"<gpx><trk><trkseg><trkpt lat="47.0" lon="8.0"><time>2023-11-14T22:13:20Z</time></trkpt></trkseg></trk></gpx>"#,
        );
        assert_eq!(gpx, vec![(1_700_000_000_000_000, 47.0, 8.0)]);
    }

    #[test]
    fn a_recorded_detection_keeps_its_capture_time() {
        let read = detections(
            r#"{"wallUs":1000250000,"capturedWallUs":1000000000,"payload":{"boxes":[]}}
{"wallUs":1,"video":"flight.mp4"}"#,
        );
        assert_eq!(read.len(), 1, "a video marker is not a detection");
        assert_eq!(read[0].received_us - read[0].captured_us, 250_000);
    }
}
