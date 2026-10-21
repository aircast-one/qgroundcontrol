use std::collections::HashMap;

use geo_types::{Coord, Geometry, LineString};
use mvt_reader::Reader;
use mvt_reader::feature::{Feature, Value as Property};

const ROAD_LAYER: &str = "transportation";
const DRIVABLE: [&str; 7] = [
    "motorway",
    "trunk",
    "primary",
    "secondary",
    "tertiary",
    "minor",
    "service",
];
pub const ROAD_TILE_ZOOM: u32 = 14;
const SNAP_M: f64 = 1.5;
const METRES_PER_DEG: f64 = 111_320.0;
const LOOKAHEAD_M: f64 = 10.0;
const PASS_M: f64 = 6.0;
const HORIZON_M: f64 = 80.0;
const LATERAL_MPS2: f64 = 2.0;
const BRAKE_MPS2: f64 = 2.0;
const SLOWEST_MPS: f64 = 1.5;
const STRAIGHT_DEG: f64 = 5.0;

#[derive(Debug, Clone, Default)]
pub struct Network {
    pub nodes: Vec<(f64, f64)>,
    pub next: Vec<Vec<usize>>,
    both: Vec<Vec<usize>>,
}

type Segment = ((f64, f64), (f64, f64), Flow);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Flow {
    Both,
    Forward,
    Backward,
}

fn text(feature: &Feature, name: &str) -> Option<String> {
    match feature.properties.as_ref()?.get(name)? {
        Property::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn whole(feature: &Feature, name: &str) -> Option<i64> {
    match feature.properties.as_ref()?.get(name)? {
        Property::Int(value) | Property::SInt(value) => Some(*value),
        Property::UInt(value) => i64::try_from(*value).ok(),
        _ => None,
    }
}

fn drivable(feature: &Feature) -> bool {
    text(feature, "class").is_some_and(|class| DRIVABLE.contains(&class.as_str()))
        && text(feature, "brunnel").as_deref() != Some("tunnel")
}

fn flow(feature: &Feature) -> Flow {
    match whole(feature, "oneway") {
        Some(1) => Flow::Forward,
        Some(-1) => Flow::Backward,
        _ => Flow::Both,
    }
}

fn lines(geometry: &Geometry<f32>) -> Vec<&LineString<f32>> {
    match geometry {
        Geometry::LineString(line) => vec![line],
        Geometry::MultiLineString(many) => many.0.iter().collect(),
        _ => Vec::new(),
    }
}

fn clipped(a: Coord<f64>, b: Coord<f64>, size: f64) -> Option<(Coord<f64>, Coord<f64>)> {
    let d = b - a;
    let limits = [
        (-d.x, a.x),
        (d.x, size - a.x),
        (-d.y, a.y),
        (d.y, size - a.y),
    ];
    let (low, high) =
        limits
            .iter()
            .try_fold((0.0_f64, 1.0_f64), |(low, high), &(p, q)| match p {
                _ if p == 0.0 => (q >= 0.0).then_some((low, high)),
                _ if p < 0.0 => Some((low.max(q / p), high)),
                _ => Some((low, high.min(q / p))),
            })?;
    (low < high).then(|| (a + d * low, a + d * high))
}

fn place(x: u32, y: u32, extent: f64, point: Coord<f64>) -> (f64, f64) {
    let tiles = f64::from(1_u32 << ROAD_TILE_ZOOM);
    let lon = (f64::from(x) + point.x / extent) / tiles * 360.0 - 180.0;
    let n = std::f64::consts::PI * (1.0 - 2.0 * (f64::from(y) + point.y / extent) / tiles);
    (n.sinh().atan().to_degrees(), lon)
}

fn segments(raw: &[u8], x: u32, y: u32) -> Vec<Segment> {
    let Ok(reader) = Reader::new(raw.to_vec()) else {
        return Vec::new();
    };
    let Some(layer) = reader
        .get_layer_metadata()
        .ok()
        .and_then(|layers| layers.into_iter().find(|layer| layer.name == ROAD_LAYER))
    else {
        return Vec::new();
    };
    let extent = f64::from(layer.extent);
    reader
        .get_features(layer.layer_index)
        .unwrap_or_default()
        .iter()
        .filter(|feature| drivable(feature))
        .flat_map(|feature| {
            let way = flow(feature);
            lines(&feature.geometry)
                .into_iter()
                .flat_map(move |line| {
                    line.0
                        .windows(2)
                        .filter_map(move |pair| {
                            clipped(
                                Coord::from((f64::from(pair[0].x), f64::from(pair[0].y))),
                                Coord::from((f64::from(pair[1].x), f64::from(pair[1].y))),
                                extent,
                            )
                        })
                        .map(move |(a, b)| (place(x, y, extent, a), place(x, y, extent, b), way))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub fn offset(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    (
        (to.0 - from.0) * METRES_PER_DEG,
        (to.1 - from.1) * METRES_PER_DEG * from.0.to_radians().cos(),
    )
}

fn metres(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (north, east) = offset(a, b);
    north.hypot(east)
}

fn cell(point: (f64, f64)) -> (i64, i64) {
    (
        (point.0 * METRES_PER_DEG / SNAP_M).floor() as i64,
        (point.1 * METRES_PER_DEG * point.0.to_radians().cos() / SNAP_M).floor() as i64,
    )
}

struct Builder {
    nodes: Vec<(f64, f64)>,
    next: Vec<Vec<usize>>,
    grid: HashMap<(i64, i64), Vec<usize>>,
}

impl Builder {
    fn node(&mut self, point: (f64, f64)) -> usize {
        let (row, col) = cell(point);
        let near = (-1..=1)
            .flat_map(|dr| (-1..=1).map(move |dc| (row + dr, col + dc)))
            .filter_map(|key| self.grid.get(&key))
            .flatten()
            .copied()
            .find(|&index| metres(self.nodes[index], point) <= SNAP_M);
        near.unwrap_or_else(|| {
            let index = self.nodes.len();
            self.nodes.push(point);
            self.next.push(Vec::new());
            self.grid.entry((row, col)).or_default().push(index);
            index
        })
    }

    fn link(&mut self, from: usize, to: usize) {
        if from != to && !self.next[from].contains(&to) {
            self.next[from].push(to);
        }
    }
}

pub fn network(tiles: &[(u32, u32, Vec<u8>)]) -> Network {
    joined(tiles.iter().flat_map(|(x, y, raw)| segments(raw, *x, *y)))
}

fn joined(segments: impl Iterator<Item = Segment>) -> Network {
    let built = segments.fold(
        Builder {
            nodes: Vec::new(),
            next: Vec::new(),
            grid: HashMap::new(),
        },
        |mut built, (a, b, way)| {
            let (from, to) = (built.node(a), built.node(b));
            if way != Flow::Backward {
                built.link(from, to);
            }
            if way != Flow::Forward {
                built.link(to, from);
            }
            built
        },
    );
    Network::of(built.nodes, built.next)
}

pub fn tile_of(lat: f64, lon: f64) -> (u32, u32) {
    let tiles = f64::from(1_u32 << ROAD_TILE_ZOOM);
    let x = ((lon + 180.0) / 360.0 * tiles).floor();
    let y = ((1.0 - lat.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0 * tiles).floor();
    (x as u32, y as u32)
}

pub fn tiles_around(lat: f64, lon: f64) -> Vec<(u32, u32)> {
    let (x, y) = tile_of(lat, lon);
    (-1_i64..=1)
        .flat_map(|dy| (-1_i64..=1).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| {
            Some((
                u32::try_from(i64::from(x) + dx).ok()?,
                u32::try_from(i64::from(y) + dy).ok()?,
            ))
        })
        .collect()
}

impl Network {
    pub fn of(nodes: Vec<(f64, f64)>, next: Vec<Vec<usize>>) -> Network {
        let both = (0..nodes.len())
            .flat_map(|a| next[a].iter().flat_map(move |&b| [(a, b), (b, a)]))
            .fold(
                vec![Vec::new(); nodes.len()],
                |mut both: Vec<Vec<usize>>, (a, b)| {
                    if !both[a].contains(&b) {
                        both[a].push(b);
                    }
                    both
                },
            );
        Network { nodes, next, both }
    }

    fn largest_piece(&self) -> Vec<bool> {
        let mut piece = vec![usize::MAX; self.nodes.len()];
        let mut sizes = Vec::new();
        for seed in 0..self.nodes.len() {
            if piece[seed] != usize::MAX {
                continue;
            }
            let id = sizes.len();
            piece[seed] = id;
            let mut stack = vec![seed];
            let mut size = 0;
            while let Some(at) = stack.pop() {
                size += 1;
                for &n in &self.both[at] {
                    if piece[n] == usize::MAX {
                        piece[n] = id;
                        stack.push(n);
                    }
                }
            }
            sizes.push(size);
        }
        let biggest = (0..sizes.len()).max_by_key(|&id| sizes[id]);
        piece.iter().map(|&id| Some(id) == biggest).collect()
    }

    pub fn nearest(&self, here: (f64, f64)) -> Option<usize> {
        let main = self.largest_piece();
        (0..self.nodes.len())
            .filter(|&index| main[index] && !self.next[index].is_empty())
            .min_by(|&a, &b| metres(here, self.nodes[a]).total_cmp(&metres(here, self.nodes[b])))
    }
}

fn turn_deg(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    let (n1, e1) = offset(a, b);
    let (n2, e2) = offset(b, c);
    let cross = n1 * e2 - e1 * n2;
    let dot = n1 * n2 + e1 * e2;
    cross.atan2(dot).to_degrees().abs()
}

fn corner_mps(turn: f64, cruise: f64) -> f64 {
    match turn {
        t if t < STRAIGHT_DEG => cruise,
        t => (LATERAL_MPS2 * LOOKAHEAD_M / (2.0 * (t / 2.0).to_radians().sin()))
            .sqrt()
            .clamp(SLOWEST_MPS, cruise),
    }
}

#[derive(Debug, Clone)]
pub struct Drive {
    path: Vec<usize>,
    prev: Option<usize>,
    rng: u64,
}

impl Drive {
    pub fn starting(network: &Network, here: (f64, f64), seed: u64) -> Option<Drive> {
        let start = network.nearest(here)?;
        Some(Drive {
            path: vec![start],
            prev: None,
            rng: seed | 1,
        })
    }

    pub fn start_point(&self, network: &Network) -> (f64, f64) {
        network.nodes[self.path[0]]
    }

    fn roll(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn choose(&mut self, network: &Network, from: Option<usize>, at: usize) -> Option<usize> {
        let ahead: Vec<usize> = network.next[at]
            .iter()
            .copied()
            .filter(|&n| Some(n) != from)
            .collect();
        let back_along: Vec<usize> = network.both[at]
            .iter()
            .copied()
            .filter(|&n| Some(n) != from)
            .collect();
        let options = if ahead.is_empty() { back_along } else { ahead };
        match options.len() {
            0 => from,
            count => Some(options[(self.roll() % count as u64) as usize]),
        }
    }

    fn extend(&mut self, network: &Network) {
        let length = |path: &[usize]| {
            path.windows(2)
                .map(|pair| metres(network.nodes[pair[0]], network.nodes[pair[1]]))
                .sum::<f64>()
        };
        while length(&self.path) < HORIZON_M {
            let at = *self.path.last().expect("a drive always has a node ahead");
            let from = match self.path.len() {
                1 => self.prev,
                n => Some(self.path[n - 2]),
            };
            match self.choose(network, from, at) {
                Some(next) => self.path.push(next),
                None => break,
            }
        }
    }

    pub fn steer(&mut self, network: &Network, here: (f64, f64), cruise: f64) -> ((f64, f64), f64) {
        while self.path.len() > 1 {
            let at = network.nodes[self.path[0]];
            let overshot = self.prev.is_some_and(|before| {
                let (along_n, along_e) = offset(network.nodes[before], at);
                let (beyond_n, beyond_e) = offset(at, here);
                along_n * beyond_n + along_e * beyond_e > 0.0
            });
            if metres(here, at) >= PASS_M && !overshot {
                break;
            }
            self.prev = Some(self.path.remove(0));
        }
        self.extend(network);
        let points: Vec<(f64, f64)> = std::iter::once(here)
            .chain(self.path.iter().map(|&n| network.nodes[n]))
            .collect();
        let legs: Vec<f64> = points
            .windows(2)
            .map(|pair| metres(pair[0], pair[1]))
            .collect();
        let reached: Vec<f64> = legs
            .iter()
            .scan(0.0, |total, leg| {
                *total += leg;
                Some(*total)
            })
            .collect();
        let aim = points
            .windows(2)
            .zip(&reached)
            .zip(&legs)
            .find(|((_, total), _)| **total >= LOOKAHEAD_M)
            .map(|((pair, total), leg)| {
                let share = 1.0 - (total - LOOKAHEAD_M) / leg.max(f64::EPSILON);
                (
                    pair[0].0 + (pair[1].0 - pair[0].0) * share,
                    pair[0].1 + (pair[1].1 - pair[0].1) * share,
                )
            })
            .unwrap_or(*points.last().expect("here is always a point"));
        let speed = points
            .windows(3)
            .zip(&reached)
            .map(|(three, distance)| {
                let corner = corner_mps(turn_deg(three[0], three[1], three[2]), cruise);
                (corner * corner + 2.0 * BRAKE_MPS2 * (distance - LOOKAHEAD_M).max(0.0)).sqrt()
            })
            .fold(cruise, f64::min);
        (aim, speed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: (f64, f64) = (47.3977419, 8.5455938);

    fn mock_home_tile() -> Network {
        let raw = std::fs::read(format!(
            "{}/../test/Bridge/fixtures/openfreemap-14-8580-5736-roads-around-mock-home.pbf",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("the recorded roads around the mock home");
        network(&[(8580, 5736, raw)])
    }

    fn grid(spacing: f64) -> Network {
        let at = |row: f64, col: f64| {
            (
                HOME.0 + row * spacing / METRES_PER_DEG,
                HOME.1 + col * spacing / (METRES_PER_DEG * HOME.0.to_radians().cos()),
            )
        };
        let nodes = vec![
            at(0.0, 0.0),
            at(1.0, 0.0),
            at(-1.0, 0.0),
            at(0.0, 1.0),
            at(0.0, -1.0),
        ];
        Network::of(
            nodes,
            vec![vec![1, 2, 3, 4], vec![0], vec![0], vec![0], vec![0]],
        )
    }

    #[test]
    fn the_mock_home_tile_holds_a_drivable_road_graph_with_junctions() {
        let roads = mock_home_tile();
        let junctions = roads.next.iter().filter(|out| out.len() >= 3).count();
        assert!(roads.nodes.len() > 500, "{} nodes", roads.nodes.len());
        assert!(junctions > 50, "{junctions} junctions");
        assert_eq!(tile_of(HOME.0, HOME.1), (8580, 5736));
    }

    #[test]
    fn footpaths_and_tunnels_are_not_roads_and_one_way_streets_only_go_one_way() {
        let raw = std::fs::read(format!(
            "{}/../test/Bridge/fixtures/openfreemap-14-8580-5736-roads-around-mock-home.pbf",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("fixture");
        let reader = Reader::new(raw.clone()).expect("tile");
        let layer = reader
            .get_layer_metadata()
            .expect("layers")
            .into_iter()
            .find(|layer| layer.name == ROAD_LAYER)
            .expect("transportation");
        let features = reader.get_features(layer.layer_index).expect("features");
        let kept = features.iter().filter(|f| drivable(f)).count();
        let paths = features
            .iter()
            .filter(|f| text(f, "class").as_deref() == Some("path"))
            .count();
        assert!(kept > 0 && paths > 0 && kept < features.len());
        assert!(features.iter().filter(|f| drivable(f)).all(|f| {
            text(f, "class").is_some_and(|c| DRIVABLE.contains(&c.as_str()))
                && text(f, "brunnel").as_deref() != Some("tunnel")
        }));
        let roads = network(&[(8580, 5736, raw)]);
        let one_way = (0..roads.nodes.len())
            .flat_map(|a| roads.next[a].iter().map(move |&b| (a, b)))
            .filter(|&(a, b)| !roads.next[b].contains(&a))
            .count();
        assert!(
            one_way > 0,
            "Winterthurerstrasse's carriageways are one-way"
        );
    }

    #[test]
    fn ends_that_meet_within_a_metre_join_into_one_junction() {
        let east = |metres: f64| HOME.1 + metres / (METRES_PER_DEG * HOME.0.to_radians().cos());
        let north = |metres: f64| HOME.0 + metres / METRES_PER_DEG;
        let roads = joined(
            [
                ((HOME.0, east(-40.0)), (HOME.0, east(0.0)), Flow::Both),
                ((north(0.4), east(0.3)), (HOME.0, east(40.0)), Flow::Both),
                ((HOME.0, east(0.2)), (north(40.0), east(0.0)), Flow::Forward),
            ]
            .into_iter(),
        );
        assert_eq!(roads.nodes.len(), 4, "{:?}", roads.nodes);
        let junction = (0..4)
            .find(|&n| roads.next[n].len() == 3)
            .expect("a 3-way junction");
        let north_end = (0..4)
            .find(|&n| roads.nodes[n].0 > north(30.0))
            .expect("north end");
        assert!(roads.next[junction].contains(&north_end));
        assert!(
            roads.next[north_end].is_empty(),
            "the one-way leg doesn't lead back"
        );
    }

    #[test]
    fn segments_are_clipped_to_their_own_tile() {
        let inside = clipped(
            Coord::from((-64.0, 100.0)),
            Coord::from((64.0, 100.0)),
            4096.0,
        )
        .expect("crosses the tile");
        assert!((inside.0.x - 0.0).abs() < 1e-9 && (inside.1.x - 64.0).abs() < 1e-9);
        assert_eq!(
            clipped(Coord::from((-64.0, 1.0)), Coord::from((-1.0, 1.0)), 4096.0),
            None
        );
    }

    #[test]
    fn a_drive_on_the_real_roads_never_leaves_the_graph_or_turns_back_mid_road() {
        let roads = mock_home_tile();
        let mut drive = Drive::starting(&roads, HOME, 7).expect("a road near home");
        let start = drive.start_point(&roads);
        let plans: Vec<Vec<usize>> = (0..600)
            .scan(start, |here, _| {
                let (aim, _) = drive.steer(&roads, *here, 5.0);
                let (north, east) = offset(*here, aim);
                let step = 1.0 / north.hypot(east).max(1.0);
                *here = (
                    here.0 + (aim.0 - here.0) * step,
                    here.1 + (aim.1 - here.1) * step,
                );
                Some(drive.path.clone())
            })
            .collect();
        assert!(plans.iter().all(|path| {
            path.windows(2)
                .all(|pair| roads.both[pair[0]].contains(&pair[1]))
        }));
        assert!(
            plans.windows(2).all(|pair| pair[0].contains(&pair[1][0])),
            "each plan continues the one before"
        );
        assert!(plans.iter().all(|path| {
            path.windows(3)
                .filter(|three| three[0] == three[2])
                .all(|three| roads.both[three[1]].iter().all(|&n| n == three[0]))
        }));
        let passed: std::collections::BTreeSet<usize> = plans.iter().map(|path| path[0]).collect();
        assert!(
            passed.len() >= 10,
            "drove somewhere: {} nodes",
            passed.len()
        );
    }

    #[test]
    fn at_a_four_way_junction_it_never_turns_back() {
        let roads = grid(50.0);
        let starts = (1..40u64).map(|seed| {
            let mut drive = Drive {
                path: vec![0],
                prev: Some(2),
                rng: seed,
            };
            drive.choose(&roads, Some(2), 0)
        });
        assert!(starts.clone().all(|next| next != Some(2)));
        assert!(
            [1, 3, 4]
                .iter()
                .all(|n| starts.clone().any(|next| next == Some(*n)))
        );
    }

    #[test]
    fn it_slows_for_a_corner_and_holds_cruise_on_a_straight() {
        assert_eq!(corner_mps(0.0, 5.0), 5.0);
        assert!(corner_mps(90.0, 5.0) < 4.0);
        assert!(corner_mps(170.0, 5.0) < corner_mps(90.0, 5.0));
        let roads = grid(40.0);
        let mut turning = Drive {
            path: vec![0, 3],
            prev: Some(2),
            rng: 1,
        };
        let at_corner = (roads.nodes[0].0 - 8.0 / METRES_PER_DEG, roads.nodes[0].1);
        let (_, corner_speed) = turning.steer(&roads, at_corner, 5.0);
        let mut straight = Drive {
            path: vec![0, 1],
            prev: Some(2),
            rng: 1,
        };
        let (_, straight_speed) = straight.steer(&roads, at_corner, 5.0);
        assert!(
            corner_speed < straight_speed - 0.5,
            "{corner_speed} vs {straight_speed}"
        );
    }
}
