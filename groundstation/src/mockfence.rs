use mavlink::dialects::ardupilotmega::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Zone {
    Polygon { vertices: Vec<(f64, f64)>, inclusion: bool },
    Circle { centre: (f64, f64), radius: f64, inclusion: bool },
}

fn point(item: &MISSION_ITEM_INT_DATA) -> (f64, f64) {
    (f64::from(item.x) / 1e7, f64::from(item.y) / 1e7)
}

fn grouped(zones: Vec<Zone>, item: &MISSION_ITEM_INT_DATA) -> Vec<Zone> {
    let inclusion = matches!(item.command, MavCmd::MAV_CMD_NAV_FENCE_POLYGON_VERTEX_INCLUSION | MavCmd::MAV_CMD_NAV_FENCE_CIRCLE_INCLUSION);
    match item.command {
        MavCmd::MAV_CMD_NAV_FENCE_POLYGON_VERTEX_INCLUSION | MavCmd::MAV_CMD_NAV_FENCE_POLYGON_VERTEX_EXCLUSION => {
            let open = matches!(zones.last(), Some(Zone::Polygon { vertices, inclusion: same }) if *same == inclusion && vertices.len() < item.param1 as usize);
            let (kept, last) = match open {
                true => (zones[..zones.len() - 1].to_vec(), zones.last().cloned()),
                false => (zones, None),
            };
            let vertices = match last {
                Some(Zone::Polygon { vertices, .. }) => vertices.into_iter().chain([point(item)]).collect(),
                _ => vec![point(item)],
            };
            kept.into_iter().chain([Zone::Polygon { vertices, inclusion }]).collect()
        }
        MavCmd::MAV_CMD_NAV_FENCE_CIRCLE_INCLUSION | MavCmd::MAV_CMD_NAV_FENCE_CIRCLE_EXCLUSION => zones.into_iter().chain([Zone::Circle { centre: point(item), radius: f64::from(item.param1), inclusion }]).collect(),
        _ => zones,
    }
}

pub fn zones(items: &[MISSION_ITEM_INT_DATA]) -> Vec<Zone> {
    items.iter().fold(Vec::new(), grouped)
}

pub fn breached(zones: &[Zone], at: (f64, f64)) -> bool {
    zones.iter().any(|zone| match zone {
        Zone::Polygon { vertices, inclusion } => crate::mappolygon::contains(vertices, at) != *inclusion,
        Zone::Circle { centre, radius, inclusion } => (crate::terrain::qt_distance(*centre, at) <= *radius) != *inclusion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(command: MavCmd, count: f32, lat: f64, lon: f64) -> MISSION_ITEM_INT_DATA {
        MISSION_ITEM_INT_DATA { command, param1: count, x: (lat * 1e7) as i32, y: (lon * 1e7) as i32, ..Default::default() }
    }

    #[test]
    fn a_vehicle_must_stay_inside_every_inclusion_and_outside_every_exclusion() {
        let inclusion = MavCmd::MAV_CMD_NAV_FENCE_POLYGON_VERTEX_INCLUSION;
        let items = [
            vertex(inclusion, 4.0, 47.0, 8.0),
            vertex(inclusion, 4.0, 47.0, 8.01),
            vertex(inclusion, 4.0, 47.01, 8.01),
            vertex(inclusion, 4.0, 47.01, 8.0),
            vertex(MavCmd::MAV_CMD_NAV_FENCE_CIRCLE_EXCLUSION, 100.0, 47.005, 8.005),
        ];
        let zones = zones(&items);
        assert_eq!(zones.len(), 2, "four vertices make one polygon and the circle stands alone");
        assert!(!breached(&zones, (47.002, 8.002)), "inside the square, away from the no-fly circle");
        assert!(breached(&zones, (47.005, 8.0055)), "inside the exclusion circle");
        assert!(breached(&zones, (47.02, 8.005)), "outside the inclusion square");
    }
}
