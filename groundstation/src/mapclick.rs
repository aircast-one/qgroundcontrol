use serde_json::{Value, json};

use crate::read::{flag, integer, object};
use crate::router::Backend;

// A click on the fly map sends one of five vehicle commands with the clicked point. Vehicle checks
// little of it: guidedModeROI and guidedModeChangeHeading log and do nothing when the vehicle does
// not support them, while the bridge answers ok; a goto, a heading or an ROI reaches a vehicle on
// the ground; and setEstimatorOrigin overrides the position of a vehicle whose GPS already knows it.
// The rules are the ones MapClickModel.swift applied before offering each action.
const GPS_SENSOR_BIT: i64 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    GoTo,
    Roi,
    SetHome,
    Heading,
    EstimatorOrigin,
}

impl Click {
    pub fn of(path: &str) -> Option<Click> {
        match path {
            "vehicle.guidedModeGotoLocation" => Some(Click::GoTo),
            "vehicle.guidedModeROI" => Some(Click::Roi),
            "vehicle.doSetHome" => Some(Click::SetHome),
            "vehicle.guidedModeChangeHeading" => Some(Click::Heading),
            "vehicle.setEstimatorOrigin" => Some(Click::EstimatorOrigin),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Aircraft {
    connected: bool,
    flying: bool,
    roi: bool,
    heading: bool,
    gps: bool,
}

fn aircraft(backend: &dyn Backend) -> Aircraft {
    let vehicle = object(&backend.get_fields("vehicle", "flying,sensorsPresentBits"));
    let supports = object(&backend.get_fields("vehicle.supports", "roiMode,changeHeading"));
    Aircraft {
        connected: vehicle.get("kind").and_then(Value::as_str) == Some("object"),
        flying: flag(&vehicle, "flying"),
        roi: flag(&supports, "roiMode"),
        heading: flag(&supports, "changeHeading"),
        gps: integer(&vehicle, "sensorsPresentBits").unwrap_or(GPS_SENSOR_BIT) & GPS_SENSOR_BIT != 0,
    }
}

fn click_refusal(click: Click, a: Aircraft) -> Option<(&'static str, &'static str)> {
    match click {
        _ if !a.connected => Some(("noVehicle", "No vehicle is connected.")),
        Click::GoTo | Click::Roi | Click::Heading if !a.flying => Some(("grounded", "The vehicle has to be flying for that.")),
        Click::Roi if !a.roi => Some(("unsupported", "This vehicle cannot point at a location.")),
        Click::Heading if !a.heading => Some(("unsupported", "This vehicle cannot be turned to face a point.")),
        Click::EstimatorOrigin if a.gps => Some(("hasGps", "The vehicle has GPS, so it already knows where it is.")),
        _ => None,
    }
}

pub fn send(backend: &dyn Backend, click: Click, path: &str, args: &str) -> Value {
    let refused = |token: &str, reason: &str| json!({ "ok": false, "refusal": token, "reason": reason });
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let Some((latitude, longitude)) = crate::fenceedit::point(given.get(0)) else {
        return refused("badCoordinate", "The point needs a latitude from -90 to 90 and a longitude from -180 to 180.");
    };
    let mut at = json!({ "latitude": latitude, "longitude": longitude });
    if let Some(altitude) = given[0].get("altitude").and_then(Value::as_f64).filter(|a| a.is_finite()) {
        at["altitude"] = json!(altitude);
    }
    let forwarded = match click {
        Click::GoTo => match given.get(1).map(Value::as_f64) {
            None => json!([at, 0.0]),
            Some(Some(radius)) if radius.is_finite() && radius >= 0.0 => json!([at, radius]),
            Some(_) => return refused("badRadius", "A loiter radius is zero or more metres."),
        },
        _ => json!([at]),
    };
    if let Some((token, reason)) = click_refusal(click, aircraft(backend)) {
        return refused(token, reason);
    }
    let dispatched = flag(&object(&backend.invoke(path, &forwarded.to_string())), "ok");
    json!({ "ok": dispatched, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => json!("The vehicle was not sent the command.") } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn each_map_click_is_sent_only_where_the_head_offered_it() {
        let flying = Aircraft { connected: true, flying: true, roi: true, heading: true, gps: true };
        assert_eq!(click_refusal(Click::GoTo, flying), None);
        assert_eq!(click_refusal(Click::GoTo, Aircraft { flying: false, ..flying }).map(|r| r.0), Some("grounded"));
        assert_eq!(click_refusal(Click::Roi, Aircraft { roi: false, ..flying }).map(|r| r.0), Some("unsupported"), "Vehicle::guidedModeROI logs and returns while the bridge answers ok");
        assert_eq!(click_refusal(Click::Heading, Aircraft { heading: false, ..flying }).map(|r| r.0), Some("unsupported"));
        assert_eq!(click_refusal(Click::SetHome, Aircraft { flying: false, ..flying }), None, "home can be set on the ground");
        assert_eq!(click_refusal(Click::EstimatorOrigin, flying).map(|r| r.0), Some("hasGps"));
        assert_eq!(click_refusal(Click::EstimatorOrigin, Aircraft { gps: false, flying: false, ..flying }), None);
        assert_eq!(click_refusal(Click::SetHome, Aircraft::default()).map(|r| r.0), Some("noVehicle"));
        assert_eq!(Click::of("vehicle.guidedModeOrbit"), None, "the orbit is guided.orbit's, with a radius and altitude the operator chose");
    }

    struct Vehicle(RefCell<Vec<String>>);
    impl Backend for Vehicle {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "vehicle" => json!({ "kind": "object", "flying": true, "sensorsPresentBits": 32 }),
                _ => json!({ "kind": "object", "roiMode": true, "changeHeading": true }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, a: &str) -> String {
            self.0.borrow_mut().push(a.to_string());
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_point_and_radius_are_checked_before_anything_is_sent() {
        let vehicle = Vehicle(RefCell::new(Vec::new()));
        assert_eq!(send(&vehicle, Click::GoTo, "vehicle.guidedModeGotoLocation", r#"[{"latitude":47.4,"longitude":8.5,"altitude":0},25]"#)["ok"], true);
        assert_eq!(send(&vehicle, Click::GoTo, "vehicle.guidedModeGotoLocation", r#"[{"latitude":47.4,"longitude":8.5},-5]"#)["refusal"], "badRadius");
        assert_eq!(send(&vehicle, Click::Roi, "vehicle.guidedModeROI", r#"[{"latitude":99,"longitude":8.5}]"#)["refusal"], "badCoordinate");
        assert_eq!(send(&vehicle, Click::EstimatorOrigin, "vehicle.setEstimatorOrigin", r#"[{"latitude":47.4,"longitude":8.5}]"#)["refusal"], "hasGps");
        assert_eq!(vehicle.0.borrow().as_slice(), &[r#"[{"altitude":0.0,"latitude":47.4,"longitude":8.5},25.0]"#.to_string()]);
    }
}
