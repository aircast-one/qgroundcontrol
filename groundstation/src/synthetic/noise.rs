pub const SHAPE_SIZE: u32 = 512;
pub const SHAPE_PERIOD: i64 = 16;
pub const DETAIL_SIZE: u32 = 128;
pub const DETAIL_DEPTH: u32 = 32;
pub const DETAIL_PERIOD: i64 = 8;
pub const DETAIL_RISE: f64 = 2.5;

fn hash(x: i64, y: i64, z: i64, seed: u32) -> f64 {
    let mixed = [x, y, z]
        .into_iter()
        .fold(seed.wrapping_mul(0x9e37_79b9), |h, v| {
            (h ^ (v as u32))
                .wrapping_mul(0x85eb_ca6b)
                .rotate_left(13)
                .wrapping_add(0xc2b2_ae35)
        });
    let spread = (mixed ^ (mixed >> 16)).wrapping_mul(0x7feb_352d);
    f64::from(spread ^ (spread >> 15)) / f64::from(u32::MAX)
}

fn wrap(cell: i64, period: i64) -> i64 {
    cell.rem_euclid(period)
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn perlin(x: f64, y: f64, period: i64) -> f64 {
    let (cx, cy) = (x.floor(), y.floor());
    let (fx, fy) = (x - cx, y - cy);
    let corner = |dx: i64, dy: i64| {
        let angle = hash(
            wrap(cx as i64 + dx, period),
            wrap(cy as i64 + dy, period),
            0,
            1,
        ) * std::f64::consts::TAU;
        angle.cos() * (fx - dx as f64) + angle.sin() * (fy - dy as f64)
    };
    let (u, v) = (fade(fx), fade(fy));
    let bottom = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * u;
    let top = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * u;
    0.5 + 0.7 * (bottom + (top - bottom) * v)
}

fn worley(x: f64, y: f64, period: i64) -> f64 {
    let (cx, cy) = (x.floor() as i64, y.floor() as i64);
    let (fx, fy) = (x - cx as f64, y - cy as f64);
    let nearest = (-1..=1)
        .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| {
            let (wx, wy) = (wrap(cx + dx, period), wrap(cy + dy, period));
            let px = dx as f64 + hash(wx, wy, 0, 2) - fx;
            let py = dy as f64 + hash(wx, wy, 0, 3) - fy;
            px * px + py * py
        })
        .fold(4.0, f64::min);
    1.0 - nearest.sqrt()
}

fn value3(x: f64, y: f64, z: f64, period: i64) -> f64 {
    let (cx, cy, cz) = (x.floor() as i64, y.floor() as i64, z.floor() as i64);
    let smooth = |t: f64| t * t * (3.0 - 2.0 * t);
    let (u, v, w) = (
        smooth(x - cx as f64),
        smooth(y - cy as f64),
        smooth(z - cz as f64),
    );
    let at =
        |dx: i64, dy: i64, dz: i64| hash(wrap(cx + dx, period), wrap(cy + dy, period), cz + dz, 4);
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let layer = |dz: i64| {
        lerp(
            lerp(at(0, 0, dz), at(1, 0, dz), u),
            lerp(at(0, 1, dz), at(1, 1, dz), u),
            v,
        )
    };
    lerp(layer(0), layer(1), w)
}

fn billow(x: f64, y: f64, z: f64, period: i64) -> f64 {
    let fold = |n: f64| 1.0 - (2.0 * n - 1.0).abs();
    fold(value3(x, y, z, period)) * 0.625
        + fold(value3(
            2.0 * x + 11.0,
            2.0 * y + 11.0,
            2.0 * z + 11.0,
            2 * period,
        )) * 0.25
        + fold(value3(
            4.0 * x + 23.0,
            4.0 * y + 23.0,
            4.0 * z + 23.0,
            4 * period,
        )) * 0.125
}

fn byte(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub fn cloud_shape() -> Vec<u8> {
    let period = SHAPE_PERIOD as f64;
    (0..SHAPE_SIZE)
        .flat_map(|j| (0..SHAPE_SIZE).map(move |i| (i, j)))
        .flat_map(|(i, j)| {
            let x = (f64::from(i) + 0.5) / f64::from(SHAPE_SIZE) * period;
            let y = (f64::from(j) + 0.5) / f64::from(SHAPE_SIZE) * period;
            let shape = perlin(x, y, SHAPE_PERIOD) * 0.625
                + perlin(2.0 * x + 13.0, 2.0 * y + 13.0, 2 * SHAPE_PERIOD) * 0.25
                + perlin(4.0 * x + 29.0, 4.0 * y + 29.0, 4 * SHAPE_PERIOD) * 0.125;
            let (qx, qy) = (1.5 * x + 5.0, 1.5 * y + 5.0);
            let puff = worley(qx, qy, 3 * SHAPE_PERIOD / 2) * 0.625
                + worley(2.0 * qx + 11.0, 2.0 * qy + 11.0, 3 * SHAPE_PERIOD) * 0.25
                + worley(4.0 * qx + 23.0, 4.0 * qy + 23.0, 6 * SHAPE_PERIOD) * 0.125;
            [byte(shape), byte(puff), 0, 255]
        })
        .collect()
}

pub fn cloud_detail() -> Vec<u8> {
    let period = DETAIL_PERIOD as f64;
    (0..DETAIL_DEPTH)
        .flat_map(|k| (0..DETAIL_SIZE).flat_map(move |j| (0..DETAIL_SIZE).map(move |i| (i, j, k))))
        .map(|(i, j, k)| {
            let x = (f64::from(i) + 0.5) / f64::from(DETAIL_SIZE) * period;
            let y = (f64::from(j) + 0.5) / f64::from(DETAIL_SIZE) * period;
            let z = (f64::from(k) + 0.5) / f64::from(DETAIL_DEPTH) * DETAIL_RISE;
            byte(billow(x, y, z, DETAIL_PERIOD))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cloud_noise_tiles_without_a_seam() {
        let period = SHAPE_PERIOD as f64;
        let shape = |x: f64| {
            perlin(x, 3.3, SHAPE_PERIOD) * 0.625
                + perlin(2.0 * x + 13.0, 19.6, 2 * SHAPE_PERIOD) * 0.25
        };
        let puff = |x: f64| worley(1.5 * x + 5.0, 9.95, 3 * SHAPE_PERIOD / 2);
        let detail = |x: f64| billow(x, 1.7, 0.9, DETAIL_PERIOD);
        [0.0, 0.37, 5.2].into_iter().for_each(|x| {
            assert!((shape(x) - shape(x + period)).abs() < 1e-9);
            assert!((puff(x) - puff(x + period)).abs() < 1e-9);
            assert!((detail(x) - detail(x + DETAIL_PERIOD as f64)).abs() < 1e-9);
        });
        assert_eq!(cloud_shape().len(), (SHAPE_SIZE * SHAPE_SIZE * 4) as usize);
        assert_eq!(
            cloud_detail().len(),
            (DETAIL_SIZE * DETAIL_SIZE * DETAIL_DEPTH) as usize
        );
    }
}
