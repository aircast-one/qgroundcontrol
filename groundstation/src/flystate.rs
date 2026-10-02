use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.armed", "vehicle.flying", "vehicle.landing", "vehicle.flightMode", "vehicle.vehicleLinkManager.communicationLost", "vehicle.vehicleLinkManager.communicationLostEnabled", "planFly.missionController.currentMissionIndex", "vehicle.rcRSSI", "vehicle.rcChannelOverrideActive", "vehicle.radioStatus.lrssi", "vehicle.radioStatus.rrssi", "vehicle.radioStatus.lNoise", "vehicle.radioStatus.rNoise", "vehicle.radioStatus.rxErrors", "vehicle.radioStatus.fixed", "vehicle.radioStatus.txBuffer", "vehicle.healthAndArmingCheckReport.supported", "vehicle.healthAndArmingCheckReport.canArm", "vehicle.healthAndArmingCheckReport.hasWarningsOrErrors", "vehicle.sysStatusSensorInfo.sensorNames", "vehicle.sysStatusSensorInfo.sensorEnabled", "vehicle.sysStatusSensorInfo.sensorHealthy", "vehicle.readyToFlyAvailable", "vehicle.readyToFly", "vehicle.allSensorsHealthy", crate::setup::COMPONENTS];

pub const STALE_NOTICE: &str = "No contact — these are the last values the vehicle sent.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    NotConnected,
    ContactLost,
    Flying,
    Landing,
    Armed,
    Disarmed,
}

pub const STATES: &[State] = &[State::NotConnected, State::ContactLost, State::Flying, State::Landing, State::Armed, State::Disarmed];

impl State {
    pub fn token(self) -> &'static str {
        match self {
            State::NotConnected => "notConnected",
            State::ContactLost => "contactLost",
            State::Flying => "flying",
            State::Landing => "landing",
            State::Armed => "armed",
            State::Disarmed => "disarmed",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            State::NotConnected => "Not connected",
            State::ContactLost => "Comms Lost",
            State::Flying => "Flying",
            State::Landing => "Landing",
            State::Armed => "Armed",
            State::Disarmed => "Disarmed",
        }
    }
}

pub fn ready_to_fly(report: Option<bool>, ready_to_fly: Option<bool>, sensors_healthy: bool, setup_complete: impl FnOnce() -> bool) -> bool {
    report.or(ready_to_fly).unwrap_or_else(|| sensors_healthy && setup_complete())
}

pub fn status_fault(report: Option<(bool, bool)>, sensors: &[(bool, bool)]) -> bool {
    match report {
        Some((can_arm, _)) => !can_arm,
        None => sensors.iter().any(|(enabled, healthy)| *enabled && !*healthy),
    }
}

pub fn status_nominal(report: Option<(bool, bool)>, sensors: &[(bool, bool)]) -> bool {
    match report {
        Some((can_arm, warnings)) => can_arm && !warnings,
        None => sensors.iter().all(|(enabled, healthy)| *enabled && *healthy),
    }
}

pub fn state_line(state: State, ready: bool, nominal: bool) -> &'static str {
    match (state, ready, nominal) {
        (State::Disarmed, true, true) => "Ready to Fly",
        (State::Disarmed, true, _) => "Not Fully Ready",
        (State::Disarmed, false, _) => "Not Ready",
        (other, ..) => other.line(),
    }
}

pub fn state_of(connected: bool, contact_lost: bool, armed: bool, flying: bool, landing: bool) -> State {
    match (connected, contact_lost, armed, flying, landing) {
        (false, ..) => State::NotConnected,
        (_, true, ..) => State::ContactLost,
        (_, _, false, ..) => State::Disarmed,
        (_, _, _, true, _) => State::Flying,
        (_, _, _, false, true) => State::Landing,
        _ => State::Armed,
    }
}

fn rc_signal(vehicle: &Value) -> Option<i64> {
    crate::read::fact_property(vehicle, "rcRSSI").and_then(|fact| fact.get("value")).and_then(Value::as_i64).filter(|rssi| (0..=100).contains(rssi))
}

fn flying_to(backend: &dyn Backend) -> Option<i64> {
    object(&backend.get_fields("planFly.missionController", "currentMissionIndex"))
        .get("currentMissionIndex")
        .and_then(Value::as_i64)
        .filter(|sequence| *sequence >= 0)
}

fn telemetry(radio: &Value) -> Value {
    let reading = |name: &str| crate::read::fact_property(radio, name).and_then(|fact| fact.get("value")).and_then(Value::as_i64);
    match reading("lrssi").filter(|local| *local != 0) {
        None => Value::Null,
        Some(local) => json!({
            "localRssiDbm": local,
            "remoteRssiDbm": reading("rrssi"),
            "localNoise": reading("lNoise"),
            "remoteNoise": reading("rNoise"),
            "receiveErrors": reading("rxErrors"),
            "errorsFixed": reading("fixed"),
            "txBuffer": reading("txBuffer"),
        }),
    }
}

pub fn fly_state_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "armed,flying,landing,flightMode,rcRSSI,rcChannelOverrideActive"));
    let radio = object(&backend.get_fields("vehicle.radioStatus", "lrssi,rrssi,lNoise,rNoise,rxErrors,fixed,txBuffer"));
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let links = object(&backend.get_fields("vehicle.vehicleLinkManager", "communicationLost,communicationLostEnabled"));
    let watching = flag(&links, "communicationLostEnabled");
    let reported = connected.then(|| watching.then(|| flag(&links, "communicationLost"))).flatten();
    let contact_lost = reported.unwrap_or(false);
    let state = state_of(connected, contact_lost, flag(&vehicle, "armed"), flag(&vehicle, "flying"), flag(&vehicle, "landing"));
    let health = health(backend);
    json!({
        "kind": "object",
        "class": "FlyState",
        "connected": connected,
        "armed": flag(&vehicle, "armed"),
        "flying": flag(&vehicle, "flying"),
        "landing": flag(&vehicle, "landing"),
        "contactLost": reported,
        "state": state.token(),
        "stateText": state_line(state, state == State::Disarmed && disarmed_ready(backend), health.nominal()),
        "nominal": health.nominal(),
        "fault": health.fault(),
        "canArm": flag(&vehicle, "armed") || health.report.is_none_or(|(can_arm, _)| can_arm),
        "summaryDetail": connected.then(|| summary_detail(flag(&vehicle, "armed"), flag(&vehicle, "flying") || flag(&vehicle, "landing"), health.check_issues, &health.sensors)),
        "staleNotice": if contact_lost { STALE_NOTICE } else { "" },
        "mode": text(&vehicle, "flightMode"),
        "flyingToSequence": flying_to(backend),
        "rcSupported": flag(&object(&backend.get_fields("vehicle.supports", "radio")), "radio"),
        "rcSignal": rc_signal(&vehicle),
        "rcSignalText": rc_signal(&vehicle).map(|percent| match percent {
            0 => "No signal".to_string(),
            percent => format!("{percent}%"),
        }),
        "rcOverride": connected.then(|| flag(&vehicle, "rcChannelOverrideActive")),
        "telemetry": telemetry(&radio),
    })
}

struct Health {
    report: Option<(bool, bool)>,
    sensors: Vec<(String, bool, bool)>,
    check_issues: usize,
}

fn health(backend: &dyn Backend) -> Health {
    let report = object(&backend.get_fields("vehicle.healthAndArmingCheckReport", "supported,canArm,hasWarningsOrErrors"));
    let supported = flag(&report, "supported");
    let info = object(&backend.get("vehicle.sysStatusSensorInfo"));
    let flags = |key: &str| -> Vec<bool> { info.get(key).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_bool().unwrap_or(v.as_i64().unwrap_or(0) != 0)).collect()).unwrap_or_default() };
    let (enabled, healthy) = (flags("sensorEnabled"), flags("sensorHealthy"));
    let names: Vec<String> = info.get("sensorNames").and_then(Value::as_array).map(|a| a.iter().map(|n| n.as_str().unwrap_or_default().to_string()).collect()).unwrap_or_default();
    let problems = supported.then(|| object(&backend.get("vehicle.healthAndArmingCheckReport.problemsForCurrentMode"))).and_then(|model| model.get("elements").and_then(Value::as_array).map(Vec::len));
    Health {
        report: supported.then(|| (flag(&report, "canArm"), flag(&report, "hasWarningsOrErrors"))),
        sensors: match supported {
            true => Vec::new(),
            false => names.into_iter().enumerate().map(|(i, name)| (name, enabled.get(i).copied().unwrap_or(false), healthy.get(i).copied().unwrap_or(false))).collect(),
        },
        check_issues: problems.unwrap_or(0),
    }
}

impl Health {
    fn fault(&self) -> bool {
        status_fault(self.report, &self.sensors.iter().map(|(_, enabled, healthy)| (*enabled, *healthy)).collect::<Vec<_>>())
    }

    fn nominal(&self) -> bool {
        status_nominal(self.report, &self.sensors.iter().map(|(_, enabled, healthy)| (*enabled, *healthy)).collect::<Vec<_>>())
    }
}

pub fn summary_detail(armed: bool, in_air: bool, check_issues: usize, sensors: &[(String, bool, bool)]) -> String {
    let listed = |pick: fn(bool, bool) -> bool| sensors.iter().filter(|(_, enabled, healthy)| pick(*enabled, *healthy)).map(|(name, ..)| name.as_str()).collect::<Vec<_>>().join(", ");
    let (faults, disabled) = (listed(|enabled, healthy| enabled && !healthy), listed(|enabled, _| !enabled));
    match (armed, in_air) {
        (true, true) => "Motors are armed and the vehicle is in the air.".to_string(),
        (true, false) => "Motors are armed. Keep clear of the propellers.".to_string(),
        _ if check_issues > 0 => format!("{check_issues} check(s) need attention before arming."),
        _ if !faults.is_empty() => format!("{faults} not working. Position modes and Return to Launch may not work."),
        _ if !disabled.is_empty() => format!("{disabled} turned off. Everything else reports normal."),
        _ => "All checks passed.".to_string(),
    }
}

fn disarmed_ready(backend: &dyn Backend) -> bool {
    let report = object(&backend.get_fields("vehicle.healthAndArmingCheckReport", "supported,canArm"));
    let vehicle = object(&backend.get_fields("vehicle", "readyToFlyAvailable,readyToFly,allSensorsHealthy"));
    ready_to_fly(flag(&report, "supported").then(|| flag(&report, "canArm")), flag(&vehicle, "readyToFlyAvailable").then(|| flag(&vehicle, "readyToFly")), flag(&vehicle, "allSensorsHealthy"), || crate::setup::setup_complete(backend))
}

fn reboot_refusal(state: &str) -> Option<(&'static str, &'static str)> {
    match state {
        "disarmed" => None,
        "notConnected" => Some(("noVehicle", "No vehicle is connected.")),
        "contactLost" => Some(("contactLost", "The vehicle has stopped answering, so it is not known to be on the ground. Restore the link before rebooting.")),
        "flying" | "landing" => Some(("flying", "The vehicle is in the air. Land and disarm before rebooting.")),
        _ => Some(("armed", "The vehicle is armed. Disarm it before rebooting.")),
    }
}

pub fn reboot(backend: &dyn Backend, path: &str) -> Value {
    let view = fly_state_view(backend, &[]);
    if let Some((token, reason)) = reboot_refusal(view["state"].as_str().unwrap_or("notConnected")) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    crate::guided::dispatch(backend, Some(json!({ "action": "reboot" })), crate::guided::active_id(backend), path, "[]")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        vehicle: Value,
        lost: bool,
        flying_to: i64,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            self.get_fields(path, "")
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" => self.vehicle.to_string(),
                "vehicle.radioStatus" => self.vehicle.get("radioStatus").cloned().unwrap_or_else(|| json!({ "kind": "null" })).to_string(),
                "vehicle.supports" => self.vehicle.get("supports").cloned().unwrap_or_else(|| json!({ "kind": "null" })).to_string(),
                "planFly.missionController" => json!({ "kind": "object", "currentMissionIndex": self.flying_to }).to_string(),
                "vehicle.vehicleLinkManager" => json!({ "kind": "object", "communicationLost": self.lost, "communicationLostEnabled": true }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _path: &str, _value: &str) -> String { String::new() }
        fn invoke(&self, _path: &str, _args: &str) -> String { String::new() }
        fn watch(&self, _paths: &[String]) {}
    }

    fn read(vehicle: Value, lost: bool) -> Value {
        fly_state_view(&Fake { vehicle, lost, flying_to: -1 }, &[])
    }

    #[test]
    fn a_disarmed_vehicle_reads_ready_or_not_in_main_status_indicator_order() {
        assert!(ready_to_fly(Some(true), Some(false), false, || false), "the arming check report wins when the vehicle supports it");
        assert!(!ready_to_fly(None, Some(false), true, || true), "then readyToFly when it is available");
        assert!(ready_to_fly(None, None, true, || true), "then healthy sensors and a finished setup");
        assert!(!ready_to_fly(None, None, true, || false));
        assert_eq!((state_line(State::Disarmed, true, true), state_line(State::Disarmed, false, true), state_line(State::Armed, false, false)), ("Ready to Fly", "Not Ready", "Armed"));
        assert_eq!(state_line(State::Disarmed, true, false), "Not Fully Ready");
    }

    #[test]
    fn summary_detail_reads_like_main_status_indicator() {
        let sensors = |list: &[(&str, bool, bool)]| list.iter().map(|(n, e, h)| (n.to_string(), *e, *h)).collect::<Vec<_>>();
        assert_eq!(summary_detail(true, true, 3, &[]), "Motors are armed and the vehicle is in the air.");
        assert_eq!(summary_detail(true, false, 0, &[]), "Motors are armed. Keep clear of the propellers.");
        assert_eq!(summary_detail(false, false, 2, &sensors(&[("GPS", true, false)])), "2 check(s) need attention before arming.");
        assert_eq!(summary_detail(false, false, 0, &sensors(&[("GPS", true, false), ("Gyro", true, false), ("Mag", false, false)])), "GPS, Gyro not working. Position modes and Return to Launch may not work.");
        assert_eq!(summary_detail(false, false, 0, &sensors(&[("GPS", true, true), ("Mag", false, false)])), "Mag turned off. Everything else reports normal.");
        assert_eq!(summary_detail(false, false, 0, &sensors(&[("GPS", true, true)])), "All checks passed.");
    }

    #[test]
    fn status_fault_ports_vehicle_status_summary() {
        assert!(status_fault(Some((false, false)), &[]), "with health checks a vehicle that cannot arm is a fault");
        assert!(!status_fault(Some((true, true)), &[(true, false)]), "warnings are only a caution, and the report replaces the sensor bits");
        assert!(status_fault(None, &[(true, false)]), "an enabled sensor that is unhealthy is a fault");
        assert!(!status_fault(None, &[(false, false)]), "a disabled sensor is a caution, not a fault");
    }

    #[test]
    fn status_nominal_ports_vehicle_status_summary() {
        assert!(status_nominal(Some((true, false)), &[(true, false)]), "the arming report replaces the sensor bits");
        assert!(!status_nominal(Some((true, true)), &[]), "warnings are a caution");
        assert!(!status_nominal(Some((false, false)), &[]));
        assert!(!status_nominal(None, &[(true, true), (false, false)]), "a disabled sensor is a caution");
        assert!(!status_nominal(None, &[(true, false)]));
        assert!(status_nominal(None, &[(true, true)]));
    }

    #[test]
    fn a_transmitter_reporting_nothing_is_not_a_transmitter_reporting_no_signal() {
        let at = |rssi: i64| {
            let mut vehicle = aloft(true, true, false);
            vehicle["facts"] = json!([{ "kind": "fact", "name": "rcRSSI", "property": "rcRSSI", "value": rssi }]);
            vehicle["supports"] = json!({ "kind": "object", "radio": true });
            fly_state_view(&Fake { vehicle, lost: false, flying_to: -1 }, &[])
        };
        assert_eq!(at(72)["rcSignal"], 72);
        assert_eq!(at(72)["rcSupported"], true, "radio support is vehicle.supports.radio; the view read vehicle.supportsRadio, which Vehicle has never had as a property, so rcSupported was false for every vehicle in the running app");
        assert_eq!(at(72)["rcSignalText"], "72%");
        assert_eq!(at(0)["rcSignalText"], "No signal", "a transmitter that is switched off is not a transmitter at zero per cent - an operator reading 0% concludes the link is alive and terrible rather than absent");
        assert_eq!(at(0)["rcSignal"], 0, "zero is a READING and the worst one - the transmitter is gone. QGC's own indicator hides at zero, so a total RC loss looks exactly like an aircraft with no transmitter fitted");
        assert_eq!(at(255)["rcSignal"], Value::Null, "255 is QGC's sentinel for a vehicle that has not reported a strength at all, which is the one case there is nothing to draw");
        assert_eq!(at(255)["rcSignalText"], Value::Null);
    }

    #[test]
    fn a_radio_that_has_never_reported_is_not_a_radio_at_zero_dbm() {
        let radio = |local: i64| {
            let mut vehicle = aloft(true, true, false);
            let fact = |property: &str, value: i64| json!({ "kind": "fact", "name": property, "property": property, "value": value });
            vehicle["radioStatus"] = json!({ "kind": "object", "class": "RadioStatusFactGroup", "facts": [fact("lrssi", local), fact("rrssi", -42), fact("rxErrors", 3), fact("fixed", 2), fact("txBuffer", 95), fact("lNoise", 12), fact("rNoise", 14)], "children": [] });
            fly_state_view(&Fake { vehicle, lost: false, flying_to: -1 }, &[])["telemetry"].clone()
        };

        assert_eq!(
            radio(0),
            Value::Null,
            "every SiK field is zero until the first RADIO_STATUS arrives, and TelemetryRSSIIndicator.qml:30 hides the whole indicator on that - serving the numbers anyway lets a head draw a healthy -0 dBm link for a radio that has never spoken"
        );

        let reporting = radio(-91);
        assert_eq!(
            (reporting["localRssiDbm"].clone(), reporting["remoteRssiDbm"].clone(), reporting["receiveErrors"].clone()),
            (json!(-91), json!(-42), json!(3))
        );
        assert_eq!((reporting["localNoise"].clone(), reporting["remoteNoise"].clone()), (json!(12), json!(14)));
        assert_eq!((reporting["errorsFixed"].clone(), reporting["txBuffer"].clone()), (json!(2), json!(95)), "TelemetryRSSIIndicator lists Errors Fixed and TX Buffer");
    }

    #[test]
    fn an_rc_override_is_unknown_without_a_vehicle_rather_than_absent() {
        let mut overridden = aloft(true, true, false);
        overridden["rcChannelOverrideActive"] = json!(true);
        assert_eq!(read(overridden, false)["rcOverride"], json!(true));

        assert_eq!(read(aloft(true, true, false), false)["rcOverride"], json!(false), "a connected vehicle with an empty override list is genuinely not overridden");
        assert_eq!(
            read(json!({ "kind": "null" }), false)["rcOverride"],
            Value::Null,
            "rcChannelOverrideActive is !_rcChannelOverrides.isEmpty(), so with no vehicle there is no list to be empty - false would tell an operator manual control is definitely not being overridden, which is a safety claim nothing has made"
        );
    }

    #[test]
    fn the_item_the_aircraft_is_flying_to_is_a_sequence_and_is_withheld_on_the_ground() {
        let airborne = |flying_to: i64| fly_state_view(&Fake { vehicle: aloft(true, true, false), lost: false, flying_to }, &[])["flyingToSequence"].clone();
        assert_eq!(airborne(3), json!(3), "MissionController::currentMissionIndex is a sequence number, already stepped past home for firmwares that do not send it, so a head matches it against an item's sequence rather than its index - the two differ as soon as a complex item is in the plan");
        assert_eq!(airborne(-1), Value::Null, "the fly controller answers -1 in the plan view and before the vehicle names an item, and a head drawing -1 would highlight nothing or the row before the first");
        assert_eq!(airborne(0), json!(0), "sequence 0 is the launch row and a real answer");
    }

    fn aloft(armed: bool, flying: bool, landing: bool) -> Value {
        json!({ "kind": "object", "armed": armed, "flying": flying, "landing": landing, "flightMode": "Hold" })
    }

    #[test]
    fn lost_contact_outranks_every_flight_state() {
        let view = read(aloft(true, true, false), true);
        assert_eq!((view["state"].as_str(), view["stateText"].as_str()), (Some("contactLost"), Some("Comms Lost")), "a vehicle that stopped answering is not known to still be flying");
        assert_eq!(view["staleNotice"], STALE_NOTICE);
        assert_eq!((view["armed"].as_bool(), view["flying"].as_bool()), (Some(true), Some(true)), "the last known state is still served, it just no longer names the line");
    }

    #[test]
    fn a_disarmed_vehicle_never_reads_as_flying() {
        let view = read(aloft(false, true, false), false);
        assert_eq!(view["state"], "disarmed", "Vehicle::_flying is set from EXTENDED_SYS_STATE and never cleared on disarm, so armed is what decides the line");
        assert_eq!(view["flying"], true, "the last flying flag is still reported for a head that wants it");
    }

    #[test]
    fn an_armed_vehicle_reads_as_what_it_is_doing() {
        let line = |armed: bool, flying: bool, landing: bool| read(aloft(armed, flying, landing), false)["state"].as_str().unwrap().to_string();
        assert_eq!(line(true, true, false), "flying");
        assert_eq!(line(true, true, true), "flying", "MainStatusIndicator asks flying before landing");
        assert_eq!(line(true, false, true), "landing", "and names Landing only once the vehicle is armed but no longer flying");
        assert_eq!(line(true, false, false), "armed");
        assert_eq!(read(aloft(true, true, false), false)["staleNotice"], "", "a vehicle in contact carries no stale notice");
    }

    #[test]
    fn no_vehicle_is_not_a_lost_contact() {
        let view = read(json!({ "kind": "null" }), true);
        assert_eq!((view["state"].as_str(), view["stateText"].as_str()), (Some("notConnected"), Some("Not connected")), "never having heard a vehicle is not the same as losing one");
        assert_eq!(view["contactLost"], Value::Null, "with no vehicle there is no link to have lost, and false would be a claim about one");
        assert_eq!((view["armed"].as_bool(), view["flying"].as_bool()), (Some(false), Some(false)));
        assert_eq!(view["staleNotice"], "");
        assert_eq!(view["mode"], "");
    }

    #[test]
    fn the_listed_states_are_exactly_the_states_reachable() {
        let bit = |mask: u8, index: u8| mask & (1 << index) != 0;
        let reachable: std::collections::BTreeSet<State> = (0u8..32).map(|mask| state_of(bit(mask, 0), bit(mask, 1), bit(mask, 2), bit(mask, 3), bit(mask, 4))).collect();
        assert_eq!(reachable, STATES.iter().copied().collect(), "every state a head switches on is one this view can answer, and there are no others");
    }

    #[test]
    fn a_link_nobody_is_watching_is_not_reported_as_healthy() {
        struct Unwatched;
        impl Backend for Unwatched {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicle" => json!({ "kind": "object", "armed": false, "flying": false, "landing": false, "flightMode": "Hold" }).to_string(),
                    "vehicle.vehicleLinkManager" => json!({ "kind": "object", "communicationLost": false, "communicationLostEnabled": false }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let unwatched = fly_state_view(&Unwatched, &[]);
        assert_eq!(unwatched["contactLost"], Value::Null, "_commLostCheck returns early when the watch is disabled, so communicationLost never updates and false means nobody is looking - this view served it raw and called an unmonitored link healthy, in five places on one head");
        assert_eq!(unwatched["state"].as_str(), Some("disarmed"), "and unknown is not evidence of a loss, so the state it drives is the one the vehicle actually reports");
        assert_eq!(unwatched["staleNotice"], "", "nor does an unknown link earn a stale notice");
    }

    #[test]
    fn a_reboot_goes_only_to_a_vehicle_known_to_be_disarmed_on_the_ground() {
        assert_eq!(reboot_refusal("disarmed"), None);
        STATES.iter().filter(|s| **s != State::Disarmed).for_each(|state| {
            assert!(reboot_refusal(state.token()).is_some(), "rebootVehicle sends MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN with no check, and {} is a state where that drops an aircraft or reaches nobody", state.token());
        });
        assert_eq!(reboot_refusal("contactLost").map(|(t, _)| t), Some("contactLost"), "a vehicle that stopped answering is not known to be on the ground");
        assert_eq!(reboot_refusal("landing").map(|(t, _)| t), Some("flying"));
        assert_eq!(reboot_refusal("somethingNew").map(|(t, _)| t), Some("armed"), "a state this list has not met is refused, not waved through");
    }
}
