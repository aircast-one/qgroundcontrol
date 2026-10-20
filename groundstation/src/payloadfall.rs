use serde_json::{Value, json};

const GRAVITY_MPS2: f64 = 9.81;
const LYING_DEG: f64 = 90.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Release {
    pub bay: i64,
    pub at_ms: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub above_home: f64,
    pub velocity: (f64, f64, f64),
    pub heading: f64,
    pub ground: f64,
}

impl Release {
    fn height(&self) -> f64 {
        self.above_home.max(self.ground)
    }

    pub fn falls_for_s(&self) -> f64 {
        let climb = self.velocity.2;
        (climb + (climb * climb + 2.0 * GRAVITY_MPS2 * (self.height() - self.ground)).sqrt())
            / GRAVITY_MPS2
    }

    pub fn at(&self, now_ms: i64) -> Value {
        let falls_for = self.falls_for_s();
        let t = ((now_ms - self.at_ms) as f64 / 1000.0).clamp(0.0, falls_for);
        let landed = t >= falls_for;
        let (north, east, climb) = self.velocity;
        let (latitude, longitude, _) = crate::geo::ned_to_geo(
            north * t,
            east * t,
            0.0,
            (self.latitude, self.longitude, 0.0),
        );
        json!({
            "bay": self.bay,
            "latitude": latitude,
            "longitude": longitude,
            "aboveHome": if landed { self.ground } else { self.height() + climb * t - GRAVITY_MPS2 * t * t / 2.0 },
            "heading": self.heading,
            "tilt": if landed { LYING_DEG } else { LYING_DEG * t / falls_for },
            "landed": landed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn released(above_home: f64, velocity: (f64, f64, f64), ground: f64) -> Release {
        Release {
            bay: 1,
            at_ms: 1_000,
            latitude: -35.363,
            longitude: 149.165,
            above_home,
            velocity,
            heading: 30.0,
            ground,
        }
    }

    fn moved(release: &Release, bottle: &Value) -> (f64, f64) {
        let (north, east, _) = crate::geo::geo_to_ned(
            bottle["latitude"].as_f64().unwrap(),
            bottle["longitude"].as_f64().unwrap(),
            0.0,
            (release.latitude, release.longitude, 0.0),
        );
        (north, east)
    }

    #[test]
    fn a_bottle_dropped_moving_lands_where_the_ballistic_flight_takes_it() {
        let release = released(20.0, (5.0, -2.0, 0.0), 0.0);
        let falls_for = (2.0 * 20.0 / GRAVITY_MPS2).sqrt();
        assert!((release.falls_for_s() - falls_for).abs() < 1e-9);
        let landed = release.at(1_000 + (falls_for * 1000.0).ceil() as i64);
        assert_eq!(landed["landed"], true);
        assert_eq!(landed["aboveHome"].as_f64(), Some(0.0));
        let (north, east) = moved(&release, &landed);
        assert!(
            (north - 5.0 * falls_for).abs() < 0.01 && (east + 2.0 * falls_for).abs() < 0.01,
            "it keeps the drone's ground speed until it lands: {north} m north, {east} m east"
        );
    }

    #[test]
    fn a_falling_bottle_is_on_its_parabola_and_turning_over() {
        let release = released(20.0, (0.0, 0.0, 2.0), 0.0);
        let half_second = release.at(1_500);
        assert_eq!(half_second["landed"], false);
        assert!(
            (half_second["aboveHome"].as_f64().unwrap()
                - (20.0 + 2.0 * 0.5 - GRAVITY_MPS2 * 0.125))
                .abs()
                < 1e-9,
            "a climbing drone throws the bottle up before it falls"
        );
        let tilt = half_second["tilt"].as_f64().unwrap();
        assert!(tilt > 0.0 && tilt < LYING_DEG);
        assert_eq!(
            release.at(0)["aboveHome"].as_f64(),
            Some(20.0),
            "before the release the bottle is still under the drone"
        );
    }

    #[test]
    fn the_ground_it_lands_on_is_the_terrain_under_the_drone_not_home() {
        let release = released(20.0, (0.0, 0.0, 0.0), 6.0);
        assert!((release.falls_for_s() - (2.0 * 14.0 / GRAVITY_MPS2).sqrt()).abs() < 1e-9);
        assert_eq!(release.at(60_000)["aboveHome"].as_f64(), Some(6.0));
        let below = released(3.0, (0.0, 0.0, 0.0), 6.0);
        assert_eq!(
            below.at(1_000)["landed"],
            true,
            "a drone drawn under the ground drops onto it"
        );
    }

    #[test]
    fn a_landed_bottle_lies_where_it_fell_for_the_rest_of_the_session() {
        let release = released(10.0, (3.0, 4.0, 0.0), 0.0);
        let first = release.at(10_000);
        assert_eq!(first["tilt"].as_f64(), Some(LYING_DEG));
        assert_eq!(first, release.at(3_600_000));
    }
}
