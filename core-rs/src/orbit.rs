use serde_json::{Value, json};

use crate::read::{Unit, flag, format_measure, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.orbitActive",
    "vehicle.vehicleLinkManager.communicationLost",
    "settings.unitsSettings.horizontalDistanceUnits",
];

fn centre_of(circle: &Value) -> Option<Value> {
    let at = circle.get("center")?;
    let latitude = at.get("latitude")?.as_f64()?;
    let longitude = at.get("longitude")?.as_f64()?;
    Some(json!({ "latitude": latitude, "longitude": longitude }))
}

pub fn orbit_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "orbitActive"));
    if vehicle.get("kind").and_then(Value::as_str) != Some("object") {
        return json!({
            "kind": "object",
            "class": "Orbit",
            "available": false,
            "orbiting": Value::Null,
            "reason": "No vehicle is connected.",
        });
    }
    let lost = flag(&object(&backend.get_fields("vehicle.vehicleLinkManager", "communicationLost")), "communicationLost");
    let orbiting = (!lost).then(|| flag(&vehicle, "orbitActive"));
    let circle = object(&backend.get("vehicle.orbitMapCircle"));
    let unit = Unit::horizontal(backend);
    let radius = circle
        .get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| facts.iter().find(|f| f.get("property").and_then(Value::as_str) == Some("radius")))
        .and_then(|f| f.get("rawValue").or_else(|| f.get("value")))
        .and_then(Value::as_f64)
        .filter(|metres| metres.is_finite() && *metres > 0.0);
    let turning = orbiting == Some(true);
    json!({
        "kind": "object",
        "class": "Orbit",
        "available": true,
        "orbiting": orbiting,
        "centre": turning.then(|| centre_of(&circle)).flatten(),
        "radiusMetres": turning.then_some(radius).flatten(),
        "radiusText": turning.then_some(radius).flatten().map(|metres| format_measure(unit.show(metres), &unit.name)),
        "clockwise": turning.then(|| flag(&circle, "clockwiseRotation")),
        "reason": match orbiting {
            None => "No contact, so whether the vehicle is still orbiting is unknown.",
            Some(false) => "This vehicle is not orbiting.",
            Some(true) => "",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Flying {
        active: bool,
        lost: bool,
    }

    impl Backend for Flying {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.orbitMapCircle" => json!({
                    "kind": "object",
                    "center": { "latitude": 47.397, "longitude": 8.546, "valid": true },
                    "clockwiseRotation": false,
                    "facts": [ { "property": "radius", "value": 120.0, "rawValue": 120.0 } ],
                }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicle" => json!({ "kind": "object", "orbitActive": self.active }),
                "vehicle.vehicleLinkManager" => json!({ "kind": "object", "communicationLost": self.lost }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn an_orbit_that_stopped_reporting_is_not_an_orbit_that_stopped() {
        let turning = orbit_view(&Flying { active: true, lost: false }, &[]);
        assert_eq!(turning["orbiting"], true);
        assert_eq!(turning["radiusText"], "120 m");
        assert_eq!(turning["clockwise"], false, "ORBIT_EXECUTION_STATUS carries the direction in the SIGN of its radius, and the vehicle splits it into an absolute radius plus this flag - a head given only the radius has lost which way round the aircraft is turning");
        assert_eq!(turning["centre"]["latitude"], 47.397);

        let stopped = orbit_view(&Flying { active: false, lost: false }, &[]);
        assert_eq!(stopped["orbiting"], false);
        assert_eq!(stopped["centre"], Value::Null, "nothing is being flown, so there is no circle to draw");

        let quiet = orbit_view(&Flying { active: true, lost: true }, &[]);
        assert_eq!(quiet["orbiting"], Value::Null, "_orbitActive is set true by telemetry and cleared ONLY by a three second watchdog, so with contact lost false means 'no ORBIT_EXECUTION_STATUS arrived' rather than 'the vehicle stopped orbiting' - and the aircraft may still be turning");
        assert!(quiet["reason"].as_str().unwrap().contains("No contact"));
    }
}
