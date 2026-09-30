use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.gcsMain",
    "vehicle.gcsControlStatusFlags_SystemManager",
    "vehicle.gcsControlStatusFlags_TakeoverAllowed",
    "vehicle.firstControlStatusReceived",
    "vehicle.sendControlRequestAllowed",
    "settings.mavlinkSettings.gcsMavlinkSystemID",
];

pub const TAKEOVER_TIMEOUT_MSECS: i64 = 10_000;
pub const DEFAULT_REQUEST_TIMEOUT_SECS: i64 = 10;
pub const REQUEST_OPERATOR_CONTROL: u16 = 32100;
pub const CONTROL_STATUS: u32 = 512;
const FLAG_SYSTEM_MANAGER: u8 = 1;
const FLAG_TAKEOVER_ALLOWED: u8 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct ControlState {
    pub gcs_main: u8,
    pub flags: u8,
    pub first: bool,
    pub request_allowed: bool,
    allowed_again_ms: Option<u64>,
    incoming: Option<Incoming>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Incoming {
    pub from: u8,
    pub allow_takeover: bool,
    pub timeout_ms: u64,
    until_ms: u64,
}

impl Default for ControlState {
    fn default() -> Self {
        ControlState { gcs_main: 0, flags: 0, first: false, request_allowed: true, allowed_again_ms: None, incoming: None }
    }
}

impl ControlState {
    pub fn system_manager(&self) -> bool {
        self.flags & FLAG_SYSTEM_MANAGER != 0
    }

    pub fn takeover_allowed(&self) -> bool {
        self.flags & FLAG_TAKEOVER_ALLOWED != 0
    }

    pub fn on_status(&mut self, flags: u8, gcs_main: u8) {
        (self.flags, self.gcs_main, self.first) = (flags, gcs_main, true);
        if !self.request_allowed && self.takeover_allowed() {
            (self.request_allowed, self.allowed_again_ms) = (true, None);
        }
    }

    pub fn requested(&mut self, timeout_secs: i64, now_ms: u64) {
        if timeout_secs > 0 {
            (self.request_allowed, self.allowed_again_ms) = (false, Some(now_ms + timeout_secs as u64 * 1000));
        }
    }

    pub fn on_request(&mut self, from: u8, allow_takeover: bool, timeout_secs: f32, default_secs: i64, now_ms: u64) {
        if !self.first {
            return;
        }
        let secs = if timeout_secs > 0.0 { timeout_secs as u64 } else { u64::try_from(default_secs).unwrap_or(0) };
        let timeout_ms = secs * 1000;
        self.incoming = Some(Incoming { from, allow_takeover, timeout_ms, until_ms: now_ms + timeout_ms });
    }

    pub fn answered(&mut self) {
        self.incoming = None;
    }

    pub fn incoming_json(&self, now_ms: u64) -> Value {
        self.incoming.filter(|i| now_ms < i.until_ms).map_or(Value::Null, |i| json!({ "systemId": i.from, "allowTakeover": i.allow_takeover, "timeoutMs": i.timeout_ms, "remainingMs": i.until_ms - now_ms }))
    }

    pub fn tick(&mut self, now_ms: u64) {
        if self.allowed_again_ms.is_some_and(|at| now_ms >= at) {
            (self.request_allowed, self.allowed_again_ms) = (true, None);
        }
        if self.incoming.is_some_and(|i| now_ms >= i.until_ms) {
            self.incoming = None;
        }
    }

    pub fn fields(&self) -> Value {
        json!({
            "gcsMain": self.gcs_main,
            "gcsControlStatusFlags_SystemManager": self.system_manager(),
            "gcsControlStatusFlags_TakeoverAllowed": self.takeover_allowed(),
            "firstControlStatusReceived": self.first,
            "sendControlRequestAllowed": self.request_allowed,
            "operatorControlTakeoverTimeoutMsecs": TAKEOVER_TIMEOUT_MSECS,
        })
    }
}

pub fn safe_timeout(asked: i64, bounds: (i64, i64), default: i64) -> i64 {
    if (bounds.0..=bounds.1).contains(&asked) { asked } else { default }
}

const FIELDS: &str = "gcsMain,gcsControlStatusFlags_SystemManager,gcsControlStatusFlags_TakeoverAllowed,firstControlStatusReceived,sendControlRequestAllowed,operatorControlTakeoverTimeoutMsecs";

fn integer(read: &Value, key: &str) -> Option<i64> {
    read.get(key).and_then(Value::as_i64)
}

fn incoming_request() -> Value {
    crate::vehiclefacade::switched_on()
        .then(|| crate::hub::lock().active().map(|v| v.control.incoming_json(crate::hub::now_ms())))
        .flatten()
        .unwrap_or(Value::Null)
}

pub fn operator_control_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", FIELDS));
    if vehicle.get("kind").and_then(Value::as_str) != Some("object") {
        return json!({
            "kind": "object",
            "class": "OperatorControl",
            "available": false,
            "known": false,
            "inControl": Value::Null,
            "holderSystemId": Value::Null,
            "takeoverAllowed": Value::Null,
            "systemManager": Value::Null,
            "reason": "No vehicle is connected.",
        });
    }
    let known = flag(&vehicle, "firstControlStatusReceived");
    let holder = integer(&vehicle, "gcsMain");
    let ours = crate::read::value_number(&backend.get("settings.mavlinkSettings.gcsMavlinkSystemID.rawValue")).map(|id| id as i64);
    let answered = |yes: bool| known.then_some(yes);
    json!({
        "kind": "object",
        "class": "OperatorControl",
        "available": true,
        "known": known,
        "inControl": known.then(|| holder.zip(ours).map(|(holder, ours)| holder == ours)).flatten(),
        "holderSystemId": known.then_some(holder).flatten(),
        "takeoverAllowed": answered(flag(&vehicle, "gcsControlStatusFlags_TakeoverAllowed")),
        "systemManager": answered(flag(&vehicle, "gcsControlStatusFlags_SystemManager")),
        "requestAllowed": flag(&vehicle, "sendControlRequestAllowed"),
        "incomingRequest": incoming_request(),
        "takeoverTimeoutMs": integer(&vehicle, "operatorControlTakeoverTimeoutMsecs"),
        "reason": match (known, holder, ours) {
            (false, _, _) => "This vehicle has not said who is flying it.",
            (true, Some(holder), Some(ours)) if holder == ours => "",
            (true, Some(_), Some(_)) => "Another ground station is flying this vehicle.",
            (true, None, _) => "This vehicle reported its control status without saying which station holds it.",
            (true, Some(_), None) => "This ground station could not read its own MAVLink system id, so it cannot tell whether the station flying this vehicle is itself.",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_waits_out_its_timeout_unless_the_holder_allows_takeover_first() {
        let mut control = ControlState::default();
        assert_eq!((control.first, control.request_allowed), (false, true));
        control.requested(10, 1_000);
        assert!(!control.request_allowed);
        control.tick(10_999);
        assert!(!control.request_allowed);
        control.on_status(FLAG_TAKEOVER_ALLOWED, 7);
        assert!(control.request_allowed, "a holder that allows takeover answers the request early");
        assert_eq!((control.gcs_main, control.first, control.system_manager()), (7, true, false));
        control.requested(5, 20_000);
        control.on_status(FLAG_SYSTEM_MANAGER, 7);
        assert!(!control.request_allowed);
        control.tick(25_000);
        assert!(control.request_allowed);
        control.requested(0, 30_000);
        assert!(control.request_allowed, "a no-wait request does not block the next one");
    }

    #[test]
    fn a_timeout_outside_the_setting_limits_is_sent_as_the_default() {
        assert_eq!((safe_timeout(10, (3, 60), 10), safe_timeout(0, (3, 60), 10), safe_timeout(90, (3, 60), 10)), (10, 10, 10));
        assert_eq!(safe_timeout(30, (3, 60), 10), 30);
    }

    #[test]
    fn the_request_goes_out_as_a_command_the_decoded_dialect_cannot_name() {
        let bytes = crate::mavout::encode(3, &crate::mavout::Outbound::RawCommandLong { target: (1, 1), command: REQUEST_OPERATOR_CONTROL, params: [0.0, 1.0, 1.0, 10.0, 0.0, 0.0, 0.0] }).unwrap();
        let (header, msgid) = crate::mavinspect::undecoded(&bytes, true).expect("a COMMAND_LONG with a good checksum");
        assert_eq!((msgid, header.sequence), (76, 3));
        let fields = crate::mavinspect::fields(76, &crate::mavinspect::payload(&bytes, true));
        let value = |name: &str| fields.iter().find(|f| f["name"] == name).and_then(|f| f["value"].as_str()).unwrap_or("").to_string();
        assert_eq!((value("command"), value("param2"), value("param3"), value("param4"), value("target_system")), ("32100".into(), "1".into(), "1".into(), "10".into(), "1".into()));
    }

    struct Station {
        known: bool,
        holder: Option<i64>,
        takeover: bool,
        own_id: Option<i64>,
    }

    impl Backend for Station {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.mavlinkSettings.gcsMavlinkSystemID.rawValue" => match self.own_id {
                    Some(id) => json!({ "kind": "value", "value": id }),
                    None => json!({ "kind": "null" }),
                },
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicle" => json!({
                    "kind": "object",
                    "gcsMain": self.holder,
                    "gcsControlStatusFlags_SystemManager": true,
                    "gcsControlStatusFlags_TakeoverAllowed": self.takeover,
                    "firstControlStatusReceived": self.known,
                    "sendControlRequestAllowed": true,
                    "operatorControlTakeoverTimeoutMsecs": 10000,
                }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_comparison_the_core_could_not_make_is_not_a_verdict_that_someone_else_is_flying() {
        let unreadable = operator_control_view(&Station { known: true, holder: Some(250), takeover: false, own_id: None }, &[]);
        assert_eq!(unreadable["inControl"], Value::Null);
        assert_eq!(
            unreadable["reason"],
            "This ground station could not read its own MAVLink system id, so it cannot tell whether the station flying this vehicle is itself.",
            "the arm used to be a catch-all that swallowed None into \"Another ground station is flying this vehicle\", so a station that could not read its own id was told someone else held control - a verdict nothing had established, and one a head greys out its controls on"
        );

        let anonymous = operator_control_view(&Station { known: true, holder: None, takeover: false, own_id: Some(255) }, &[]);
        assert_eq!(
            anonymous["reason"],
            "This vehicle reported its control status without saying which station holds it.",
            "a missing holder and an unreadable own id are two different silences and neither is the other station"
        );

        let theirs = operator_control_view(&Station { known: true, holder: Some(42), takeover: false, own_id: Some(255) }, &[]);
        assert_eq!(theirs["reason"], "Another ground station is flying this vehicle.", "the real case still reads as it did");
    }

    #[test]
    fn the_countdown_is_not_served_because_no_watching_head_could_receive_it() {
        let view = operator_control_view(&Station { known: true, holder: Some(250), takeover: false, own_id: Some(250) }, &[]);

        assert!(
            !view.as_object().unwrap().contains_key("remainingMs"),
            "Vehicle::requestOperatorControlStartTimer sets _sendControlRequestAllowed and emits BEFORE starting _timerRequestOperatorControl, and that emit is the only dep that fires because requestOperatorControlRemainingMsecs is CONSTANT. So the view renders at the one instant remainingTime() returns -1 for an inactive timer, and nothing re-fires for the next ten seconds. A poll through /bridge/get counts down cleanly, which is why this looked like it worked: it serves a value no watching head can ever receive, and a field that is null exactly when a head would use it reads as available. A countdown belongs to the head, ticking off takeoverTimeoutMs"
        );
        assert!(view["takeoverTimeoutMs"].is_i64(), "the head needs the duration to tick off, so removing the countdown must not remove what a countdown is built from");
    }

    #[test]
    fn a_vehicle_that_has_not_said_who_is_flying_it_is_not_a_vehicle_flown_by_someone_else() {
        let silent = operator_control_view(&Station { known: false, holder: Some(0), takeover: false, own_id: Some(250) }, &[]);
        assert_eq!(silent["inControl"], Value::Null, "before any CONTROL_STATUS arrives gcsMain is 0 and both flags are false, which is byte-identical to another station holding it with takeover denied - firstControlStatusReceived is the only thing that tells them apart");
        assert_eq!(silent["takeoverAllowed"], Value::Null);
        assert_eq!(silent["holderSystemId"], Value::Null);
        assert!(silent["reason"].as_str().unwrap().contains("not said"));

        let ours = operator_control_view(&Station { known: true, holder: Some(250), takeover: false, own_id: Some(250) }, &[]);
        assert_eq!(ours["inControl"], true, "being in control is gcsMain matching THIS ground station's own MAVLink system id, a GCS-side setting - comparing against the vehicle's id would be wrong on every non-default station");
        assert_eq!(ours["reason"], "");

        let theirs = operator_control_view(&Station { known: true, holder: Some(42), takeover: true, own_id: Some(250) }, &[]);
        assert_eq!(theirs["inControl"], false);
        assert_eq!(theirs["holderSystemId"], 42);
        assert_eq!(theirs["takeoverAllowed"], true);
        assert!(theirs["reason"].as_str().unwrap().contains("Another ground station"));
    }
}
