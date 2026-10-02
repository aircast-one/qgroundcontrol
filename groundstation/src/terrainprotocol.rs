pub const SEND_INTERVAL_MS: u64 = 1000 / 12;
const GRID_COLUMNS: u8 = 8;
const POINTS_PER_SIDE: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Request {
    pub lat: i32,
    pub lon: i32,
    pub grid_spacing: u16,
    pub mask: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Send { bit: u8, data: [i16; 16] },
    Skip { bit: u8 },
    Wait,
    Done,
}

pub fn accepts(lat: i32, lon: i32) -> bool {
    lat != 0 || lon != 0
}

fn next_bit(mask: u64) -> Option<u8> {
    (mask != 0).then(|| mask.trailing_zeros() as u8)
}

pub fn grid_points(request: &Request, bit: u8) -> Vec<(f64, f64)> {
    let start = (f64::from(request.lat) / 1e7, f64::from(request.lon) / 1e7);
    let spacing = f64::from(request.grid_spacing);
    let between_grids = spacing * f64::from(POINTS_PER_SIDE);
    let (row, column) = (bit / GRID_COLUMNS, bit % GRID_COLUMNS);
    let move_to = |from: (f64, f64), east: f64, north: f64| crate::surveygrid::at_distance_and_azimuth(crate::surveygrid::at_distance_and_azimuth(from, east, 90.0), north, 0.0);
    let south_west = move_to(start, between_grids * f64::from(column), between_grids * f64::from(row));
    (0..POINTS_PER_SIDE)
        .flat_map(|r| (0..POINTS_PER_SIDE).map(move |c| (r, c)))
        .map(|(r, c)| move_to(south_west, spacing * f64::from(c), spacing * f64::from(r)))
        .collect()
}

pub fn step(request: &Request, height: impl Fn(f64, f64) -> Option<Option<f64>>) -> Step {
    let Some(bit) = next_bit(request.mask) else { return Step::Done };
    let heights: Option<Vec<Option<f64>>> = grid_points(request, bit).into_iter().map(|(lat, lon)| height(lat, lon)).collect();
    match heights {
        None => Step::Wait,
        Some(found) => match found.into_iter().collect::<Option<Vec<f64>>>() {
            None => Step::Skip { bit },
            Some(known) => Step::Send { bit, data: std::array::from_fn(|i| known[i] as i16) },
        },
    }
}

pub fn without(request: Request, bit: u8) -> Request {
    Request { mask: request.mask & !(1u64 << bit), ..request }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(mask: u64) -> Request {
        Request { lat: 474_000_000, lon: 85_000_000, grid_spacing: 100, mask }
    }

    #[test]
    fn the_lowest_set_bit_is_answered_first_with_sixteen_heights() {
        let sent = step(&request(0b1010), |lat, _| Some(Some(400.0 + (lat - 47.4) * 1e4)));
        let Step::Send { bit, data } = sent else { panic!("{sent:?}") };
        assert_eq!(bit, 1, "TerrainProtocolHandler walks the 8x7 grid from the south-west bit");
        assert_eq!(data[0], data[3], "a row runs east at one latitude");
        assert!(data[4] > data[0], "the next row is further north");
    }

    #[test]
    fn heights_still_downloading_wait_and_a_failed_tile_drops_its_bit() {
        assert_eq!(step(&request(1), |_, _| None), Step::Wait);
        assert_eq!(step(&request(1), |_, _| Some(None)), Step::Skip { bit: 0 });
        assert_eq!(step(&request(0), |_, _| Some(Some(1.0))), Step::Done);
        assert_eq!(without(request(0b11), 0).mask, 0b10);
    }

    #[test]
    fn grid_corners_step_four_spacings_per_bit() {
        let first = grid_points(&request(0), 0)[0];
        let east = grid_points(&request(0), 1)[0];
        let north = grid_points(&request(0), 8)[0];
        assert!((first.0 - 47.4).abs() < 1e-9 && (first.1 - 8.5).abs() < 1e-9);
        assert!((east.0 - first.0).abs() < 1e-5 && east.1 > first.1, "a great circle heading east drifts a hair south, as QGeoCoordinate does");
        assert!(north.0 > first.0 && (north.1 - first.1).abs() < 1e-6);
        assert!(!accepts(0, 0) && accepts(1, 0), "an uninitialised request before GPS lock is dropped");
    }
}
