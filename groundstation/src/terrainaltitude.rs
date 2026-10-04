const ALT_QUERY_MIN_INTERVAL_MS: u64 = 500;
const ALT_MIN_DISTANCE_TRAVELED_M: f64 = 2.0;
const ALT_MIN_ALTITUDE_CHANGED_M: f32 = 0.5;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AboveTerrain {
    queried_ms: Option<u64>,
    last: Option<((f64, f64), f32)>,
    pending: Option<(f64, f64)>,
    pub value: Option<f64>,
}

impl AboveTerrain {
    pub fn on_position(&mut self, now_ms: u64, at: (f64, f64), relative_altitude: f32) {
        let Some(queried_ms) = self.queried_ms else {
            self.queried_ms = Some(now_ms);
            return;
        };
        let unmoved = self.last.is_some_and(|(coordinate, altitude)| {
            !altitude.is_nan() && crate::track::distance_m(coordinate, at) < ALT_MIN_DISTANCE_TRAVELED_M && (relative_altitude - altitude).abs() < ALT_MIN_ALTITUDE_CHANGED_M
        });
        if now_ms.saturating_sub(queried_ms) < ALT_QUERY_MIN_INTERVAL_MS || unmoved {
            return;
        }
        self.last = Some((at, relative_altitude));
        self.pending = Some(at);
        self.queried_ms = Some(now_ms);
    }

    pub fn pending(&self) -> Option<(f64, f64)> {
        self.pending
    }

    pub fn on_terrain(&mut self, terrain: Option<f64>, altitude_amsl: f64) {
        self.pending = None;
        if let Some(height) = terrain {
            self.value = Some(altitude_amsl - height);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_waits_half_a_second_and_for_two_metres_or_half_a_metre_of_climb() {
        let mut above = AboveTerrain::default();
        above.on_position(0, (47.0, 8.0), 10.0);
        assert_eq!(above.pending(), None, "the coordinator's timer starts with the vehicle");
        above.on_position(499, (47.0, 8.0), 10.0);
        assert_eq!(above.pending(), None);
        above.on_position(500, (47.0, 8.0), 10.0);
        assert_eq!(above.pending(), Some((47.0, 8.0)));
        above.on_terrain(Some(400.0), 520.0);
        assert_eq!((above.pending(), above.value), (None, Some(120.0)));
        above.on_position(2000, (47.00001, 8.0), 10.4);
        assert_eq!(above.pending(), None, "1.1 m and 0.4 m are too little to ask again");
        above.on_position(2000, (47.0, 8.0), 10.5);
        assert_eq!(above.pending(), Some((47.0, 8.0)), "half a metre of climb asks again");
        above.on_position(2200, (47.001, 8.0), 10.5);
        assert_eq!(above.pending(), Some((47.0, 8.0)), "a new query no sooner than 500 ms after the last");
        above.on_position(2500, (47.001, 8.0), 10.5);
        assert_eq!(above.pending(), Some((47.001, 8.0)), "the newer query replaces the one in flight");
    }

    #[test]
    fn a_failed_query_keeps_the_last_height_above_terrain() {
        let mut above = AboveTerrain::default();
        above.on_position(0, (47.0, 8.0), f32::NAN);
        above.on_position(500, (47.0, 8.0), f32::NAN);
        above.on_terrain(Some(400.0), 450.0);
        above.on_position(1000, (47.0, 8.0), f32::NAN);
        assert_eq!(above.pending(), Some((47.0, 8.0)), "an unknown relative altitude always asks again");
        above.on_terrain(None, 460.0);
        assert_eq!((above.pending(), above.value), (None, Some(50.0)));
    }
}
