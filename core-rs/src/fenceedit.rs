use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

const FENCE_CONTROLLER: &str = "plan.geoFenceController";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Polygon,
    Circle,
}

impl Shape {
    fn list(self) -> &'static str {
        match self {
            Shape::Polygon => "plan.geoFenceController.polygons",
            Shape::Circle => "plan.geoFenceController.circles",
        }
    }

    fn word(self) -> &'static str {
        match self {
            Shape::Polygon => "polygon",
            Shape::Circle => "circle",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Fence {
    supported: Option<bool>,
    syncing: bool,
}

fn fence(backend: &dyn Backend) -> Fence {
    Fence {
        supported: crate::plan::capability(backend, "geoFenceController"),
        syncing: flag(&object(&backend.get_fields("plan", "syncInProgress")), "syncInProgress"),
    }
}

fn count(backend: &dyn Backend, shape: Shape) -> usize {
    object(&backend.get(shape.list())).get("elements").and_then(Value::as_array).map_or(0, Vec::len)
}

fn point(value: Option<&Value>) -> Option<(f64, f64)> {
    let value = value?;
    let latitude = value.get("latitude")?.as_f64().filter(|v| v.is_finite() && (-90.0..=90.0).contains(v))?;
    let longitude = value.get("longitude")?.as_f64().filter(|v| v.is_finite() && (-180.0..=180.0).contains(v))?;
    Some((latitude, longitude))
}

fn editing_refusal(state: Fence) -> Option<(&'static str, &'static str)> {
    match () {
        _ if state.supported == Some(false) => Some(("unsupported", "This link does not accept a geofence.")),
        _ if state.syncing => Some(("busy", "Wait for the sync to finish before changing the geofence.")),
        _ => None,
    }
}

fn window_refusal(top_left: Option<(f64, f64)>, bottom_right: Option<(f64, f64)>) -> Option<(&'static str, &'static str)> {
    let (Some((north, west)), Some((south, east))) = (top_left, bottom_right) else {
        return Some(("badCoordinate", "A new fence needs the map window's top-left and bottom-right corners."));
    };
    match north > south && east != west {
        true => None,
        false => Some(("emptyWindow", "The map window has no area to place a fence in, or its corners are the wrong way round.")),
    }
}

pub fn add(backend: &dyn Backend, shape: Shape, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let (top_left, bottom_right) = (point(given.get(0)), point(given.get(1)));
    if let Some((token, reason)) = editing_refusal(fence(backend)).or_else(|| window_refusal(top_left, bottom_right)) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let before = count(backend, shape);
    let corner = |(latitude, longitude): (f64, f64)| json!({ "latitude": latitude, "longitude": longitude });
    let dispatched = flag(&object(&backend.invoke(path, &json!([corner(top_left.unwrap_or_default()), corner(bottom_right.unwrap_or_default())]).to_string())), "ok");
    let added = dispatched && count(backend, shape) == before + 1;
    json!({
        "ok": added,
        "refusal": Value::Null,
        "index": added.then_some(before),
        "reason": match added { true => Value::Null, false => json!(format!("The fence {} was not added.", shape.word())) },
    })
}

pub fn delete(backend: &dyn Backend, shape: Shape, path: &str, args: &str) -> Value {
    let index = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_u64()).map(|i| i as usize);
    let before = count(backend, shape);
    let refusal = editing_refusal(fence(backend)).filter(|(token, _)| *token == "busy").map(|(t, r)| (t, r.to_string())).or_else(|| match index {
        None => Some(("malformed", format!("Name the fence {} by its position.", shape.word()))),
        Some(i) if i >= before => Some(("noSuchShape", format!("There is no fence {} at position {i}.", shape.word()))),
        Some(_) => None,
    });
    if let Some((token, reason)) = refusal {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let dispatched = flag(&object(&backend.invoke(path, &json!([index]).to_string())), "ok");
    let removed = dispatched && count(backend, shape) + 1 == before;
    json!({
        "ok": removed,
        "refusal": Value::Null,
        "reason": match removed { true => Value::Null, false => json!(format!("The fence {} is still there.", shape.word())) },
    })
}

pub fn write_breach_return(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let given = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned()).unwrap_or(Value::Null);
    let Some((latitude, longitude)) = point(Some(&given)) else {
        return json!({ "ok": false, "result": false, "refusal": "badCoordinate", "reason": "A breach return point needs a latitude from -90 to 90 and a longitude from -180 to 180." });
    };
    let altitude = given.get("altitude").and_then(Value::as_f64).filter(|a| a.is_finite());
    if let Some((token, reason)) = editing_refusal(fence(backend)).filter(|(token, _)| *token == "busy") {
        return json!({ "ok": false, "result": false, "refusal": token, "reason": reason });
    }
    let mut written = json!({ "latitude": latitude, "longitude": longitude });
    if let Some(altitude) = altitude {
        written["altitude"] = json!(altitude);
    }
    let answered = flag(&object(&backend.set(path, &json!({ "value": written }).to_string())), "ok");
    let held = crate::read::nested_coordinate_at(&object(&backend.get_fields(FENCE_CONTROLLER, "breachReturnPoint")), "breachReturnPoint");
    let took = answered && held.is_some_and(|(lat, lon)| (lat - latitude).abs() < 1e-9 && (lon - longitude).abs() < 1e-9);
    json!({
        "ok": took,
        "result": took,
        "refusal": Value::Null,
        "reason": match took { true => Value::Null, false => json!("The geofence did not keep that breach return point.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn a_fence_is_placed_only_in_a_window_with_area_on_a_link_that_takes_one() {
        let (nw, se) = (Some((47.5, 8.4)), Some((47.3, 8.6)));
        assert_eq!(window_refusal(nw, se), None);
        assert_eq!(window_refusal(se, nw).map(|r| r.0), Some("emptyWindow"), "addInclusionPolygon walks south and east from the top-left corner, so swapped corners place the fence outside the window the operator was looking at");
        assert_eq!(window_refusal(nw, nw).map(|r| r.0), Some("emptyWindow"), "a zero-area window makes a zero-size fence");
        assert_eq!(window_refusal(None, se).map(|r| r.0), Some("badCoordinate"));
        assert_eq!(point(Some(&json!({ "latitude": 91.0, "longitude": 8.0 }))), None);
        assert_eq!(editing_refusal(Fence { supported: Some(false), syncing: false }).map(|r| r.0), Some("unsupported"));
        assert_eq!(editing_refusal(Fence { supported: None, syncing: false }), None, "a vehicle that has not said what it accepts is offered a fence, as view.plan offers one");
        assert_eq!(editing_refusal(Fence { supported: Some(true), syncing: true }).map(|r| r.0), Some("busy"));
    }

    struct Controller {
        polygons: RefCell<usize>,
        obeys: bool,
        breach: RefCell<Value>,
    }

    impl Backend for Controller {
        fn get(&self, p: &str) -> String {
            let n = if p.ends_with("polygons") { *self.polygons.borrow() } else { 0 };
            json!({ "kind": "object", "elements": vec![json!({}); n] }).to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "plan.managerVehicle" => json!({ "kind": "object", "capabilitiesKnown": true }),
                FENCE_CONTROLLER => json!({ "kind": "object", "supported": true, "breachReturnPoint": self.breach.borrow().clone() }),
                _ => json!({ "kind": "object", "syncInProgress": false }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, v: &str) -> String {
            if self.obeys {
                let mut held = object(v)["value"].clone();
                held["valid"] = json!(true);
                *self.breach.borrow_mut() = held;
            }
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, p: &str, _a: &str) -> String {
            if self.obeys {
                let n = *self.polygons.borrow();
                *self.polygons.borrow_mut() = if p.ends_with("deletePolygon") { n - 1 } else { n + 1 };
            }
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_fence_change_is_confirmed_by_reading_it_back() {
        let controller = Controller { polygons: RefCell::new(1), obeys: true, breach: RefCell::new(Value::Null) };
        let window = r#"[{"latitude":47.5,"longitude":8.4},{"latitude":47.3,"longitude":8.6}]"#;
        assert_eq!(add(&controller, Shape::Polygon, "plan.geoFenceController.addInclusionPolygon", window)["index"], 1);
        assert_eq!(delete(&controller, Shape::Polygon, "plan.geoFenceController.deletePolygon", "[5]")["refusal"], "noSuchShape", "deletePolygon returns on a bad index and says nothing");
        assert_eq!(delete(&controller, Shape::Polygon, "plan.geoFenceController.deletePolygon", "[0]")["ok"], true);
        assert_eq!(*controller.polygons.borrow(), 1);
        let written = write_breach_return(&controller, "plan.geoFenceController.breachReturnPoint", r#"{"value":{"latitude":47.4,"longitude":8.5,"altitude":60}}"#);
        assert_eq!((&written["ok"], &written["result"]), (&json!(true), &json!(true)));
        assert_eq!(write_breach_return(&controller, "plan.geoFenceController.breachReturnPoint", r#"{"value":{"latitude":147.4,"longitude":8.5}}"#)["refusal"], "badCoordinate", "setBreachReturnPoint stores whatever coordinate it is given and marks the plan dirty for it");

        let deaf = Controller { polygons: RefCell::new(1), obeys: false, breach: RefCell::new(Value::Null) };
        assert_eq!(add(&deaf, Shape::Polygon, "plan.geoFenceController.addInclusionPolygon", window)["ok"], false);
        assert_eq!(write_breach_return(&deaf, "plan.geoFenceController.breachReturnPoint", r#"{"value":{"latitude":47.4,"longitude":8.5}}"#)["ok"], false);
    }
}
