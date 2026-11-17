use serde_json::{Value, json};

use crate::read::{flag, integer};
use crate::router::Backend;

const GPS_SENSOR_BIT: i64 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    GoTo,
    Roi,
    HomeRoi,
    SetHome,
    Heading,
    EstimatorOrigin,
    MarkTarget,
}

impl Click {
    pub fn of(path: &str) -> Option<Click> {
        match path {
            "vehicle.guidedModeGotoLocation" => Some(Click::GoTo),
            "vehicle.guidedModeROI" => Some(Click::Roi),
            "vehicle.doSetHome" => Some(Click::SetHome),
            "vehicle.guidedModeChangeHeading" => Some(Click::Heading),
            "vehicle.setEstimatorOrigin" => Some(Click::EstimatorOrigin),
            "vehicle.markTarget" => Some(Click::MarkTarget),
            _ => None,
        }
    }
}

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.flying",
    "vehicle.rover",
    "vehicle.armed",
    "vehicle.flightMode",
    "vehicle.sensorsPresentBits",
    "vehicle.isROIEnabled",
    "settings.flyViewSettings.goToLocationRequiresConfirmInGuided",
    "vehicle.homePosition",
    "vehicle.landing",
    "settings.unitsSettings.horizontalDistanceUnits",
    "vehicle.px4Firmware",
    "vehicle.orbitActive",
    "vehicle.fixedWing",
    "vehicle.vtolInFwdFlight",
    "vehicle.coordinate",
    "vehicle.groundSpeed",
    "vehicle.altitudeRelative",
    "settings.unitsSettings.verticalDistanceUnits",
];

const ORBIT_DEFAULT_RADIUS_METRES: f64 = 30.0;
const ORBIT_RADIUS_METRES: (f64, f64, f64) = (10.0, 500.0, 10.0);
const ARRIVED_METRES: f64 = 5.0;
const SETTLED_MPS: f64 = 0.5;
const PERCENT: f64 = 100.0;

pub const PREVIEW_DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.coordinate",
    "vehicle.homePosition",
    "vehicle.altitudeRelative",
    "vehicle.distanceToHome",
    "vehicle.flying",
    "vehicle.batteries.count",
    "settings.unitsSettings.horizontalDistanceUnits",
    "settings.unitsSettings.verticalDistanceUnits",
];

fn place(backend: &dyn Backend, path: &str) -> Option<(f64, f64)> {
    let at = backend.value(path);
    let valid = at.get("valid").and_then(Value::as_bool).unwrap_or(true);
    Some((
        at.get("latitude")?.as_f64()?,
        at.get("longitude")?.as_f64()?,
    ))
    .filter(|(lat, lon)| valid && lat.is_finite() && lon.is_finite() && (*lat, *lon) != (0.0, 0.0))
}

fn raw(backend: &dyn Backend, path: &str) -> Option<f64> {
    crate::read::value_number(&backend.value(&format!("{path}.rawValue")))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GotoBudget {
    pub landing_percent: f64,
    pub past_return_point: bool,
}

pub fn goto_budget(
    percent: f64,
    seconds_left: f64,
    reserve: f64,
    there_seconds: f64,
    home_seconds: f64,
) -> Option<GotoBudget> {
    (percent > 0.0 && seconds_left > 0.0).then(|| {
        let landing_percent =
            (percent - (there_seconds + home_seconds) * percent / seconds_left).max(0.0);
        GotoBudget {
            landing_percent,
            past_return_point: landing_percent <= reserve,
        }
    })
}

fn left_on_landing(budget: GotoBudget) -> String {
    format!(
        "About {}% left on landing",
        budget.landing_percent.round() as i64
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GotoSpeeds {
    pub across_mps: f64,
    pub up_mps: f64,
    pub down_mps: f64,
}

fn goto_speeds(backend: &dyn Backend) -> Option<GotoSpeeds> {
    let si = |names: &[&str]| crate::battery::parameter_si(backend, names);
    let speeds = |across: &[&str], up: &[&str], down: &[&str]| {
        Some(GotoSpeeds {
            across_mps: si(across)?,
            up_mps: si(up)?,
            down_mps: si(down)?,
        })
    };
    speeds(
        &["MPC_XY_CRUISE"],
        &["MPC_Z_V_AUTO_UP"],
        &["MPC_Z_V_AUTO_DN"],
    )
    .or_else(|| {
        speeds(
            &["WP_SPD", "WPNAV_SPEED"],
            &["WP_SPD_UP", "WPNAV_SPEED_UP"],
            &["WP_SPD_DN", "WPNAV_SPEED_DN"],
        )
    })
}

pub fn goto_seconds(across_m: f64, climb_m: f64, speeds: GotoSpeeds) -> Option<f64> {
    let vertical_mps = if climb_m < 0.0 {
        speeds.down_mps
    } else {
        speeds.up_mps
    };
    [speeds.across_mps, vertical_mps]
        .iter()
        .all(|s| s.is_finite() && *s > 0.0)
        .then(|| (across_m / speeds.across_mps).max(climb_m.abs() / vertical_mps))
        .filter(|s| s.is_finite())
}

pub fn about_text(seconds: f64) -> String {
    match seconds < 59.5 {
        true => format!("about {} sec", seconds.round().max(1.0) as i64),
        false => format!("about {} min", (seconds / 60.0).round() as i64),
    }
}

pub fn goto_verdict(budget: GotoBudget) -> String {
    match budget.past_return_point {
        true => "Past your return point. Return home instead?".to_string(),
        false => left_on_landing(budget),
    }
}

pub fn goto_progress(
    remaining_m: f64,
    ground_speed: f64,
    distance: &str,
    height: &str,
) -> (bool, String) {
    let arrived = remaining_m <= ARRIVED_METRES && ground_speed < SETTLED_MPS;
    let eta = (ground_speed >= SETTLED_MPS)
        .then(|| crate::battery::duration_text(remaining_m / ground_speed))
        .filter(|t| !t.is_empty());
    let text = match arrived {
        true => format!("Arrived \u{00b7} holding at {height}"),
        false => std::iter::once(format!("Going to point \u{00b7} {distance}"))
            .chain(eta)
            .collect::<Vec<_>>()
            .join(" \u{00b7} "),
    };
    (arrived, text)
}

pub fn goto_preview_view(backend: &dyn Backend, args: &[String]) -> Value {
    let target = args.first().zip(args.get(1)).and_then(|(lat, lon)| {
        Some((
            lat.trim().parse::<f64>().ok()?,
            lon.trim().parse::<f64>().ok()?,
        ))
    });
    let (Some(target), Some(aircraft)) = (target, place(backend, "vehicle.coordinate")) else {
        return json!({ "kind": "object", "class": "GotoPreview", "available": false });
    };
    let (horizontal, vertical) = (
        crate::read::Unit::horizontal(backend),
        crate::read::Unit::vertical(backend),
    );
    let height = raw(backend, "vehicle.altitudeRelative");
    let across = crate::track::distance_m(aircraft, target);
    let chosen = args.get(2).and_then(|h| h.trim().parse::<f64>().ok());
    let climb = height.map_or(0.0, |now| chosen.unwrap_or(now) - now);
    let eta = goto_speeds(backend).and_then(|speeds| goto_seconds(across, climb, speeds));
    let headline = crate::battery::battery_view(backend, &[])["headline"].clone();
    let trip = crate::battery::return_trip(backend);
    let budget = (|| {
        let trip = trip?;
        let home = place(backend, "vehicle.homePosition")?;
        let there = across / trip.cruise_mps;
        let back = crate::battery::return_seconds(crate::battery::ReturnTrip {
            distance_m: crate::track::distance_m(target, home),
            ..trip
        })?;
        goto_budget(
            headline["percent"].as_f64()?,
            headline["secondsLeft"].as_f64()?,
            headline["reserve"].as_f64().unwrap_or(0.0),
            there,
            back,
        )
    })();
    json!({
        "kind": "object",
        "class": "GotoPreview",
        "available": true,
        "heightText": height.map(|h| format!("at {:.0} {} (current height)", vertical.show(h), vertical.name)),
        "heightMetres": height,
        "heightUnit": vertical.name,
        "metresPerHeightUnit": 1.0 / vertical.factor,
        "pastReturnPoint": budget.is_some_and(|b| b.past_return_point),
        "verdict": budget.map(goto_verdict),
        "distanceText": format!("{:.0} {}", horizontal.show(across), horizontal.name),
        "etaText": eta.map(about_text),
    })
}

pub fn rtl_verdict(budget: GotoBudget) -> String {
    match budget.past_return_point {
        true => format!("{}, under the reserve", left_on_landing(budget)),
        false => left_on_landing(budget),
    }
}

pub fn rtl_preview_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let Some(distance) = raw(backend, "vehicle.distanceToHome").filter(|d| d.is_finite()) else {
        return json!({ "kind": "object", "class": "RtlPreview", "available": false });
    };
    let (horizontal, vertical) = (
        crate::read::Unit::horizontal(backend),
        crate::read::Unit::vertical(backend),
    );
    let trip = crate::battery::return_trip(backend);
    let headline = crate::battery::battery_view(backend, &[])["headline"].clone();
    let budget = trip.and_then(|trip| {
        goto_budget(
            headline["percent"].as_f64()?,
            headline["secondsLeft"].as_f64()?,
            headline["reserve"].as_f64().unwrap_or(0.0),
            0.0,
            crate::battery::return_seconds(trip)?,
        )
    });
    let flown_at = trip.map(|t| t.flown_at_m());
    json!({
        "kind": "object",
        "class": "RtlPreview",
        "available": true,
        "distanceText": format!("{:.0} {}", horizontal.show(distance), horizontal.name),
        "returnAltitudeText": flown_at.map(|metres| format!("{:.0} {}", vertical.show(metres), vertical.name)),
        "climbs": trip.map(|t| t.return_altitude_m > t.altitude_m),
        "verdict": budget.map(rtl_verdict),
        "underReserve": budget.is_some_and(|b| b.past_return_point),
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GotoMark {
    latitude: f64,
    longitude: f64,
    radius: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct GotoFlight {
    in_goto_mode: bool,
    armed: bool,
    flying: bool,
    mode: String,
}

struct GotoState {
    mark: Option<GotoMark>,
    flight: GotoFlight,
}

static LAST_GOTO: std::sync::Mutex<GotoState> = std::sync::Mutex::new(GotoState {
    mark: None,
    flight: GotoFlight {
        in_goto_mode: false,
        armed: false,
        flying: false,
        mode: String::new(),
    },
});

fn goto_state() -> std::sync::MutexGuard<'static, GotoState> {
    LAST_GOTO
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn remember_goto(mark: GotoMark) {
    let held = &mut *goto_state();
    *held = GotoState {
        mark: Some(mark),
        flight: GotoFlight {
            armed: true,
            flying: true,
            ..held.flight.clone()
        },
    };
}

fn goto_outlived(was: &GotoFlight, now: &GotoFlight) -> bool {
    let left_goto = was.in_goto_mode && !now.in_goto_mode;
    let mode_changed = !was.mode.is_empty() && was.mode != now.mode && !now.in_goto_mode;
    let took_off = now.flying && !was.flying;
    !now.armed || left_goto || mode_changed || took_off
}

fn goto_shown(now: GotoFlight) -> Option<GotoMark> {
    let held = &mut *goto_state();
    *held = GotoState {
        mark: held.mark.filter(|_| !goto_outlived(&held.flight, &now)),
        flight: now,
    };
    held.mark
}

fn goto_vehicle(backend: &dyn Backend) -> (Value, GotoFlight) {
    let vehicle = backend.value_fields(
        "vehicle",
        "armed,flying,px4Firmware,orbitActive,flightMode,gotoFlightMode",
    );
    let flight = GotoFlight {
        in_goto_mode: vehicle.get("flightMode").is_some()
            && vehicle.get("flightMode") == vehicle.get("gotoFlightMode"),
        armed: flag(&vehicle, "armed"),
        flying: flag(&vehicle, "flying"),
        mode: crate::read::text(&vehicle, "flightMode"),
    };
    (vehicle, flight)
}

fn loiter_circle_shown(backend: &dyn Backend, vehicle: &Value) -> bool {
    !flag(vehicle, "px4Firmware")
        && !flag(vehicle, "orbitActive")
        && crate::guided::forward_flight(backend)
}

fn loiter_offer(backend: &dyn Backend) -> Option<GotoMark> {
    let (vehicle, flight) = goto_vehicle(backend);
    let in_goto_mode = flight.in_goto_mode;
    let mark = goto_shown(flight).filter(|_| in_goto_mode)?;
    let guided = flag(
        &backend.value_fields("vehicle.supports", "guidedMode"),
        "guidedMode",
    );
    let shown = flag(&vehicle, "armed")
        && flag(&vehicle, "flying")
        && guided
        && loiter_circle_shown(backend, &vehicle)
        && !crate::guided::mission_active(backend);
    shown.then_some(mark)
}

fn goto_location(backend: &dyn Backend) -> Option<Value> {
    let (vehicle, flight) = goto_vehicle(backend);
    let in_goto_mode = flight.in_goto_mode;
    goto_shown(flight).map(|mark| {
        let radius = loiter_circle_shown(backend, &vehicle).then_some(mark.radius.abs());
        let unit = crate::read::Unit::horizontal(backend);
        let vertical = crate::read::Unit::vertical(backend);
        let remaining = place(backend, "vehicle.coordinate").map(|at| crate::track::distance_m(at, (mark.latitude, mark.longitude)));
        let progress = remaining.filter(|_| in_goto_mode).map(|metres| {
            let height = raw(backend, "vehicle.altitudeRelative").map_or(String::new(), |h| format!("{:.0} {}", vertical.show(h), vertical.name));
            goto_progress(metres, raw(backend, "vehicle.groundSpeed").unwrap_or(0.0), &format!("{:.0} {}", unit.show(metres), unit.name), &height)
        });
        json!({
            "arrived": progress.as_ref().is_some_and(|(arrived, _)| *arrived),
            "progressText": progress.map(|(_, text)| text),
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
    let vehicle = backend.value_fields(
        "vehicle",
        "flying,sensorsPresentBits,flightMode,gotoFlightMode,isROIEnabled",
    );
    let supports = backend.value_fields("vehicle.supports", "roiMode,orbitMode");
    if vehicle.get("kind").and_then(Value::as_str) != Some("object") {
        return vec![];
    }
    let flying = flag(&vehicle, "flying");
    let gps =
        integer(&vehicle, "sensorsPresentBits").unwrap_or(GPS_SENSOR_BIT) & GPS_SENSOR_BIT != 0;
    let in_goto_mode = vehicle.get("flightMode").is_some()
        && vehicle.get("flightMode") == vehicle.get("gotoFlightMode");
    let confirm_in_guided = crate::read::value_number(
        &backend.value("settings.flyViewSettings.goToLocationRequiresConfirmInGuided.rawValue"),
    )
    .is_none_or(|v| v != 0.0);
    let home = backend.value("vehicle.homePosition");
    let home_known = flag(&home, "valid")
        && home
            .get("altitude")
            .and_then(Value::as_f64)
            .is_some_and(f64::is_finite);
    let targeting = crate::autodrop::turned_on(backend);
    let orbit = flying
        && flag(&supports, "orbitMode")
        && home_known
        && !crate::guided::mission_active(backend);
    [
        (flying, Offer { id: "GoTo", path: "vehicle.guidedModeGotoLocation", label: "Go to location", title: "Go To Location", message: "Move the vehicle to the specified location", confirm: !in_goto_mode || confirm_in_guided }),
        (orbit, Offer { id: "Orbit", path: "guided.orbit", label: "Orbit at location", title: "Orbit", message: "Orbit the vehicle around the specified location", confirm: true }),
        (flying && flag(&supports, "roiMode"), Offer { id: "Roi", path: "vehicle.guidedModeROI", label: "Look at this point", title: "Look at", message: "Point the camera at the specified location", confirm: false }),
        (flying && flag(&supports, "roiMode") && flag(&vehicle, "isROIEnabled"), Offer { id: "StopLooking", path: "vehicle.stopGuidedModeROI", label: "Stop looking", title: "Stop looking", message: "Stop pointing the camera at the location it is looking at", confirm: false }),
        (crate::synthview::native(), Offer { id: "Look3d", path: crate::map3d::MAP3D_LOOK, label: "Look here in 3D", title: "Look here in 3D", message: "Show this spot on the 3D map", confirm: false }),
        (true, Offer { id: "SetHome", path: "vehicle.doSetHome", label: "Set home here", title: "Set Home", message: "Set vehicle home as the specified location. This will affect Return to Home position", confirm: true }),
        (flying, Offer { id: "Heading", path: "vehicle.guidedModeChangeHeading", label: "Set Heading", title: "Change Heading", message: "Set the vehicle heading towards the specified location", confirm: true }),
        (!gps, Offer { id: "EstimatorOrigin", path: "vehicle.setEstimatorOrigin", label: "Set Estimator Origin", title: "Set Estimator Origin", message: "Make the specified location the estimator origin", confirm: true }),
        (targeting, Offer { id: "MarkTarget", path: "vehicle.markTarget", label: "Mark as target", title: "Mark Target", message: "Mark the specified location as the manual target for Auto Intercept", confirm: false }),
        (targeting, Offer { id: "DropZoneCorner", path: crate::dropzone::ADD_CORNER, label: "Add drop zone corner", title: "Drop Zone", message: "Add this point as a corner of the area Auto Intercept may drop into", confirm: false }),
        (targeting && !crate::dropzone::corners().is_empty(), Offer { id: "DropZoneClear", path: crate::dropzone::CLEAR, label: "Clear drop zone", title: "Clear Drop Zone", message: "Remove every corner of the drop zone", confirm: true }),
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
        "orbitRadiusMetres": {
            "default": ORBIT_DEFAULT_RADIUS_METRES,
            "min": ORBIT_RADIUS_METRES.0,
            "max": ORBIT_RADIUS_METRES.1,
            "step": ORBIT_RADIUS_METRES.2,
        },
        "orbitRadiusUnit": unit.name,
        "orbitMetresPerUnit": 1.0 / unit.factor,
        "orbitClockwise": true,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct Aircraft {
    connected: bool,
    flying: bool,
    rover: bool,
    roi: bool,
    heading: bool,
    gps: bool,
}

fn aircraft(backend: &dyn Backend) -> Aircraft {
    let vehicle = backend.value_fields("vehicle", "flying,sensorsPresentBits,rover");
    let supports = backend.value_fields("vehicle.supports", "roiMode,changeHeading");
    Aircraft {
        connected: vehicle.get("kind").and_then(Value::as_str) == Some("object"),
        flying: flag(&vehicle, "flying"),
        rover: flag(&vehicle, "rover"),
        roi: flag(&supports, "roiMode"),
        heading: flag(&supports, "changeHeading"),
        gps: integer(&vehicle, "sensorsPresentBits").unwrap_or(GPS_SENSOR_BIT) & GPS_SENSOR_BIT
            != 0,
    }
}

fn click_refusal(click: Click, a: Aircraft) -> Option<(&'static str, &'static str)> {
    match click {
        _ if !a.connected => Some(("noVehicle", "No vehicle is connected.")),
        Click::GoTo | Click::Roi | Click::Heading if !a.flying && !a.rover => {
            Some(("grounded", "The vehicle has to be flying for that."))
        }
        Click::Roi | Click::HomeRoi if !a.roi => {
            Some(("unsupported", "This vehicle cannot point at a location."))
        }
        Click::Heading if !a.heading => Some((
            "unsupported",
            "This vehicle cannot be turned to face a point.",
        )),
        Click::EstimatorOrigin if a.gps => Some((
            "hasGps",
            "The vehicle has GPS, so it already knows where it is.",
        )),
        _ => None,
    }
}

fn chosen_height(backend: &dyn Backend, chosen: &Value) -> Result<Option<f64>, String> {
    if chosen.is_null() {
        return Ok(None);
    }
    let limit = |name: &str| raw(backend, &format!("settings.flyViewSettings.{name}"));
    let range = limit("guidedMinimumAltitude")
        .zip(limit("guidedMaximumAltitude"))
        .map(|(lowest, highest)| (lowest.max(crate::guidedcmd::GOTO_LOWEST_HEIGHT_M), highest));
    match (chosen.as_f64().filter(|height| height.is_finite()), range) {
        (Some(height), Some((lowest, highest))) if (lowest..=highest).contains(&height) => {
            Ok(Some(height))
        }
        (_, Some((lowest, highest))) => Err(format!(
            "Choose a go-to height from {} above launch.",
            crate::read::range_text(lowest, highest, &crate::read::Unit::vertical(backend))
        )),
        (_, None) => Err(
            "The guided altitude limits are not known, so the chosen height cannot be checked."
                .to_string(),
        ),
    }
}

fn roi_action(backend: &dyn Backend, latitude: f64, longitude: f64, altitude: f64) -> Value {
    match flag(
        &backend.value_fields("vehicle", "px4Firmware"),
        "px4Firmware",
    ) {
        true => {
            let home = backend
                .value("vehicle.homePosition")
                .get("altitude")
                .and_then(Value::as_f64)
                .unwrap_or(f64::NAN);
            let terrain = crate::terrainservice::height_now(latitude, longitude)
                .ok()
                .flatten()
                .unwrap_or(home);
            json!({ "action": "roi", "latitude": latitude, "longitude": longitude, "altitude": terrain, "frame": crate::guidedcmd::FRAME_GLOBAL })
        }
        false => {
            json!({ "action": "roi", "latitude": latitude, "longitude": longitude, "altitude": altitude, "frame": crate::guidedcmd::FRAME_GLOBAL_RELATIVE_ALT })
        }
    }
}

pub fn send(backend: &dyn Backend, click: Click, path: &str, args: &str) -> Value {
    let refused =
        |token: &str, reason: &str| json!({ "ok": false, "refusal": token, "reason": reason });
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let Some((latitude, longitude)) = crate::fenceedit::point(given.get(0)) else {
        return refused(
            "badCoordinate",
            "The point needs a latitude from -90 to 90 and a longitude from -180 to 180.",
        );
    };
    let mut at = json!({ "latitude": latitude, "longitude": longitude });
    if let Some(altitude) = given[0]
        .get("altitude")
        .and_then(Value::as_f64)
        .filter(|a| a.is_finite())
    {
        at["altitude"] = json!(altitude);
    }
    let forwarded = match click {
        Click::GoTo => match given.get(1).map(Value::as_f64) {
            None => json!([at, crate::guided::goto_loiter_radius(backend)]),
            Some(Some(radius)) if radius.is_finite() => json!([at, radius]),
            Some(_) => {
                return refused(
                    "badRadius",
                    "A loiter radius is a number of metres; its sign is the direction.",
                );
            }
        },
        _ => json!([at]),
    };
    if let Some((token, reason)) = click_refusal(click, aircraft(backend)) {
        return refused(token, reason);
    }
    let height = match (click, chosen_height(backend, &given[0]["height"])) {
        (Click::GoTo, Err(reason)) => return refused("badHeight", &reason),
        (Click::GoTo, Ok(height)) => height,
        _ => None,
    };
    let core = match click {
        Click::GoTo => Some(
            json!({ "action": "goto", "latitude": latitude, "longitude": longitude, "loiterRadius": forwarded[1], "height": height }),
        ),
        Click::EstimatorOrigin => Some(
            json!({ "action": "estimatorOrigin", "latitude": latitude, "longitude": longitude, "altitude": at.get("altitude").cloned().unwrap_or(json!(0.0)) }),
        ),
        _ if crate::qthost::present() => None,
        Click::Heading => {
            Some(json!({ "action": "heading", "latitude": latitude, "longitude": longitude }))
        }
        Click::SetHome => Some(
            json!({ "action": "setHome", "latitude": latitude, "longitude": longitude, "terrain": crate::terrainservice::height_now(latitude, longitude).ok().flatten() }),
        ),
        Click::Roi | Click::HomeRoi => Some(roi_action(
            backend,
            latitude,
            longitude,
            at.get("altitude").and_then(Value::as_f64).unwrap_or(0.0),
        )),
        Click::MarkTarget => {
            Some(json!({ "action": "markTarget", "latitude": latitude, "longitude": longitude }))
        }
    };
    let sent = crate::guided::dispatch(
        backend,
        core,
        crate::guided::active_id(backend),
        path,
        &forwarded.to_string(),
    );
    if click == Click::GoTo && flag(&sent, "ok") {
        remember_goto(GotoMark {
            latitude,
            longitude,
            radius: forwarded[1].as_f64().unwrap_or(0.0),
        });
    }
    sent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_go_to_takes_as_long_as_its_slower_leg_and_says_so_roughly() {
        let speeds = GotoSpeeds {
            across_mps: 5.0,
            up_mps: 2.5,
            down_mps: 1.0,
        };
        assert_eq!(goto_seconds(100.0, 0.0, speeds), Some(20.0));
        assert_eq!(
            goto_seconds(10.0, 50.0, speeds),
            Some(20.0),
            "a 50 m climb at 2.5 m/s outlasts 10 m across"
        );
        assert_eq!(
            goto_seconds(10.0, -30.0, speeds),
            Some(30.0),
            "going down uses the descent speed"
        );
        assert_eq!(
            goto_seconds(
                10.0,
                0.0,
                GotoSpeeds {
                    across_mps: 0.0,
                    ..speeds
                }
            ),
            None
        );
        assert_eq!(about_text(0.2), "about 1 sec");
        assert_eq!(about_text(24.6), "about 25 sec");
        assert_eq!(about_text(59.6), "about 1 min");
        assert_eq!(about_text(150.0), "about 3 min");
    }

    #[test]
    fn a_go_to_preview_says_what_is_left_on_landing_or_that_it_is_past_the_return_point() {
        let fine = goto_budget(60.0, 600.0, 7.0, 60.0, 120.0).unwrap();
        assert_eq!(fine.landing_percent, 42.0, "180 s at 0.1 %/s");
        assert!(!fine.past_return_point);
        assert_eq!(goto_verdict(fine), "About 42% left on landing");
        let late = goto_budget(13.0, 120.0, 7.0, 16.0, 40.0).unwrap();
        assert!(
            late.past_return_point,
            "13 % with 56 s of flying at 0.108 %/s lands under the 7 % reserve"
        );
        assert_eq!(
            goto_verdict(late),
            "Past your return point. Return home instead?"
        );
        assert_eq!(goto_budget(13.0, 0.0, 7.0, 16.0, 40.0), None);
    }

    #[test]
    fn a_go_to_in_progress_counts_down_and_then_says_it_has_arrived() {
        assert_eq!(
            goto_progress(45.0, 5.0, "45 m", "20 m"),
            (
                false,
                "Going to point \u{00b7} 45 m \u{00b7} 9 sec".to_string()
            )
        );
        assert_eq!(
            goto_progress(3.0, 0.1, "3 m", "20 m"),
            (true, "Arrived \u{00b7} holding at 20 m".to_string())
        );
        assert_eq!(
            goto_progress(30.0, 0.1, "30 m", "20 m").0,
            false,
            "stopped short of the point is not arrived"
        );
    }
    use std::cell::RefCell;

    #[test]
    fn each_map_click_is_sent_only_where_the_head_offered_it() {
        let flying = Aircraft {
            connected: true,
            flying: true,
            rover: false,
            roi: true,
            heading: true,
            gps: true,
        };
        assert_eq!(click_refusal(Click::GoTo, flying), None);
        assert_eq!(
            click_refusal(
                Click::GoTo,
                Aircraft {
                    flying: false,
                    ..flying
                }
            )
            .map(|r| r.0),
            Some("grounded")
        );
        assert_eq!(
            click_refusal(
                Click::GoTo,
                Aircraft {
                    flying: false,
                    rover: true,
                    ..flying
                }
            ),
            None,
            "a rover drives to a clicked point without ever reporting flying"
        );
        assert_eq!(
            click_refusal(
                Click::MarkTarget,
                Aircraft {
                    flying: false,
                    rover: false,
                    ..flying
                }
            ),
            None,
            "marking a target on the map needs no flight state at all - autodrop reads it later, whenever it runs"
        );
        assert_eq!(
            click_refusal(
                Click::Roi,
                Aircraft {
                    roi: false,
                    ..flying
                }
            )
            .map(|r| r.0),
            Some("unsupported"),
            "Vehicle::guidedModeROI logs and returns while the bridge answers ok"
        );
        assert_eq!(
            click_refusal(
                Click::HomeRoi,
                Aircraft {
                    flying: false,
                    ..flying
                }
            ),
            None,
            "GimbalIndicator's Point Home calls guidedModeROI(homePosition) on the ground too"
        );
        assert_eq!(
            click_refusal(
                Click::Heading,
                Aircraft {
                    heading: false,
                    ..flying
                }
            )
            .map(|r| r.0),
            Some("unsupported")
        );
        assert_eq!(
            click_refusal(
                Click::SetHome,
                Aircraft {
                    flying: false,
                    ..flying
                }
            ),
            None,
            "home can be set on the ground"
        );
        assert_eq!(
            click_refusal(Click::EstimatorOrigin, flying).map(|r| r.0),
            Some("hasGps")
        );
        assert_eq!(
            click_refusal(
                Click::EstimatorOrigin,
                Aircraft {
                    gps: false,
                    flying: false,
                    ..flying
                }
            ),
            None
        );
        assert_eq!(
            click_refusal(Click::SetHome, Aircraft::default()).map(|r| r.0),
            Some("noVehicle")
        );
        assert_eq!(
            Click::of("vehicle.guidedModeOrbit"),
            None,
            "the orbit is guided.orbit's, with a radius and altitude the operator chose"
        );
    }

    struct Offered {
        flying: bool,
        sensors: i64,
        mode: &'static str,
        roi: bool,
        confirm_in_guided: f64,
        orbit: bool,
        looking: bool,
        targeting: bool,
    }
    impl Backend for Offered {
        fn get(&self, p: &str) -> String {
            match p {
                "settings.appSettings.autodropEnabled.rawValue" => {
                    json!({ "value": self.targeting }).to_string()
                }
                "settings.flyViewSettings.goToLocationRequiresConfirmInGuided.rawValue" => {
                    json!({ "value": self.confirm_in_guided }).to_string()
                }
                "vehicle.homePosition" => {
                    json!({ "kind": "object", "valid": true, "altitude": 480.0 }).to_string()
                }
                _ => self.get_fields(p, ""),
            }
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "vehicle" => json!({ "kind": "object", "flying": self.flying, "sensorsPresentBits": self.sensors, "flightMode": self.mode, "gotoFlightMode": "Guided", "isROIEnabled": self.looking }),
                "vehicle.supports" => json!({ "kind": "object", "roiMode": self.roi, "orbitMode": self.orbit }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn the_menu_offers_what_qgc_offers() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let labels = |b: &Offered| {
            map_click_view(b, &[])["actions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a["label"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };
        let flying = Offered {
            flying: true,
            sensors: 32,
            mode: "Loiter",
            roi: true,
            confirm_in_guided: 0.0,
            orbit: false,
            looking: false,
            targeting: true,
        };
        assert_eq!(
            labels(&flying),
            [
                "Go to location",
                "Look at this point",
                "Set home here",
                "Set Heading",
                "Mark as target",
                "Add drop zone corner"
            ]
        );
        assert_eq!(
            labels(&Offered {
                flying: false,
                ..flying
            }),
            ["Set home here", "Mark as target", "Add drop zone corner"],
            "on the ground only home can be moved, but a target can always be marked"
        );
        assert_eq!(
            labels(&Offered {
                flying: false,
                targeting: false,
                ..flying
            }),
            ["Set home here"],
            "with Auto Intercept off nothing in the menu marks a target or a drop zone"
        );
        assert_eq!(
            labels(&Offered {
                flying: false,
                sensors: 0,
                ..flying
            }),
            [
                "Set home here",
                "Set Estimator Origin",
                "Mark as target",
                "Add drop zone corner"
            ],
            "a vehicle without GPS is offered an origin"
        );
        let actions = |b: &Offered| map_click_view(b, &[])["actions"].clone();
        assert_eq!(
            actions(&flying)[0]["confirm"],
            true,
            "outside the goto mode a goto is confirmed"
        );
        assert_eq!(
            actions(&Offered {
                mode: "Guided",
                ..flying
            })[0]["confirm"],
            false,
            "already in Guided the goto is sent at once unless the setting asks for a confirmation"
        );
        assert_eq!(
            actions(&Offered {
                mode: "Guided",
                confirm_in_guided: 1.0,
                ..flying
            })[0]["confirm"],
            true
        );
        assert_eq!(
            actions(&flying)[1]["confirm"],
            false,
            "QGC sends an ROI without asking"
        );
        assert_eq!(
            labels(&Offered {
                orbit: true,
                ..flying
            })[1],
            "Orbit at location",
            "an orbit follows the goto when the vehicle supports it and home carries an altitude"
        );
        assert_eq!(map_click_view(&flying, &[])["orbitDefaultRadius"], 30.0);
        assert_eq!(
            map_click_view(&flying, &[])["orbitRadiusMetres"],
            json!({ "default": 30.0, "min": 10.0, "max": 500.0, "step": 10.0 }),
            "the radius stepper FlyViewMap shows beside Orbit at location"
        );
        assert_eq!(
            labels(&Offered {
                looking: true,
                ..flying
            })[1..3],
            ["Look at this point", "Stop looking"],
            "Stop looking is offered while the camera is looking at a point"
        );
        assert_eq!(
            map_click_view(&Vehicle(RefCell::new(vec![])), &[])["offered"],
            true
        );
    }

    struct Vehicle(RefCell<Vec<String>>);
    impl Backend for Vehicle {
        fn get(&self, p: &str) -> String {
            match p {
                "settings.flyViewSettings.guidedMinimumAltitude.rawValue" => {
                    json!({ "kind": "value", "value": 5.0 }).to_string()
                }
                "settings.flyViewSettings.guidedMaximumAltitude.rawValue" => {
                    json!({ "kind": "value", "value": 120.0 }).to_string()
                }
                _ => self.get_fields(p, ""),
            }
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "vehicle" => json!({ "kind": "object", "flying": true, "sensorsPresentBits": 32 }),
                _ => json!({ "kind": "object", "roiMode": true, "changeHeading": true }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, a: &str) -> String {
            self.0.borrow_mut().push(a.to_string());
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_go_to_height_outside_the_guided_limits_is_refused_before_anything_is_sent() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let vehicle = Vehicle(RefCell::new(Vec::new()));
        let go = |height: Value| {
            send(
                &vehicle,
                Click::GoTo,
                "vehicle.guidedModeGotoLocation",
                &json!([{ "latitude": 47.4, "longitude": 8.5, "height": height }, 25]).to_string(),
            )
        };
        [
            json!(0),
            json!(-10),
            json!(4.9),
            json!(120.5),
            json!("high"),
        ]
        .into_iter()
        .for_each(|height| {
            let refused = go(height.clone());
            assert_eq!(
                (refused["refusal"].as_str(), refused["reason"].as_str()),
                (
                    Some("badHeight"),
                    Some("Choose a go-to height from 5 m to 120 m above launch.")
                ),
                "{height}"
            );
        });
        assert!(
            vehicle
                .0
                .borrow()
                .iter()
                .all(|sent| !sent.contains("latitude")),
            "nothing went to the vehicle: {:?}",
            vehicle.0.borrow()
        );
        assert_eq!(go(json!(30))["ok"], true);
        assert_eq!(
            go(Value::Null)["ok"],
            true,
            "no height keeps the current one"
        );
        assert_eq!(
            chosen_height(&Plane, &json!(30)),
            Err(
                "The guided altitude limits are not known, so the chosen height cannot be checked."
                    .to_string()
            )
        );
        assert_eq!(
            chosen_height(&Limits(0.0, 120.0), &json!(1)),
            Err("Choose a go-to height from 2 m to 120 m above launch.".to_string()),
            "a guided minimum set to 0 still keeps the go-to off the ground"
        );
    }

    struct Limits(f64, f64);
    impl Backend for Limits {
        fn get(&self, p: &str) -> String {
            match p {
                "settings.flyViewSettings.guidedMinimumAltitude.rawValue" => {
                    json!({ "kind": "value", "value": self.0 })
                }
                "settings.flyViewSettings.guidedMaximumAltitude.rawValue" => {
                    json!({ "kind": "value", "value": self.1 })
                }
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            self.get(p)
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_point_and_radius_are_checked_before_anything_is_sent() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let vehicle = Vehicle(RefCell::new(Vec::new()));
        assert_eq!(
            send(
                &vehicle,
                Click::GoTo,
                "vehicle.guidedModeGotoLocation",
                r#"[{"latitude":47.4,"longitude":8.5,"altitude":0},25]"#
            )["ok"],
            true
        );
        assert_eq!(
            send(
                &vehicle,
                Click::GoTo,
                "vehicle.guidedModeGotoLocation",
                r#"[{"latitude":47.4,"longitude":8.5},"wide"]"#
            )["refusal"],
            "badRadius"
        );
        assert_eq!(
            send(
                &vehicle,
                Click::Roi,
                "vehicle.guidedModeROI",
                r#"[{"latitude":99,"longitude":8.5}]"#
            )["refusal"],
            "badCoordinate"
        );
        assert_eq!(
            send(
                &vehicle,
                Click::EstimatorOrigin,
                "vehicle.setEstimatorOrigin",
                r#"[{"latitude":47.4,"longitude":8.5}]"#
            )["refusal"],
            "hasGps"
        );
        assert_eq!(
            vehicle.0.borrow().as_slice(),
            &[r#"[{"altitude":0.0,"latitude":47.4,"longitude":8.5},25.0]"#.to_string()]
        );
        assert_eq!(
            goto_state().mark,
            Some(GotoMark {
                latitude: 47.4,
                longitude: 8.5,
                radius: 25.0
            }),
            "a goto that went out is the point a loiter radius change re-sends"
        );
        assert_eq!(
            send(
                &vehicle,
                Click::GoTo,
                "vehicle.guidedModeGotoLocation",
                r#"[{"latitude":47.4,"longitude":8.5},-40]"#
            )["ok"],
            true,
            "the sign of the radius is the loiter direction, as QGC sends it"
        );
        assert_eq!(
            goto_state().mark.map(|m| m.radius),
            Some(-40.0),
            "a loiter radius change is the radius the circle now draws"
        );
    }

    struct Plane;
    impl Backend for Plane {
        fn get(&self, p: &str) -> String {
            self.get_fields(p, "")
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }),
                "vehicle" => json!({ "kind": "object", "armed": true, "flying": true, "fixedWing": true, "flightMode": "Guided", "gotoFlightMode": "Guided" }),
                "vehicle.supports" => json!({ "kind": "object", "guidedMode": true }),
                _ => json!({ "kind": "object" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_loiter_circle_keeps_the_committed_radius_and_direction() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        goto_shown(GotoFlight::default());
        remember_goto(GotoMark {
            latitude: 47.4,
            longitude: 8.5,
            radius: -120.0,
        });
        let view = map_click_view(&Plane, &[]);
        assert_eq!(view["gotoLocation"]["loiterRadiusMetres"], 120.0);
        assert_eq!(
            view["gotoLocation"]["loiterClockwise"], false,
            "QGCMapCircleVisuals points its rotation arrows the way the circle was committed"
        );
        assert_eq!(
            view["loiter"]["defaultRadius"], 120.0,
            "the radius edit starts from the committed circle, not the setting"
        );
        assert_eq!(view["loiter"]["clockwise"], false);
        remember_goto(GotoMark {
            latitude: 47.4,
            longitude: 8.5,
            radius: 80.0,
        });
        assert_eq!(
            map_click_view(&Plane, &[])["gotoLocation"]["loiterClockwise"],
            true
        );
        goto_shown(GotoFlight::default());
    }

    #[test]
    fn the_go_here_marker_stays_until_the_vehicle_leaves_the_goto_mode() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mark = GotoMark {
            latitude: 47.4,
            longitude: 8.5,
            radius: 0.0,
        };
        let loiter = GotoFlight {
            in_goto_mode: false,
            armed: true,
            flying: true,
            mode: "Loiter".to_string(),
        };
        let guided = GotoFlight {
            in_goto_mode: true,
            mode: "Guided".to_string(),
            ..loiter.clone()
        };
        goto_shown(loiter.clone());
        remember_goto(mark);
        assert_eq!(
            goto_shown(loiter.clone()),
            Some(mark),
            "a goto sent from Loiter shows before the vehicle has switched to Guided"
        );
        assert_eq!(goto_shown(guided.clone()), Some(mark));
        assert_eq!(
            goto_shown(loiter),
            None,
            "onInGotoFlightModeChanged hides the item when the goto mode is left"
        );
        assert_eq!(goto_shown(guided), None);
    }

    #[test]
    fn the_return_confirm_says_how_far_how_high_and_what_is_left_on_landing() {
        assert_eq!(
            rtl_verdict(GotoBudget {
                landing_percent: 41.6,
                past_return_point: false
            }),
            "About 42% left on landing"
        );
        assert_eq!(
            rtl_verdict(GotoBudget {
                landing_percent: 12.2,
                past_return_point: true
            }),
            "About 12% left on landing, under the reserve"
        );
        assert_eq!(
            rtl_preview_view(&Plane, &[])["available"],
            false,
            "with no distance to home there is nothing honest to say"
        );
    }

    struct Homeward {
        altitude: f64,
        percent: f64,
    }
    impl Backend for Homeward {
        fn get(&self, path: &str) -> String {
            let value = |value: f64| json!({ "kind": "value", "value": value });
            let fact = |value: f64| json!({ "kind": "fact", "value": value, "rawValue": value });
            let parameter = path
                .strip_prefix("vehicle.parameterManager.getParameter(-1,")
                .and_then(|rest| rest.strip_suffix(')'))
                .and_then(|name| {
                    [
                        ("RTL_RETURN_ALT", "m", 60.0),
                        ("MPC_XY_CRUISE", "m/s", 5.0),
                        ("MPC_Z_V_AUTO_UP", "m/s", 2.0),
                        ("MPC_LAND_SPEED", "m/s", 1.0),
                        ("COM_LOW_BAT_ACT", "", 3.0),
                        ("BAT_CRIT_THR", "%", 7.0),
                    ]
                    .into_iter()
                    .find(|(known, _, _)| *known == name)
                })
                .map(|(name, units, v)| json!({ "kind": "fact", "name": name, "value": v, "rawValue": v, "rawUnits": units }));
            match (path, parameter) {
                (_, Some(parameter)) => parameter,
                ("vehicle.distanceToHome.rawValue", _) => value(600.0),
                ("vehicle.altitudeRelative.rawValue", _) => value(self.altitude),
                ("vehicle.batteries.count", _) => value(1.0),
                ("vehicle.batteries.0.percentRemaining", _) => fact(self.percent),
                ("vehicle.batteries.0.timeRemaining", _) => fact(600.0),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            self.get(p)
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_return_preview_reads_the_trip_home_and_what_the_pack_has_left() {
        let preview = |altitude: f64, percent: f64| {
            let view = rtl_preview_view(&Homeward { altitude, percent }, &[]);
            (
                view["distanceText"].clone(),
                view["returnAltitudeText"].clone(),
                view["climbs"].clone(),
                view["verdict"].clone(),
                view["underReserve"].clone(),
            )
        };
        assert_eq!(
            preview(40.0, 60.0),
            (
                json!("600 m"),
                json!("60 m"),
                json!(true),
                json!("About 41% left on landing"),
                json!(false)
            ),
            "climb 20 m at 2 m/s, fly 600 m at 5 m/s, descend 60 m at 1 m/s: 190 s at 0.1 %/s is 19 %"
        );
        assert_eq!(
            preview(90.0, 60.0),
            (
                json!("600 m"),
                json!("90 m"),
                json!(false),
                json!("About 39% left on landing"),
                json!(false)
            ),
            "above the return altitude it comes home where it is: 120 s and 90 s down"
        );
        assert_eq!(
            preview(40.0, 10.0),
            (
                json!("600 m"),
                json!("60 m"),
                json!(true),
                json!("About 7% left on landing, under the reserve"),
                json!(true)
            ),
            "190 s costs 3.2 % of a 10 % pack, landing at the 7 % the failsafe keeps"
        );
    }

    #[test]
    fn the_go_here_marker_never_outlives_a_disarm_a_new_takeoff_or_a_mode_change() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let guided = GotoFlight {
            in_goto_mode: true,
            armed: true,
            flying: true,
            mode: "Guided".to_string(),
        };
        goto_shown(guided.clone());
        remember_goto(GotoMark {
            latitude: 47.4,
            longitude: 8.5,
            radius: 0.0,
        });
        assert!(goto_shown(guided.clone()).is_some());
        let landed = GotoFlight {
            armed: false,
            flying: false,
            ..guided.clone()
        };
        assert_eq!(
            goto_shown(landed),
            None,
            "an ArduCopter lands and disarms in Guided, so leaving the goto mode never happens"
        );
        assert_eq!(
            goto_shown(guided.clone()),
            None,
            "the next takeoff in Guided must not resurrect the old point"
        );
        let on_ground = GotoFlight {
            flying: false,
            ..guided.clone()
        };
        assert!(goto_outlived(&on_ground, &guided), "a takeoff clears it");
        let loiter = GotoFlight {
            in_goto_mode: false,
            mode: "Loiter".to_string(),
            ..guided.clone()
        };
        let alt_hold = GotoFlight {
            mode: "AltHold".to_string(),
            ..loiter.clone()
        };
        assert!(
            goto_outlived(&loiter, &alt_hold),
            "a mode change before the goto mode is reached cancels it"
        );
        assert!(
            !goto_outlived(&loiter, &guided),
            "switching into the goto mode is the goto starting, not a change of mind"
        );
        assert!(!goto_outlived(&guided, &guided));
        goto_shown(GotoFlight::default());
    }
}
