use mavlink::dialects::ardupilotmega::{GripperActions, MavCmd, MavMessage};
use serde_json::{Value, json};

use crate::dropsim::{Air, Payload, Point, Release, Velocity};

const DRAG_AREA_MAX_M2: f64 = 0.06;
const DRAG_AREA_STEPS: usize = 300;
const REFINE_ROUNDS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observed {
    pub command_us: Option<u64>,
    pub separation_us: u64,
    pub release: Release,
    pub wind: Velocity,
    pub landed: Point,
}

fn is_release(message: &MavMessage) -> bool {
    match message {
        MavMessage::COMMAND_LONG(c) => match c.command {
            MavCmd::MAV_CMD_DO_GRIPPER => {
                c.param2 as u32 == GripperActions::GRIPPER_ACTION_RELEASE as u32
            }
            MavCmd::MAV_CMD_DO_SET_SERVO => true,
            _ => false,
        },
        _ => false,
    }
}

pub fn commands(tlog: &[u8]) -> Vec<u64> {
    let gcs = crate::mavout::gcs_system();
    let mut found = Vec::new();
    crate::tlog::for_each(tlog, |at, header, message| {
        if header.system_id == gcs && is_release(message) {
            found.push(at);
        }
    });
    found
}

#[derive(Default, Clone, Copy)]
struct State {
    at_us: u64,
    position: Option<(f64, f64, f64)>,
    velocity: (f64, f64, f64),
    wind: Velocity,
}

pub fn observed(tlog: &[u8], vehicle: u8, separation_us: u64, landed: Point) -> Option<Observed> {
    let mut state = State::default();
    crate::tlog::for_each(tlog, |at, header, message| {
        if header.system_id != vehicle || at > separation_us {
            return;
        }
        match message {
            MavMessage::GLOBAL_POSITION_INT(d) => {
                state.at_us = at;
                state.position = Some((
                    f64::from(d.lat) / 1e7,
                    f64::from(d.lon) / 1e7,
                    f64::from(d.relative_alt) / 1000.0,
                ));
                state.velocity = (
                    f64::from(d.vx) / 100.0,
                    f64::from(d.vy) / 100.0,
                    -f64::from(d.vz) / 100.0,
                );
            }
            MavMessage::WIND(d) => {
                state.wind = crate::dropsim::blowing(f64::from(d.speed), f64::from(d.direction));
            }
            MavMessage::WIND_COV(d) => {
                state.wind = Velocity {
                    north_mps: f64::from(d.wind_x),
                    east_mps: f64::from(d.wind_y),
                }
            }
            _ => {}
        }
    });
    let (lat, lon, altitude) = state.position?;
    let ahead_s = separation_us.saturating_sub(state.at_us) as f64 / 1e6;
    let (vn, ve, vu) = state.velocity;
    let (lat, lon, _) = crate::geo::ned_to_geo(vn * ahead_s, ve * ahead_s, 0.0, (lat, lon, 0.0));
    Some(Observed {
        command_us: commands(tlog)
            .into_iter()
            .rev()
            .find(|at| *at <= separation_us),
        separation_us,
        release: Release {
            position: Point { lat, lon },
            altitude_agl_m: altitude + vu * ahead_s,
            velocity: Velocity {
                north_mps: vn,
                east_mps: ve,
            },
            climb_mps: vu,
        },
        wind: state.wind,
        landed,
    })
}

fn miss_m(drop: &Observed, payload: Payload) -> f64 {
    let fall = crate::dropsim::fall(
        drop.release,
        Air {
            payload,
            wind: drop.wind,
        },
    );
    let (lat, lon, _) = crate::geo::ned_to_geo(
        fall.north_m,
        fall.east_m,
        0.0,
        (drop.release.position.lat, drop.release.position.lon, 0.0),
    );
    crate::track::distance_m((lat, lon), (drop.landed.lat, drop.landed.lon))
}

fn rms(drops: &[Observed], payload: Payload) -> f64 {
    (drops
        .iter()
        .map(|drop| miss_m(drop, payload).powi(2))
        .sum::<f64>()
        / drops.len().max(1) as f64)
        .sqrt()
}

pub fn fitted_drag_area(drops: &[Observed], mass_kg: f64) -> (f64, f64) {
    let at = |area: f64| {
        rms(
            drops,
            Payload {
                mass_kg,
                drag_area_m2: area,
            },
        )
    };
    let step = DRAG_AREA_MAX_M2 / DRAG_AREA_STEPS as f64;
    let coarse = (0..=DRAG_AREA_STEPS)
        .map(|i| i as f64 * step)
        .min_by(|a, b| at(*a).total_cmp(&at(*b)))
        .unwrap_or(0.0);
    let (low, high) = (0..REFINE_ROUNDS).fold(
        ((coarse - step).max(0.0), coarse + step),
        |(low, high), _| {
            let third = (high - low) / 3.0;
            match at(low + third) <= at(high - third) {
                true => (low, high - third),
                false => (low + third, high),
            }
        },
    );
    let best = (low + high) / 2.0;
    (best, at(best))
}

pub fn release_latency_ms(drops: &[Observed]) -> Option<f64> {
    let mut delays: Vec<f64> = drops
        .iter()
        .filter_map(|drop| {
            drop.command_us
                .map(|command| drop.separation_us.saturating_sub(command) as f64 / 1000.0)
        })
        .collect();
    delays.sort_by(f64::total_cmp);
    delays.get(delays.len() / 2).copied()
}

pub fn calibration(drops: &[Observed], mass_kg: f64) -> Value {
    let (area_m2, rms_m) = fitted_drag_area(drops, mass_kg);
    let no_drag_rms = rms(drops, Payload::default());
    let latency = release_latency_ms(drops);
    let moving = drops.iter().any(|drop| {
        drop.release
            .velocity
            .north_mps
            .hypot(drop.release.velocity.east_mps)
            > 2.0
            || drop.wind.north_mps.hypot(drop.wind.east_mps) > 2.0
    });
    json!({
        "drops": drops.len(),
        "dragAreaCm2": area_m2 * 10_000.0,
        "rmsM": rms_m,
        "rmsWithoutDragM": no_drag_rms,
        "releaseLatencyMs": latency,
        "dragObservable": moving,
        "settings": {
            "settings.appSettings.autodropPayloadMassG": (mass_kg * 1000.0).round(),
            "settings.appSettings.autodropPayloadDragAreaCm2": (area_m2 * 10_000.0 * 100.0).round() / 100.0,
            "settings.appSettings.autodropReleaseLatencyMs": latency.map(f64::round),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::MavHeader;
    use mavlink::dialects::ardupilotmega::*;

    struct Lcg(u64);

    impl Lcg {
        fn unit(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (self.0 >> 11) as f64 / (1_u64 << 53) as f64
        }

        fn gauss(&mut self) -> f64 {
            let (u, v) = (self.unit().max(1e-12), self.unit());
            (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
        }
    }

    fn framed(system: u8, message: &MavMessage) -> Vec<u8> {
        let mut bytes = Vec::new();
        let header = MavHeader {
            system_id: system,
            component_id: 1,
            sequence: 0,
        };
        let _ = mavlink::write_v2_msg(&mut bytes, header, message);
        bytes
    }

    #[test]
    fn simulated_drops_of_a_known_payload_recover_its_drag_and_release_delay() {
        let truth = Payload {
            mass_kg: 0.5,
            drag_area_m2: 0.012,
        };
        let true_delay_us = 320_000_u64;
        let mut rng = Lcg(7);
        let gcs = crate::mavout::gcs_system();
        let drops: Vec<(Vec<u8>, u64, Point)> = (0..12_u64)
            .map(|i| {
                let command_us = 1_700_000_000_000_000 + i * 60_000_000;
                let separation_us = command_us + true_delay_us + (rng.gauss() * 20_000.0) as u64;
                let heading = rng.unit() * 360.0;
                let speed = rng.unit() * 10.0;
                let wind_from = rng.unit() * 360.0;
                let wind_speed = rng.unit() * 6.0;
                let altitude = 25.0 + rng.unit() * 20.0;
                let (vn, ve) = (
                    speed * heading.to_radians().cos(),
                    speed * heading.to_radians().sin(),
                );
                let release = Release {
                    position: Point {
                        lat: 47.4,
                        lon: 8.55,
                    },
                    altitude_agl_m: altitude,
                    velocity: Velocity {
                        north_mps: vn,
                        east_mps: ve,
                    },
                    climb_mps: 0.0,
                };
                let fall = crate::dropsim::fall(
                    release,
                    Air {
                        payload: truth,
                        wind: crate::dropsim::blowing(wind_speed, wind_from),
                    },
                );
                let (lat, lon, _) = crate::geo::ned_to_geo(
                    fall.north_m + rng.gauss() * 0.3,
                    fall.east_m + rng.gauss() * 0.3,
                    0.0,
                    (47.4, 8.55, 0.0),
                );
                let position = MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA {
                    lat: (47.4 * 1e7) as i32,
                    lon: (8.55 * 1e7) as i32,
                    relative_alt: (altitude * 1000.0) as i32,
                    vx: (vn * 100.0) as i16,
                    vy: (ve * 100.0) as i16,
                    ..Default::default()
                });
                let wind = MavMessage::WIND(WIND_DATA {
                    direction: wind_from as f32,
                    speed: wind_speed as f32,
                    speed_z: 0.0,
                });
                let command = MavMessage::COMMAND_LONG(COMMAND_LONG_DATA {
                    command: MavCmd::MAV_CMD_DO_GRIPPER,
                    param1: 1.0,
                    param2: GripperActions::GRIPPER_ACTION_RELEASE as u32 as f32,
                    ..Default::default()
                });
                let tlog = [
                    crate::tlog::record(command_us - 50_000, &framed(1, &wind)),
                    crate::tlog::record(command_us, &framed(gcs, &command)),
                    crate::tlog::record(separation_us, &framed(1, &position)),
                ]
                .concat();
                (tlog, separation_us, Point { lat, lon })
            })
            .collect();
        let observed: Vec<Observed> = drops
            .iter()
            .filter_map(|(tlog, separation, landed)| observed(tlog, 1, *separation, *landed))
            .collect();
        let result = calibration(&observed, 0.5);
        let area = result["dragAreaCm2"].as_f64().unwrap();
        let latency = result["releaseLatencyMs"].as_f64().unwrap();
        println!("{result}");
        assert_eq!(observed.len(), 12);
        assert!(
            (area - 120.0).abs() < 12.0,
            "true 120 cm2, fitted {area:.1}: {result}"
        );
        assert!(
            (latency - 320.0).abs() < 25.0,
            "true 320 ms, fitted {latency}: {result}"
        );
        assert!(
            result["rmsM"].as_f64().unwrap() < result["rmsWithoutDragM"].as_f64().unwrap(),
            "{result}"
        );
    }
}
