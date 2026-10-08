use serde_json::{Value, json};

use crate::read::{flag, integer};
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
    HomeRoi,
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

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.flying",
    "vehicle.armed",
    "vehicle.flightMode",
    "vehicle.sensorsPresentBits",
    "settings.flyViewSettings.goToLocationRequiresConfirmInGuided",
    "vehicle.homePosition",
    "vehicle.landing",
    "settings.unitsSettings.horizontalDistanceUnits",
    "vehicle.px4Firmware",
    "vehicle.orbitActive",
    "vehicle.fixedWing",
    "vehicle.vtolInFwdFlight",
];

const ORBIT_DEFAULT_RADIUS_METRES: f64 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct GotoMark {
    latitude: f64,
    longitude: f64,
    radius: f64,
}

struct GotoState {
    mark: Option<GotoMark>,
    in_goto_mode: bool,
}

static LAST_GOTO: std::sync::Mutex<GotoState> = std::sync::Mutex::new(GotoState { mark: None, in_goto_mode: false });

fn goto_state() -> std::sync::MutexGuard<'static, GotoState> {
    LAST_GOTO.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn remember_goto(mark: GotoMark) {
    goto_state().mark = Some(mark);
}

fn goto_shown(in_goto_mode: bool) -> Option<GotoMark> {
    let mut state = goto_state();
    if state.in_goto_mode && !in_goto_mode {
        state.mark = None;
    }
    state.in_goto_mode = in_goto_mode;
    state.mark
}

fn goto_vehicle(backend: &dyn Backend) -> (Value, bool) {
    let vehicle = backend.value_fields("vehicle", "armed,flying,px4Firmware,orbitActive,flightMode,gotoFlightMode");
    let in_goto_mode = vehicle.get("flightMode").is_some() && vehicle.get("flightMode") == vehicle.get("gotoFlightMode");
    (vehicle, in_goto_mode)
}

fn loiter_circle_shown(backend: &dyn Backend, vehicle: &Value) -> bool {
    !flag(vehicle, "px4Firmware") && !flag(vehicle, "orbitActive") && crate::guided::forward_flight(backend)
}

fn loiter_offer(backend: &dyn Backend) -> Option<GotoMark> {
    let (vehicle, in_goto_mode) = goto_vehicle(backend);
    let mark = goto_shown(in_goto_mode).filter(|_| in_goto_mode)?;
    let guided = flag(&backend.value_fields("vehicle.supports", "guidedMode"), "guidedMode");
    let shown = flag(&vehicle, "armed") && flag(&vehicle, "flying") && guided && loiter_circle_shown(backend, &vehicle) && !crate::guided::mission_active(backend);
    shown.then_some(mark)
}

fn goto_location(backend: &dyn Backend) -> Option<Value> {
    let (vehicle, in_goto_mode) = goto_vehicle(backend);
    goto_shown(in_goto_mode).map(|mark| {
        let radius = loiter_circle_shown(backend, &vehicle).then_some(mark.radius.abs());
        let unit = crate::read::Unit::horizontal(backend);
        json!({
            "latitude": mark.latitude,
            "longitude": mark.longitude,
            "loiterRadiusMetres": radius,
            "loiterRadiusText": radius.map(|metres| format!("{:.0} {}", unit.show(metres), unit.name)),
            "loiterClockwise": radius.map(|_| mark.radius >= 0.0),
        })
    })
}

struct Offer {
    id: &'static str,
    path: &'static str,
    label: &'static str,
    title: &'static str,
    message: &'static str,
    confirm: bool,
}

fn offers(backend: &dyn Backend) -> Vec<Offer> {
    let vehicle = backend.value_fields("vehicle", "flying,sensorsPresentBits,flightMode,gotoFlightMode");
    let supports = backend.value_fields("vehicle.supports", "roiMode,orbitMode");
    if vehicle.get("kind").and_then(Value::as_str) != Some("object") {
        return vec![];
    }
    let flying = flag(&vehicle, "flying");
    let gps = integer(&vehicle, "sensorsPresentBits").unwrap_or(GPS_SENSOR_BIT) & GPS_SENSOR_BIT != 0;
    let in_goto_mode = vehicle.get("flightMode").is_some() && vehicle.get("flightMode") == vehicle.get("gotoFlightMode");
    let confirm_in_guided = crate::read::value_number(&backend.value("settings.flyViewSettings.goToLocationRequiresConfirmInGuided.rawValue")).is_none_or(|v| v != 0.0);
    let home = backend.value("vehicle.homePosition");
    let home_known = flag(&home, "valid") && home.get("altitude").and_then(Value::as_f64).is_some_and(f64::is_finite);
    let orbit = flying && flag(&supports, "orbitMode") && home_known && !crate::guided::mission_active(backend);
    [
        (flying, Offer { id: "GoTo", path: "vehicle.guidedModeGotoLocation", label: "Go to location", title: "Go To Location", message: "Move the vehicle to the specified location", confirm: !in_goto_mode || confirm_in_guided }),
        (orbit, Offer { id: "Orbit", path: "guided.orbit", label: "Orbit at location", title: "Orbit", message: "Orbit the vehicle around the specified location", confirm: true }),
        (flying && flag(&supports, "roiMode"), Offer { id: "Roi", path: "vehicle.guidedModeROI", label: "ROI at location", title: "ROI", message: "Make the specified location a Region Of Interest", confirm: false }),
        (true, Offer { id: "SetHome", path: "vehicle.doSetHome", label: "Set home here", title: "Set Home", message: "Set vehicle home as the specified location. This will affect Return to Home position", confirm: true }),
        (flying, Offer { id: "Heading", path: "vehicle.guidedModeChangeHeading", label: "Set Heading", title: "Change Heading", message: "Set the vehicle heading towards the specified location", confirm: true }),
        (!gps, Offer { id: "EstimatorOrigin", path: "vehicle.setEstimatorOrigin", label: "Set Estimator Origin", title: "Set Estimator Origin", message: "Make the specified location the estimator origin", confirm: true }),
    ]
    .into_iter()
    .filter_map(|(shown, offer)| shown.then_some(offer))
    .collect()
}

pub fn map_click_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let listed = offers(backend);
    let unit = crate::read::Unit::horizontal(backend);
    json!({
        "kind": "object",
        "class": "MapClick",
        "offered": !listed.is_empty(),
        "actions": listed.iter().map(|o| json!({
            "id": o.id,
            "path": o.path,
            "label": o.label,
            "title": o.title,
            "message": o.message,
            "confirm": o.confirm,
        })).collect::<Vec<_>>(),
        "loiter": loiter_offer(backend).map(|mark| json!({
            "latitude": mark.latitude,
            "longitude": mark.longitude,
            "title": "Change Loiter Radius",
            "message": "Change the forward flight loiter radius",
            "defaultRadius": unit.show(mark.radius.abs()),
            "clockwise": mark.radius >= 0.0,
        })),
        "gotoLocation": goto_location(backend),
        "orbitDefaultRadius": unit.show(ORBIT_DEFAULT_RADIUS_METRES),
        "orbitRadiusUnit": unit.name,
        "orbitMetresPerUnit": 1.0 / unit.factor,
        "orbitClockwise": true,
    })
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
    let vehicle = backend.value_fields("vehicle", "flying,sensorsPresentBits");
    let supports = backend.value_fields("vehicle.supports", "roiMode,changeHeading");
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
        Click::Roi | Click::HomeRoi if !a.roi => Some(("unsupported", "This vehicle cannot point at a location.")),
        Click::Heading if !a.heading => Some(("unsupported", "This vehicle cannot be turned to face a point.")),
        Click::EstimatorOrigin if a.gps => Some(("hasGps", "The vehicle has GPS, so it already knows where it is.")),
        _ => None,
    }
}

fn roi_action(backend: &dyn Backend, latitude: f64, longitude: f64, altitude: f64) -> Value {
    match flag(&backend.value_fields("vehicle", "px4Firmware"), "px4Firmware") {
        true => {
            let home = backend.value("vehicle.homePosition").get("altitude").and_then(Value::as_f64).unwrap_or(f64::NAN);
            let terrain = crate::terrainservice::height_now(latitude, longitude).ok().flatten().unwrap_or(home);
            json!({ "action": "roi", "latitude": latitude, "longitude": longitude, "altitude": terrain, "frame": crate::guidedcmd::FRAME_GLOBAL })
        }
        false => json!({ "action": "roi", "latitude": latitude, "longitude": longitude, "altitude": altitude, "frame": crate::guidedcmd::FRAME_GLOBAL_RELATIVE_ALT }),
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
            None => json!([at, crate::guided::goto_loiter_radius(backend)]),
            Some(Some(radius)) if radius.is_finite() => json!([at, radius]),
            Some(_) => return refused("badRadius", "A loiter radius is a number of metres; its sign is the direction."),
        },
        _ => json!([at]),
    };
    if let Some((token, reason)) = click_refusal(click, aircraft(backend)) {
        return refused(token, reason);
    }
    let core = match click {
        Click::GoTo => Some(json!({ "action": "goto", "latitude": latitude, "longitude": longitude, "loiterRadius": forwarded[1] })),
        Click::EstimatorOrigin => Some(json!({ "action": "estimatorOrigin", "latitude": latitude, "longitude": longitude, "altitude": at.get("altitude").cloned().unwrap_or(json!(0.0)) })),
        _ if crate::qthost::present() => None,
        Click::Heading => Some(json!({ "action": "heading", "latitude": latitude, "longitude": longitude })),
        Click::SetHome => Some(json!({ "action": "setHome", "latitude": latitude, "longitude": longitude, "terrain": crate::terrainservice::height_now(latitude, longitude).ok().flatten() })),
        Click::Roi | Click::HomeRoi => Some(roi_action(backend, latitude, longitude, at.get("altitude").and_then(Value::as_f64).unwrap_or(0.0))),
    };
    let sent = crate::guided::dispatch(backend, core, crate::guided::active_id(backend), path, &forwarded.to_string());
    if click == Click::GoTo && flag(&sent, "ok") {
        remember_goto(GotoMark { latitude, longitude, radius: forwarded[1].as_f64().unwrap_or(0.0) });
    }
    sent
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
        assert_eq!(click_refusal(Click::HomeRoi, Aircraft { flying: false, ..flying }), None, "GimbalIndicator's Point Home calls guidedModeROI(homePosition) on the ground too");
        assert_eq!(click_refusal(Click::Heading, Aircraft { heading: false, ..flying }).map(|r| r.0), Some("unsupported"));
        assert_eq!(click_refusal(Click::SetHome, Aircraft { flying: false, ..flying }), None, "home can be set on the ground");
        assert_eq!(click_refusal(Click::EstimatorOrigin, flying).map(|r| r.0), Some("hasGps"));
        assert_eq!(click_refusal(Click::EstimatorOrigin, Aircraft { gps: false, flying: false, ..flying }), None);
        assert_eq!(click_refusal(Click::SetHome, Aircraft::default()).map(|r| r.0), Some("noVehicle"));
        assert_eq!(Click::of("vehicle.guidedModeOrbit"), None, "the orbit is guided.orbit's, with a radius and altitude the operator chose");
    }

    struct Offered { flying: bool, sensors: i64, mode: &'static str, roi: bool, confirm_in_guided: f64, orbit: bool }
    impl Backend for Offered {
        fn get(&self, p: &str) -> String {
            match p {
                "settings.flyViewSettings.goToLocationRequiresConfirmInGuided.rawValue" => json!({ "value": self.confirm_in_guided }).to_string(),
                "vehicle.homePosition" => json!({ "kind": "object", "valid": true, "altitude": 480.0 }).to_string(),
                _ => self.get_fields(p, ""),
            }
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "vehicle" => json!({ "kind": "object", "flying": self.flying, "sensorsPresentBits": self.sensors, "flightMode": self.mode, "gotoFlightMode": "Guided" }),
                "vehicle.supports" => json!({ "kind": "object", "roiMode": self.roi, "orbitMode": self.orbit }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn the_menu_offers_what_qgc_offers() {
        let _serial = SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let labels = |b: &Offered| map_click_view(b, &[])["actions"].as_array().unwrap().iter().map(|a| a["label"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        let flying = Offered { flying: true, sensors: 32, mode: "Loiter", roi: true, confirm_in_guided: 0.0, orbit: false };
        assert_eq!(labels(&flying), ["Go to location", "ROI at location", "Set home here", "Set Heading"]);
        assert_eq!(labels(&Offered { flying: false, ..flying }), ["Set home here"], "on the ground only home can be moved");
        assert_eq!(labels(&Offered { flying: false, sensors: 0, ..flying }), ["Set home here", "Set Estimator Origin"], "a vehicle without GPS is offered an origin");
        let actions = |b: &Offered| map_click_view(b, &[])["actions"].clone();
        assert_eq!(actions(&flying)[0]["confirm"], true, "outside the goto mode a goto is confirmed");
        assert_eq!(actions(&Offered { mode: "Guided", ..flying })[0]["confirm"], false, "already in Guided the goto is sent at once unless the setting asks for a confirmation");
        assert_eq!(actions(&Offered { mode: "Guided", confirm_in_guided: 1.0, ..flying })[0]["confirm"], true);
        assert_eq!(actions(&flying)[1]["confirm"], false, "QGC sends an ROI without asking");
        assert_eq!(labels(&Offered { orbit: true, ..flying })[1], "Orbit at location", "an orbit follows the goto when the vehicle supports it and home carries an altitude");
        assert_eq!(map_click_view(&flying, &[])["orbitDefaultRadius"], 30.0);
        assert_eq!(map_click_view(&Vehicle(RefCell::new(vec![])), &[])["offered"], true);
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
        let _serial = SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let vehicle = Vehicle(RefCell::new(Vec::new()));
        assert_eq!(send(&vehicle, Click::GoTo, "vehicle.guidedModeGotoLocation", r#"[{"latitude":47.4,"longitude":8.5,"altitude":0},25]"#)["ok"], true);
        assert_eq!(send(&vehicle, Click::GoTo, "vehicle.guidedModeGotoLocation", r#"[{"latitude":47.4,"longitude":8.5},"wide"]"#)["refusal"], "badRadius");
        assert_eq!(send(&vehicle, Click::Roi, "vehicle.guidedModeROI", r#"[{"latitude":99,"longitude":8.5}]"#)["refusal"], "badCoordinate");
        assert_eq!(send(&vehicle, Click::EstimatorOrigin, "vehicle.setEstimatorOrigin", r#"[{"latitude":47.4,"longitude":8.5}]"#)["refusal"], "hasGps");
        assert_eq!(vehicle.0.borrow().as_slice(), &[r#"[{"altitude":0.0,"latitude":47.4,"longitude":8.5},25.0]"#.to_string()]);
        assert_eq!(goto_state().mark, Some(GotoMark { latitude: 47.4, longitude: 8.5, radius: 25.0 }), "a goto that went out is the point a loiter radius change re-sends");
        assert_eq!(send(&vehicle, Click::GoTo, "vehicle.guidedModeGotoLocation", r#"[{"latitude":47.4,"longitude":8.5},-40]"#)["ok"], true, "the sign of the radius is the loiter direction, as QGC sends it");
        assert_eq!(goto_state().mark.map(|m| m.radius), Some(-40.0), "a loiter radius change is the radius the circle now draws");
    }

    struct Plane;
    impl Backend for Plane {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }),
                "vehicle" => json!({ "kind": "object", "armed": true, "flying": true, "fixedWing": true, "flightMode": "Guided", "gotoFlightMode": "Guided" }),
                "vehicle.supports" => json!({ "kind": "object", "guidedMode": true }),
                _ => json!({ "kind": "object" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_loiter_circle_keeps_the_committed_radius_and_direction() {
        let _serial = SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        goto_shown(false);
        remember_goto(GotoMark { latitude: 47.4, longitude: 8.5, radius: -120.0 });
        let view = map_click_view(&Plane, &[]);
        assert_eq!(view["gotoLocation"]["loiterRadiusMetres"], 120.0);
        assert_eq!(view["gotoLocation"]["loiterClockwise"], false, "QGCMapCircleVisuals points its rotation arrows the way the circle was committed");
        assert_eq!(view["loiter"]["defaultRadius"], 120.0, "the radius edit starts from the committed circle, not the setting");
        assert_eq!(view["loiter"]["clockwise"], false);
        remember_goto(GotoMark { latitude: 47.4, longitude: 8.5, radius: 80.0 });
        assert_eq!(map_click_view(&Plane, &[])["gotoLocation"]["loiterClockwise"], true);
        goto_shown(false);
    }

    #[test]
    fn the_go_here_marker_stays_until_the_vehicle_leaves_the_goto_mode() {
        let _serial = SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mark = GotoMark { latitude: 47.4, longitude: 8.5, radius: 0.0 };
        goto_shown(false);
        remember_goto(mark);
        assert_eq!(goto_shown(false), Some(mark), "a goto sent from Loiter shows before the vehicle has switched to Guided");
        assert_eq!(goto_shown(true), Some(mark));
        assert_eq!(goto_shown(false), None, "onInGotoFlightModeChanged hides the item when the goto mode is left");
        assert_eq!(goto_shown(true), None);
    }
}
