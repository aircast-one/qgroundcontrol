#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Camera {
    pub focal_length: f64,
    pub sensor_width: f64,
    pub sensor_height: f64,
    pub image_width: f64,
    pub image_height: f64,
    pub landscape: bool,
    pub frontal_overlap: f64,
    pub side_overlap: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Footprint {
    pub image_density: f64,
    pub distance_to_surface: f64,
    pub footprint_side: f64,
    pub footprint_frontal: f64,
    pub adjusted_side: f64,
    pub adjusted_frontal: f64,
}

fn usable(camera: &Camera) -> bool {
    [camera.focal_length, camera.sensor_width, camera.sensor_height, camera.image_width, camera.image_height].iter().all(|value| *value > 0.0)
}

pub fn from_distance(camera: &Camera, distance_to_surface: f64) -> Option<Footprint> {
    if !usable(camera) || distance_to_surface <= 0.0 {
        return None;
    }
    let image_density = (distance_to_surface * camera.sensor_width * 100.0) / (camera.image_width * camera.focal_length);
    Some(footprint(camera, image_density, distance_to_surface))
}

pub fn from_density(camera: &Camera, image_density: f64) -> Option<Footprint> {
    if !usable(camera) || image_density <= 0.0 {
        return None;
    }
    let distance_to_surface = (camera.image_width * image_density * camera.focal_length) / (camera.sensor_width * 100.0);
    Some(footprint(camera, image_density, distance_to_surface))
}

fn footprint(camera: &Camera, image_density: f64, distance_to_surface: f64) -> Footprint {
    let across = (camera.image_width * image_density) / 100.0;
    let along = (camera.image_height * image_density) / 100.0;
    let (footprint_side, footprint_frontal) = if camera.landscape { (across, along) } else { (along, across) };
    Footprint {
        image_density,
        distance_to_surface,
        footprint_side,
        footprint_frontal,
        adjusted_side: footprint_side * ((100.0 - camera.side_overlap) / 100.0),
        adjusted_frontal: footprint_frontal * ((100.0 - camera.frontal_overlap) / 100.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn oracle() -> Value {
        let text = std::fs::read_to_string("../test/Bridge/fixtures/survey-transects.json").expect("the oracle is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the oracle is JSON")
    }

    #[test]
    fn every_recorded_camera_computes_identically() {
        let cases = oracle();
        let cases = cases.as_object().expect("the oracle is an object of cases");
        let checked: Vec<(String, bool, String)> = cases
            .iter()
            .filter(|(_, case)| case["kind"] == "camera")
            .map(|(name, case)| {
                let camera = Camera {
                    focal_length: case["focalLength"].as_f64().unwrap(),
                    sensor_width: case["sensorWidth"].as_f64().unwrap(),
                    sensor_height: case["sensorHeight"].as_f64().unwrap(),
                    image_width: case["imageWidth"].as_f64().unwrap(),
                    image_height: case["imageHeight"].as_f64().unwrap(),
                    landscape: case["landscape"].as_bool().unwrap(),
                    frontal_overlap: case["frontalOverlap"].as_f64().unwrap(),
                    side_overlap: case["sideOverlap"].as_f64().unwrap(),
                };
                let ours = from_distance(&camera, case["distanceToSurface"].as_f64().unwrap()).unwrap();
                let spelled = |value: f64| format!("{value:.7}");
                let matched = spelled(ours.image_density) == case["imageDensity"].as_str().unwrap()
                    && spelled(ours.adjusted_side) == case["adjustedFootprintSide"].as_str().unwrap()
                    && spelled(ours.adjusted_frontal) == case["adjustedFootprintFrontal"].as_str().unwrap();
                let report = format!(
                    "{name}\n  qt:   {} {} {}\n  rust: {} {} {}",
                    case["imageDensity"].as_str().unwrap(),
                    case["adjustedFootprintSide"].as_str().unwrap(),
                    case["adjustedFootprintFrontal"].as_str().unwrap(),
                    spelled(ours.image_density),
                    spelled(ours.adjusted_side),
                    spelled(ours.adjusted_frontal)
                );
                (name.clone(), matched, report)
            })
            .collect();
        assert_eq!(checked.len(), 5, "every recorded camera case is checked");
        let wrong: Vec<&str> = checked.iter().filter(|(_, matched, _)| !matched).map(|(_, _, report)| report.as_str()).collect();
        assert!(wrong.is_empty(), "{} of {} cameras differ from the Qt calculation:\n{}", wrong.len(), checked.len(), wrong.join("\n"));
    }

    fn phantom() -> Camera {
        Camera { focal_length: 8.6, sensor_width: 13.2, sensor_height: 8.8, image_width: 5472.0, image_height: 3648.0, landscape: true, frontal_overlap: 70.0, side_overlap: 70.0 }
    }

    #[test]
    fn distance_and_density_are_each_computed_from_the_other() {
        let from_height = from_distance(&phantom(), 50.0).unwrap();
        let back = from_density(&phantom(), from_height.image_density).unwrap();
        assert!((back.distance_to_surface - 50.0).abs() < 1e-9, "asking for the density that a height gives, then the height that density needs, returns the height");
        assert!((back.adjusted_side - from_height.adjusted_side).abs() < 1e-9);
    }

    #[test]
    fn a_camera_missing_a_measurement_computes_nothing() {
        let cases = [
            Camera { focal_length: 0.0, ..phantom() },
            Camera { sensor_width: 0.0, ..phantom() },
            Camera { sensor_height: 0.0, ..phantom() },
            Camera { image_width: 0.0, ..phantom() },
            Camera { image_height: 0.0, ..phantom() },
        ];
        assert!(cases.iter().all(|camera| from_distance(camera, 50.0).is_none()), "a camera with a measurement missing answers nothing rather than a footprint built from a zero");
        assert!(from_distance(&phantom(), 0.0).is_none());
        assert!(from_density(&phantom(), 0.0).is_none());
        assert!(from_distance(&phantom(), 50.0).is_some());
    }

    #[test]
    fn turning_the_camera_swaps_which_way_the_picture_is_wide() {
        let landscape = from_distance(&phantom(), 50.0).unwrap();
        let portrait = from_distance(&Camera { landscape: false, ..phantom() }, 50.0).unwrap();
        assert_eq!(landscape.footprint_side, portrait.footprint_frontal);
        assert_eq!(landscape.footprint_frontal, portrait.footprint_side);
        assert!(landscape.footprint_side > landscape.footprint_frontal, "a landscape picture is wider across the flight path than along it");
    }

    #[test]
    fn overlap_narrows_the_spacing_the_generators_fly_to() {
        let none = from_distance(&Camera { side_overlap: 0.0, frontal_overlap: 0.0, ..phantom() }, 50.0).unwrap();
        assert_eq!(none.adjusted_side, none.footprint_side, "with no overlap the passes are a full picture apart");
        let half = from_distance(&Camera { side_overlap: 50.0, ..phantom() }, 50.0).unwrap();
        assert!((half.adjusted_side - none.footprint_side / 2.0).abs() < 1e-9);
        let total = from_distance(&Camera { side_overlap: 100.0, ..phantom() }, 50.0).unwrap();
        assert_eq!(total.adjusted_side, 0.0, "a hundred percent overlap leaves no spacing, which the generators read as one pass");
    }
}
